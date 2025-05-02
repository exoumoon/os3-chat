use chrono::{DateTime, Local};
use std::fmt;

#[derive(sqlx::FromRow, Clone, Debug)]
#[must_use]
pub struct Message {
    pub id: i64,
    pub sender_account_id: i64,
    pub room_id: i64,
    pub content: String,
    pub sent_at: DateTime<Local>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{sender_id} at {sent_at} in room {room_id}]: {text}",
            sender_id = self.sender_account_id,
            sent_at = self.sent_at.to_rfc2822(),
            room_id = self.room_id,
            text = self.content,
        )
    }
}
