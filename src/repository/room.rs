use std::fs::File;
use std::io::Write;
use std::path::Path;

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
                SELECT a.username, a.password_hash, a.registered_at
                FROM accounts a
                LEFT JOIN room_membership m
                ON a.username = m.member
                WHERE m.room_id = ?
            "#,
            self.id
        );
        query.fetch_all(connection).await
    }

    #[instrument(skip_all, fields(room.id = self.id, room.name = self.name), err(Debug))]
    pub async fn get_messages(&self, connection: &SqlitePool) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as!(Message, "SELECT * FROM messages WHERE room_id = ?", self.id)
            .fetch_all(connection)
            .await
    }

    #[instrument(skip(self, connection, text), err(Debug))]
    pub async fn send_new_message(
        &self,
        connection: &SqlitePool,
        sender: &str,
        text: Option<String>,
    ) -> Result<Message, sqlx::Error> {
        let query = sqlx::query_as!(
            Message,
            "INSERT INTO messages (sender, room_id, text) VALUES (?, ?, ?) RETURNING *",
            sender,
            self.id,
            text,
        );
        query.fetch_one(connection).await
    }

    #[instrument(skip(self, connection, text), err(Debug))]
    pub async fn send_new_message_with_file(
        &self,
        connection: &SqlitePool,
        sender: &str,
        text: Option<String>,
        file_uuid: Uuid,
    ) -> Result<Message, sqlx::Error> {
        let uuid_str = file_uuid.to_string();
        let query = sqlx::query_as!(
            Message,
            "INSERT INTO messages (sender, room_id, text, file_upload_uuid) VALUES (?, ?, ?, ?) RETURNING *",
            sender,
            self.id,
            text,
            uuid_str,
        );
        query.fetch_one(connection).await
    }

    #[instrument(skip_all, fields(filename = ?filename), err(Debug))]
    pub async fn upload(
        &self,
        connection: &SqlitePool,
        filename: &Path,
        data: &Bytes,
    ) -> Result<Upload, FileUploadError> {
        const DEFAULT_STORE_DIRECTORY: &str = "database/file_uploads/";

        let uuid = Uuid::new_v4();
        let filename = filename.to_string_lossy();
        let store_path = format!("{DEFAULT_STORE_DIRECTORY}/{uuid}_{filename}");

        let mut store_file = File::create(&store_path)
            .inspect(|_| tracing::debug!(store_path, "Created store file"))
            .inspect_err(|error| tracing::error!(?error, "Failed to create store file"))?;
        store_file
            .write_all(data)
            .inspect(|()| tracing::debug!(store_path, "Wrote to store path"))
            .inspect_err(|error| tracing::error!(?error, "Failed to write to store path"))?;

        let uuid_string = uuid.to_string();

        let upload = sqlx::query_as!(
            Upload,
            r#"
                INSERT INTO file_uploads (uuid, filename)
                VALUES (?, ?)
                RETURNING *
            "#,
            uuid_string,
            filename,
        )
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
