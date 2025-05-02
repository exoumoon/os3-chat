use chrono::NaiveDateTime;
use std::fmt;

#[derive(sqlx::FromRow, Clone, Debug)]
#[must_use]
pub struct Message {
    pub id: i64,
    pub sender_account_id: i64,
    pub room_id: i64,
    pub content: Option<String>,
    pub sent_at: NaiveDateTime,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = self.content.clone().unwrap_or_default();
        write!(
            f,
            "[{sender_id} at {sent_at} in room {room_id}]: {content}",
            sender_id = self.sender_account_id,
            sent_at = self.sent_at,
            room_id = self.room_id,
        )
    }
}
