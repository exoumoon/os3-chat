use std::path::PathBuf;
use std::str::FromStr;

use axum::body::Body;
use axum::debug_handler;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::instrument;
use uuid::Uuid;

use crate::auth::Session;
use crate::state::SharedState;

#[instrument(skip_all, err(Debug))]
#[debug_handler]
pub async fn upload_handler(
    State(state): State<SharedState>,
    Session(uploader): Session,
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

    let upload = room
        .upload(&state.db_pool, &original_filename, &data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let uuid = Uuid::from_str(&upload.uuid).unwrap();
    let message = room
        .send_new_message_with_file(&state.db_pool, &uploader.username, None, uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let echoed_message = message
        .to_echoed_message(&state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _recv_count = state
        .broadcast_tx
        .send(echoed_message)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/chat/{room_id}")))
}

#[instrument(skip_all, err(Debug))]
#[debug_handler]
pub async fn download_handler(
    State(state): State<SharedState>,
    Path(uuid): Path<String>,
) -> Result<Response, StatusCode> {
    let uuid = Uuid::from_str(&uuid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let upload = state
        .repository
        .uploads
        .find(uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let store_path = upload
        .store_path()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match File::open(store_path).await {
        Err(error) => {
            tracing::error!(?error, "Failed to handle file download request");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }

        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            let filename = upload.filename.to_string_lossy();
            let value = format!("attachment; filename = \"{filename}\"",);
            let headers =
                HeaderMap::from_iter([(header::CONTENT_DISPOSITION, value.parse().unwrap())]);

            Ok((headers, body).into_response())
        }
    }
}
