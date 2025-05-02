use chrono::NaiveDateTime;
use tracing::instrument;

use crate::endpoints::chat::EchoedMessage;
use crate::state::SharedState;

#[derive(sqlx::FromRow, Clone, Debug)]
#[must_use]
pub struct Message {
    pub id: i64,
    pub sender_account_id: i64,
    pub room_id: i64,
    pub content: Option<String>,
    pub sent_at: NaiveDateTime,
    pub file_upload_id: Option<i64>,
}

impl Message {
    #[instrument(skip_all, err(Debug), fields(message.id = self.id))]
    pub async fn to_echoed_message(self, state: &SharedState) -> sqlx::Result<EchoedMessage> {
        let query = sqlx::query!(
            "SELECT username FROM accounts WHERE id = ?",
            self.sender_account_id
        );

        let file_upload = if let Some(file_upload_id) = self.file_upload_id {
            let room = state
                .repository
                .rooms
                .find_by_id(self.room_id)
                .await?
                .unwrap();
            room.get_uploads(&state.db_pool)
                .await?
                .into_iter()
                .find(|upload| upload.id == file_upload_id)
        } else {
            None
        };

        let record = query.fetch_one(&state.db_pool).await?;
        let (file_upload_url, file_upload_original_name) = match file_upload {
            None => (None, None),
            Some(upload) => (
                Some(upload.id.to_string()),
                Some(upload.original_filename.to_string_lossy().to_string()),
            ),
        };

        let echoed_message = EchoedMessage {
            id: self.id,
            sender_username: record.username,
            sender_id: self.sender_account_id,
            room_id: self.room_id,
            text: self.content,
            sent_at: self.sent_at,
            file_upload_url,
            file_upload_original_name,
        };

        Ok(echoed_message)
    }
}
