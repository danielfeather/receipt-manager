#[cfg(not(feature = "debug"))]
use assets::Manifest;
use axum::{
    Router,
    extract::State,
    response::Html,
    routing::{get, post},
};
use minijinja::{Environment, context, path_loader};
use minijinja_autoreload::AutoReloader;
use std::{path::Path, sync::Arc};
use tower_http::services::ServeDir;
use tower_sessions::{
    Expiry, Session, SessionManagerLayer,
    cookie::{Key, time::Duration},
};

use crate::session::FileSessionStorage;

mod assets;
mod flash;
mod session;
mod upload;

const TEMPLATE_PATH: &str = "views";

struct AppState {
    loader: AutoReloader,
    #[cfg(not(feature = "debug"))]
    manifest: Manifest,
}

#[tokio::main]
async fn main() {
    let maybe_manifest = assets::load_manifest();

    let key = Key::generate();

    let reloader = AutoReloader::new(|notifier| {
        let mut env = Environment::new();
        env.set_loader(path_loader(TEMPLATE_PATH));
        notifier.watch_path(TEMPLATE_PATH, true);
        Ok(env)
    });

    let session_store = FileSessionStorage::new();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(10)))
        .with_http_only(true)
        .with_always_save(true)
        .with_signed(key);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(home))
        .route("/upload", post(upload::upload))
        .layer(session_layer)
        .fallback_service(ServeDir::new("public"))
        .with_state(Arc::new(AppState {
            loader: reloader,
            #[cfg(not(feature = "debug"))]
            manifest: maybe_manifest.expect("Unable to find asset manifest"),
        }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn home(State(state): State<Arc<AppState>>, _session: Session) -> Html<String> {
    let env = state.loader.acquire_env().unwrap();

    let templ = env.get_template("home.html").unwrap();

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

    let res = templ
        .render(context! { css => css, scripts => scripts })
        .unwrap();

    Html(res)
}
