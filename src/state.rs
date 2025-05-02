use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::endpoints::chat::EchoedMessage;
use crate::repository::Repository;

#[derive(Debug, Clone)]
#[must_use]
pub struct SharedState {
    pub repository: Repository,
    pub db_pool: SqlitePool,
    pub broadcast_tx: broadcast::Sender<EchoedMessage>,
}
