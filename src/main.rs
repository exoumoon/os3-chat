use axum::Router;
use axum::routing::{any, get};
use chrono::{DateTime, Local};
use color_eyre::eyre::Report;
use error_layer::ErrorLayer;
use std::fmt;
use std::{net::Ipv4Addr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast};

pub const BROADCAST_CHANNEL_CAPACITY: usize = 256;

pub mod endpoints;
pub mod error_layer;

#[derive(Debug, Clone)]
#[must_use]
pub struct Message {
    timestamp: DateTime<Local>,
    text: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.timestamp.to_rfc2822(), self.text)
    }
}

#[derive(Debug, Clone)]
#[must_use]
struct SharedState {
    messages: Arc<RwLock<Vec<Message>>>,
    broadcast_tx: broadcast::Sender<Message>,
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    ErrorLayer.setup()?;

    let (broadcast_tx, _) = broadcast::channel::<Message>(BROADCAST_CHANNEL_CAPACITY);
    let shared_state = SharedState {
        messages: Arc::new(RwLock::new(vec![])),
        broadcast_tx,
    };

    let app = Router::new()
        .route("/", get(endpoints::root))
        .route("/websocket", any(endpoints::websocket))
        .with_state(shared_state);

    let addr = Ipv4Addr::UNSPECIFIED;
    let listener = TcpListener::bind((addr, 3000)).await?;
    tracing::info!(local_addr = ?listener.local_addr()?, "Server started");
    axum::serve(listener, app).await?;

    Ok(())
}
