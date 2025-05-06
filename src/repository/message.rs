use chrono::NaiveDateTime;
use tracing::instrument;
use uuid::Uuid;

use crate::endpoints::chat::EchoedMessage;
use crate::state::SharedState;

#[derive(sqlx::FromRow, Clone, Debug)]
#[must_use]
pub struct Message {
    pub id: i64,
    pub sender: String,
    pub room_id: i64,
    pub text: Option<String>,
    pub sent_at: NaiveDateTime,
    pub file_upload_uuid: Option<String>,
}

impl Message {
    #[instrument(skip_all, err(Debug), fields(message.id = self.id))]
    pub async fn to_echoed_message(self, state: &SharedState) -> sqlx::Result<EchoedMessage> {
        let file_upload = if let Some(file_upload_uuid) = self.file_upload_uuid {
            let uuid = file_upload_uuid.parse::<Uuid>().unwrap();
            state.repository.uploads.find(uuid).await?
        } else {
            None
        };

        let (upload_url, upload_filename) = match file_upload {
            None => (None, None),
            Some(upload) => (
                Some(upload.uuid.to_string()),
                Some(upload.filename.to_string_lossy().to_string()),
            ),
        };

        let echoed_message = EchoedMessage {
            id: self.id,
            sender: self.sender,
            room_id: self.room_id,
            text: self.text,
            sent_at: self.sent_at,
            upload_url,
            upload_filename,
        };

        Ok(echoed_message)
    }
}
