use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{Router, extract::State, response::Html, routing::get};
use minijinja::{Environment, context, path_loader};
use tower_http::services::ServeDir;
use tower_sessions::{
    Expiry, Session, SessionManagerLayer,
    cookie::{Key, time::Duration},
};

use crate::{assets::Manifest, session::FileSessionStorage};

mod assets;
mod session;

#[derive(Debug)]
struct AppState {
    minij: minijinja::Environment<'static>,
    manifest: Manifest,
}

#[tokio::main]
async fn main() {
    let key = Key::generate();

    let mut env = Environment::new();

    let Ok(manifest_bytes) = std::fs::read_to_string("public/.vite/manifest.json") else {
        return;
    };

    let Ok(manifest) = serde_json::from_str(&manifest_bytes) else {
        return;
    };

    env.set_loader(path_loader("views"));

    let session_store = FileSessionStorage::new();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(10)))
        .with_always_save(true);
    // .with_signed(key);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(home).layer(session_layer))
        .fallback_service(ServeDir::new("public"))
        .with_state(Arc::new(AppState {
            minij: env,
            manifest,
        }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn home(State(state): State<Arc<AppState>>, session: Session) -> Html<String> {
    session.save().await;

    let templ = state.minij.get_template("home.html").unwrap();

    let mut css: Vec<String> = Vec::new();
    let mut scripts: Vec<String> = Vec::new();

    #[cfg(feature = "debug")]
    {
        scripts.push("http://localhost:5173/@vite/client".to_string());
    }

    let chunk = state.manifest.get(Path::new("client/main.ts")).unwrap();

    scripts.push(chunk.file.clone());

    for file in &chunk.css {
        css.push(file.clone());
    }

    let res = templ
        .render(context! { css => css, scripts => scripts })
        .unwrap();

    Html(res)
}
