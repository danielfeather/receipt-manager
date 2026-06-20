use std::{path::Path, sync::Arc, time::Duration};

use aws_config::{
    BehaviorVersion, Region, meta::region::RegionProviderChain, sts::AssumeRoleProvider,
};
use aws_sdk_s3::{Client, presigning::PresigningConfig};
use axum::{
    Router,
    extract::{Path as PathParam, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::get,
};
use minijinja::context;
use tracing::{debug, error};

use crate::{AppState, PAGES, assets};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/new", get(new_form))
        .route("/{id}", get(get_receipt))
}

async fn list(State(state): State<Arc<AppState>>) -> axum::response::Result<Html<String>> {
    let scripts = assets::resolve_scripts(
        Path::new("client/main.ts"),
        #[cfg(not(feature = "debug"))]
        Some(&state.manifest),
        #[cfg(feature = "debug")]
        None,
    );
    debug!("Loaded scripts");

    let css = assets::resolve_css(
        Path::new("client/main.ts"),
        #[cfg(not(feature = "debug"))]
        Some(&state.manifest),
        #[cfg(feature = "debug")]
        None,
    );
    debug!("Loaded css");

    debug!("attempting to read IAM_ROLE");
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

    debug!("attempting to list objects");
    let response = client
        .list_objects_v2()
        .bucket(bucket_name)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to list receipts: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list receipts")
        })?;

    let objects = response.contents();

    let receipts: Vec<&str> = objects.iter().filter_map(|ob| ob.key()).collect();

    let env = state.loader.acquire_env().unwrap();

    let templ = env.get_template("receipts.njk").unwrap();

    let res = match templ
        .render(context! { css => css, scripts => scripts, receipts => receipts, pages => PAGES, active => 0 }) {
            Ok(templ) => templ,
            Err(e) => {
                let string = format!("{e}");
                tracing::error!(string);
                return Ok(Html(string))
            },
        };

    Ok(Html(res))
}

async fn get_receipt(PathParam(id): PathParam<String>) -> axum::response::Result<Redirect> {
    let bucket_name = std::env::var("S3_BUCKET")
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Service is misconfigured"))?;

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

    let url = client
        .get_object()
        .bucket(bucket_name)
        .key(id)
        .presigned(
            PresigningConfig::builder()
                .expires_in(Duration::from_secs(60 * 5))
                .build()
                .expect("Invalid presigned config"),
        )
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to get receipt object",
            )
        })?;

    Ok(Redirect::to(url.uri()))
}

async fn new_form(State(state): State<Arc<AppState>>) -> axum::response::Result<Html<String>> {
    let scripts = assets::resolve_scripts(
        Path::new("client/main.ts"),
        #[cfg(not(feature = "debug"))]
        Some(&state.manifest),
        #[cfg(feature = "debug")]
        None,
    );
    debug!("Loaded scripts");

    let css = assets::resolve_css(
        Path::new("client/main.ts"),
        #[cfg(not(feature = "debug"))]
        Some(&state.manifest),
        #[cfg(feature = "debug")]
        None,
    );
    debug!("Loaded css");

    let env = state.loader.acquire_env().unwrap();

    let templ = env.get_template("receipt.njk").unwrap();

    let res = match templ
        .render(context! { css => css, scripts => scripts, pages => PAGES, active => 0 })
    {
        Ok(templ) => templ,
        Err(e) => {
            let string = format!("{e}");
            tracing::error!(string);
            return Ok(Html(string));
        }
    };

    Ok(Html(res))
}
