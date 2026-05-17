use std::{path::Path, sync::Arc};

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use minijinja::context;
use tower_sessions::Session;

use crate::{AppState, assets, flash::Flash};

pub async fn upload(
    State(state): State<Arc<AppState>>,
    session: Session,
    mut multipart: Multipart,
) -> axum::response::Result<Redirect> {
    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!("Length of `{}` is {} bytes", name, data.len());
    }

    let _ = session.insert("filename", uuid::Uuid::now_v7()).await;

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
        .remove::<uuid::Uuid>("filename")
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
