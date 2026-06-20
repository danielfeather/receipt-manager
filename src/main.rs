#[cfg(not(feature = "debug"))]
use assets::Manifest;
use aws_config::{
    BehaviorVersion, Region, meta::region::RegionProviderChain, sts::AssumeRoleProvider,
};
use aws_sdk_s3::Client;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use minijinja::{Environment, context, path_loader};
use minijinja_autoreload::AutoReloader;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use std::{path::Path, sync::Arc};
use tower_http::services::ServeDir;
use tower_sessions::{
    Expiry, MemoryStore, SessionManagerLayer,
    cookie::{Key, time::Duration},
};
use tracing::{debug, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod assets;
mod routes;

const TEMPLATE_PATH: &str = "views";

struct AppState {
    loader: AutoReloader,
    #[cfg(not(feature = "debug"))]
    manifest: Manifest,
    pool: Pool<Postgres>,
}

const PAGES: &'static [(&str, &str)] = &[
    ("Manage receipts", "/receipts"),
    ("Upload receipts", "/upload"),
];

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    #[allow(unused)]
    let maybe_manifest = assets::load_manifest();

    debug!("attempting to read DATABASE_URL");
    let database_url = std::env::var("DATABASE_URL")
        .expect("Unable to connect to DB, DATABASE_URL is not present in env");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Unable to connect to DB");

    let key = Key::generate();

    let reloader = AutoReloader::new(|notifier| {
        let mut env = Environment::new();
        env.set_loader(path_loader(TEMPLATE_PATH));
        notifier.watch_path(TEMPLATE_PATH, true);
        Ok(env)
    });

    let session_store = MemoryStore::default();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_expiry(Expiry::OnInactivity(Duration::hours(12)))
        .with_always_save(true)
        .with_signed(key);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(async || Redirect::to("/receipts").into_response()))
        .nest("/receipts", routes::receipts::router())
        .nest("/upload", routes::upload::router())
        .layer(session_layer)
        .fallback_service(ServeDir::new("public"))
        .with_state(Arc::new(AppState {
            loader: reloader,
            #[cfg(not(feature = "debug"))]
            manifest: maybe_manifest.expect("Unable to find asset manifest"),
            pool,
        }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!("Listening on 0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
