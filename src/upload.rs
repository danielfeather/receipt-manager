use std::{fmt::Write as _, path::Path, sync::Arc};

use aws_config::{BehaviorVersion, meta::region::RegionProviderChain, sts::AssumeRoleProvider};
use aws_sdk_s3::{Client, config::Region};
use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use minijinja::context;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{AppState, assets};

pub async fn upload(
    session: Session,
    mut multipart: Multipart,
) -> axum::response::Result<Redirect> {
    let mut name: Option<String> = None;
    let mut data: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await? {
        let field_name = field.name().unwrap().to_string();

        if field_name == "name" {
            name = Some(field.text().await?);
            continue;
        }

        if field_name == "file" {
            data = Some(field.bytes().await?);
        }
    }

    let role_arn = std::env::var("IAM_ROLE")
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Service is misconfigured"))?;

    let region_provider = RegionProviderChain::first_try(Region::new("us-west-2"));

    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let provider = AssumeRoleProvider::builder(role_arn)
        .configure(&shared_config)
        .session_name("testAR")
        .build()
        .await;

    let local_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new("eu-west-2"))
        .credentials_provider(provider)
        .load()
        .await;

    let client = Client::new(&local_config);

    let bucket_name = std::env::var("S3_BUCKET")
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Service is misconfigured"))?;

    let mut filename = Uuid::now_v7().to_string();

    if let Some(name) = name {
        if name.len() > 255 {
            return Err((StatusCode::BAD_REQUEST, "Bad request").into());
        }

        let _ = write!(&mut filename, "-{name}");
    }

    let Some(body) = data else {
        return Err((StatusCode::BAD_REQUEST, "No receipt provided").into());
    };

    client
        .put_object()
        .bucket(bucket_name)
        .key(&filename)
        .body(body.into())
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload receipt, got S3 error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload receipt",
            )
        })?;

    match session.insert("filename", filename).await {
        Err(e) => {
            tracing::error!("Failed to update session: {e}")
        }
        _ => {}
    }

    Ok(Redirect::to("/success"))
}

pub async fn success(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> axum::response::Result<Response> {
    let scripts = assets::resolve_scripts(
        Path::new("client/main.ts"),
        #[cfg(not(feature = "debug"))]
        Some(&state.manifest),
        #[cfg(feature = "debug")]
        None,
    );

    let css = assets::resolve_css(
        Path::new("client/main.ts"),
        #[cfg(not(feature = "debug"))]
        Some(&state.manifest),
        #[cfg(feature = "debug")]
        None,
    );

    let filename = session
        .remove::<String>("filename")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(filename) = filename else {
        return Ok(Redirect::to("/").into_response());
    };

    let env = state.loader.acquire_env().unwrap();

    let templ = env.get_template("success.html").unwrap();

    let res = templ
        .render(context! { css => css, scripts => scripts, filename => filename })
        .unwrap();

    Ok(Html(res).into_response())
}
