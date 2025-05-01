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
    pub token: String,
    pub acccount_id: usize,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct Message {
    pub timestamp: DateTime<Local>,
    pub text: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.timestamp.to_rfc2822(), self.text)
    }
}
