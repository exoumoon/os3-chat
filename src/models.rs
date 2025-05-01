use chrono::{DateTime, Local};
use std::fmt;

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
