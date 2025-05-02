use crate::{models::Message, repository::Repository};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[derive(Debug, Clone)]
#[must_use]
pub struct SharedState {
    pub repository: Repository,
    pub db_pool: SqlitePool,
    pub messages: Arc<RwLock<Vec<Message>>>,
    pub broadcast_tx: broadcast::Sender<Message>,
}
