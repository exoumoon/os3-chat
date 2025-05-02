use chrono::{DateTime, Local};
use std::fmt;

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
