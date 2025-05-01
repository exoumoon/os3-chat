use crate::models::Message;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[derive(Debug, Clone)]
#[must_use]
pub struct SharedState {
    pub db_pool: SqlitePool,
    pub messages: Arc<RwLock<Vec<Message>>>,
    pub broadcast_tx: broadcast::Sender<Message>,
}
