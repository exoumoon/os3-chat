use chrono::{DateTime, Local, NaiveDateTime};
use std::fmt;

#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub registered_at: NaiveDateTime,
}

#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: i64,
    pub token: String,
    pub account_id: i64,
    pub created_at: NaiveDateTime,
    pub expired: bool,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct Message {
    pub text: String,
    pub sender_username: String,
    pub timestamp: DateTime<Local>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{} at {}]: {}",
            self.sender_username,
            self.timestamp.to_rfc2822(),
            self.text
        )
    }
}
