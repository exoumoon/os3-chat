use std::fs::File;
use std::io::Write;

use axum::debug_handler;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use tracing::instrument;
use uuid::Uuid;

use crate::auth::Session;
use crate::state::SharedState;

#[instrument(skip_all, err(Debug))]
#[debug_handler]
pub async fn upload_handler(
    State(app_state): State<SharedState>,
    Session(account): Session,
    mut multipart: Multipart,
) -> Result<Redirect, (StatusCode, String)> {
    let mut file_data = None;
    let mut room_id = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or_default().to_string();

        if name == "file" {
            let filename = field
                .file_name()
                .unwrap_or("unnamed_upload.bin")
                .to_string();
            let data = field.bytes().await.unwrap();
            let uuid = Uuid::new_v4().to_string();

            let store_path = format!("database/file_uploads/{uuid}_{filename}");
            let mut store_file = File::create(&store_path)
                .inspect_err(|error| tracing::error!(?error, "Failed to handle file upload"))
                .map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to create file: {error}"),
                    )
                })?;

            store_file.write_all(&data).unwrap();
            file_data = Some((filename, store_path));
        } else if name == "room_id" {
            room_id = Some(field.text().await.unwrap().parse::<i64>().unwrap());
        }
    }

    let (original_filename, store_path) =
        file_data.ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing file".to_string()))?;
    let room_id =
        room_id.ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing room_id".to_string()))?;

    let query = sqlx::query!(
        r#"
            INSERT INTO file_uploads (uploader_account_id, room_id, original_filename, store_path)
            VALUES (?, ?, ?, ?)
        "#,
        account.id,
        room_id,
        original_filename,
        store_path
    );

    query
        .execute(&app_state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok(Redirect::to(format!("/chat/{room_id}").as_str()))
}
