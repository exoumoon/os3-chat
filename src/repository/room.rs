use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use axum::body::Bytes;
use chrono::NaiveDateTime;
use sqlx::SqlitePool;
use tracing::instrument;
use uuid::Uuid;

use super::account::Account;
use super::message::Message;
use super::upload::Upload;

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct Room {
    pub id: i64,
    pub name: String,
    pub created_at: NaiveDateTime,
}

impl Room {
    #[instrument(skip_all, fields(room.id = self.id, room.name = self.name), err(Debug))]
    pub async fn get_members(&self, connection: &SqlitePool) -> Result<Vec<Account>, sqlx::Error> {
        let query = sqlx::query_as!(
            Account,
            r#"
                SELECT a.id, a.username, a.password_hash, a.registered_at
                FROM accounts a
                LEFT JOIN room_membership m
                ON a.id = m.account_id
                WHERE m.room_id = ?
            "#,
            self.id
        );
        query.fetch_all(connection).await
    }

    #[instrument(skip_all, fields(room.id = self.id, room.name = self.name), err(Debug))]
    pub async fn get_messages(&self, connection: &SqlitePool) -> Result<Vec<Message>, sqlx::Error> {
        let query = sqlx::query_as!(
            Message,
            r#"
                SELECT * FROM messages WHERE room_id = ?
            "#,
            self.id
        );
        query.fetch_all(connection).await
    }

    #[instrument(skip(self, connection, content), err(Debug))]
    pub async fn send_new_message(
        &self,
        connection: &SqlitePool,
        sender_account_id: i64,
        content: Option<String>,
    ) -> Result<Message, sqlx::Error> {
        let query = sqlx::query_as!(
            Message,
            "INSERT INTO messages (sender_account_id, room_id, content) VALUES (?, ?, ?) RETURNING *",
            sender_account_id,
            self.id,
            content,
        );
        query.fetch_one(connection).await
    }

    #[instrument(skip_all, fields(room.id = self.id, room.name = self.name), err(Debug))]
    pub async fn get_uploads(&self, connection: &SqlitePool) -> Result<Vec<Upload>, sqlx::Error> {
        let query = sqlx::query_as!(
            Upload,
            r#"
                SELECT * FROM file_uploads WHERE room_id = ?
            "#,
            self.id
        );
        query.fetch_all(connection).await
    }

    #[instrument(skip(self, connection, data), err(Debug))]
    pub async fn upload(
        &self,
        connection: &SqlitePool,
        uploader_account_id: i64,
        room_id: i64,
        original_filename: &Path,
        data: &Bytes,
    ) -> Result<Upload, FileUploadError> {
        const DEFAULT_STORE_DIRECTORY: &str = "database/file_uploads/";

        let uuid = Uuid::new_v4();
        let original_filename = original_filename.to_string_lossy();
        let store_path = format!("{DEFAULT_STORE_DIRECTORY}/{uuid}_{original_filename}");

        let mut store_file = File::create(&store_path)
            .inspect(|_| tracing::debug!(store_path, "Created store file"))
            .inspect_err(|error| tracing::error!(?error, "Failed to create store file"))?;
        store_file
            .write_all(data)
            .inspect(|()| tracing::debug!(store_path, "Wrote to store path"))
            .inspect_err(|error| tracing::error!(?error, "Failed to write to store path"))?;

        let store_path = PathBuf::from(store_path).canonicalize()?;
        let store_path = store_path.to_string_lossy();

        let query = sqlx::query_as!(
            Upload,
            r#"
                INSERT INTO file_uploads (uploader_account_id, room_id, original_filename, store_path)
                VALUES (?, ?, ?, ?)
                RETURNING *
            "#,
            uploader_account_id,
            room_id,
            original_filename,
            store_path,
        );

        let upload = query
            .fetch_one(connection)
            .await
            .map_err(FileUploadError::Database)?;

        Ok(upload)
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct RoomRepository {
    pub(super) connection: SqlitePool,
}

impl RoomRepository {
    #[instrument(skip(self), err(Debug))]
    pub async fn find_by_id(&self, room_id: i64) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as!(Room, "SELECT * FROM rooms WHERE id = ?", room_id)
            .fetch_optional(&self.connection)
            .await
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileUploadError {
    Io(#[from] std::io::Error),
    Database(sqlx::Error),
}
