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
        .route("/success", get(upload::success))
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

async fn home(State(state): State<Arc<AppState>>) -> axum::response::Result<Html<String>> {
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

    let response = client
        .list_objects_v2()
        .bucket(bucket_name)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let objects = response.contents();

    let receipts: Vec<&str> = objects.iter().filter_map(|ob| ob.key()).collect();

    let env = state.loader.acquire_env().unwrap();

    let templ = env.get_template("home.html").unwrap();

    let res = templ
        .render(context! { css => css, scripts => scripts, receipts => receipts })
        .unwrap();

    Ok(Html(res))
}
