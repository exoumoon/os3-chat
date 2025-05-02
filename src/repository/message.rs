use chrono::NaiveDateTime;
use sqlx::SqlitePool;
use tracing::instrument;

use crate::endpoints::chat::EchoedMessage;

#[derive(sqlx::FromRow, Clone, Debug)]
#[must_use]
pub struct Message {
    pub id: i64,
    pub sender_account_id: i64,
    pub room_id: i64,
    pub content: Option<String>,
    pub sent_at: NaiveDateTime,
}

impl Message {
    #[instrument(skip_all, err(Debug), fields(message.id = self.id))]
    pub async fn to_echoed_message(
        self,
        db_connection: &SqlitePool,
    ) -> sqlx::Result<EchoedMessage> {
        let query = sqlx::query!(
            "SELECT username FROM accounts WHERE id = ?",
            self.sender_account_id
        );

        let record = query.fetch_one(db_connection).await?;
        let echoed_message = EchoedMessage {
            id: self.id,
            sender_username: record.username,
            sender_id: self.sender_account_id,
            room_id: self.room_id,
            text: self.content,
            sent_at: self.sent_at,
        };

        Ok(echoed_message)
    }
}
