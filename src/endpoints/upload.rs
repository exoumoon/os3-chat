use std::path::PathBuf;

use axum::debug_handler;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use tracing::instrument;

use crate::auth::Session;
use crate::state::SharedState;

#[instrument(skip_all, err(Debug))]
#[debug_handler]
pub async fn upload_handler(
    State(state): State<SharedState>,
    Session(account): Session,
    mut multipart: Multipart,
) -> Result<Redirect, StatusCode> {
    let mut file_data = None;
    let mut room_id = None;

    while let Some(f) = multipart.next_field().await.unwrap() {
        match f.name() {
            Some("file") => {
                const DEFAULT_FILENAME: &str = "unnamed_upload.bin";
                let filename = PathBuf::from(f.file_name().unwrap_or(DEFAULT_FILENAME));
                let data = f
                    .bytes()
                    .await
                    .inspect_err(|error| tracing::error!(?error, "Failed to handle file upload"))
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                file_data = Some((filename, data));
            }

            Some("room_id") => {
                room_id = f
                    .text()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .parse::<i64>()
                    .map(Some)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
            }

            _ => { /* Unknown field */ }
        }
    }

    let (original_filename, data) = file_data.ok_or(StatusCode::BAD_REQUEST)?;
    let room_id = room_id.ok_or(StatusCode::BAD_REQUEST)?;

    let room = state
        .repository
        .rooms
        .find_by_id(room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let _upload = room
        .upload(
            &state.db_pool,
            account.id,
            room_id,
            &original_filename,
            &data,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/chat/{room_id}")))
}
