use chrono::NaiveDateTime;
use sqlx::SqlitePool;
use tracing::instrument;

use super::account::Account;
use super::message::Message;

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
