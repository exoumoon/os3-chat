use crate::repository::Repository;
use crate::repository::message::Message;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
#[must_use]
pub struct SharedState {
    pub repository: Repository,
    pub db_pool: SqlitePool,
    pub broadcast_tx: broadcast::Sender<Message>,
}
