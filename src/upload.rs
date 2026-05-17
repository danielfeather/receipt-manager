use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use tower_sessions::Session;

use crate::{AppState, flash::Flash};

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

    let flash = session
        .get::<Flash>("flash")
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Unable to read session"))?;

    let mut flash = match flash {
        Some(value) => value,
        None => Flash::default(),
    };

    flash.message(
        crate::flash::MessageKind::Info,
        "Uploaded successfully".to_string(),
    );

    let _ = session.insert("flash", flash).await;

    Ok(Redirect::to("/"))
}
