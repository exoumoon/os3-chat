use crate::models::Message;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[derive(Debug, Clone)]
#[must_use]
pub struct SharedState {
    pub messages: Arc<RwLock<Vec<Message>>>,
    pub broadcast_tx: broadcast::Sender<Message>,
}
