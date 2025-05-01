#![allow(clippy::missing_errors_doc)]

use crate::state::SharedState;
use axum::Router;
use axum::routing::{any, get};
use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast};
use tracing::instrument;

pub mod endpoints;
pub mod layers;
pub mod models;
pub mod state;

#[derive(Parser, Clone, Debug)]
#[must_use]
pub struct Settings {
    #[arg(default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3000))]
    pub socket_addr: SocketAddr,

    #[arg(long, default_value_t = 256)]
    pub broadcast_channel_capacity: usize,
}

#[instrument]
pub async fn run(settings: Settings) -> Result<(), color_eyre::eyre::Report> {
    let (broadcast_tx, _) = broadcast::channel(settings.broadcast_channel_capacity);
    let shared_state = SharedState {
        messages: Arc::new(RwLock::new(vec![])),
        broadcast_tx,
    };

    let router = Router::new()
        .route("/", get(endpoints::root))
        .route("/websocket", any(endpoints::websocket))
        .with_state(shared_state)
        .layer(layers::trace_layer());

    let listener = TcpListener::bind(settings.socket_addr).await?;
    tracing::info!(listen_addr = ?listener.local_addr()?, "Bound to local socket");
    axum::serve(listener, router)
        .with_graceful_shutdown(self::shutdown_signal())
        .await?;

    Ok(())
}

#[instrument]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c()
        .await
        .inspect(|()| tracing::info!("Caught CTRL+C signal, shutting down"))
        .inspect_err(|error| tracing::error!(?error, "Failed to await CTRL+C signal"));
}
