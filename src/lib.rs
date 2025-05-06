#![allow(clippy::missing_errors_doc)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::from_extractor_with_state;
use axum::response::Redirect;
use axum::routing::{any, get, post};
use clap::Parser;
use repository::Repository;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::instrument;

use crate::state::SharedState;

const GIGABYTE: usize = 1024 * 1024 * 1024;

pub mod auth;
pub mod endpoints;
pub mod layers;
pub mod repository;
pub mod state;

#[derive(Parser, Clone, Debug)]
#[must_use]
pub struct Settings {
    #[arg(default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3000))]
    pub socket_addr: SocketAddr,

    #[arg(long("sqlite-db"), default_value_t = env!("DATABASE_URL").to_string())]
    pub database_url: String,

    #[arg(long, default_value_t = 256)]
    pub broadcast_channel_capacity: usize,
}

#[instrument]
pub async fn run(settings: Settings) -> Result<(), color_eyre::eyre::Report> {
    let db_pool = SqlitePool::connect(&settings.database_url).await?;
    let (broadcast_tx, _) = broadcast::channel(settings.broadcast_channel_capacity);
    let state = SharedState {
        repository: Repository::new(db_pool.clone()),
        db_pool,
        broadcast_tx,
    };

    let file_router = Router::new()
        .route("/upload", post(endpoints::upload::upload_handler))
        .route("/upload/{uuid}", get(endpoints::upload::download_handler))
        .layer(DefaultBodyLimit::max(GIGABYTE));

    let room_router = Router::new()
        .route("/list", get(endpoints::rooms::list))
        .route("/create", post(endpoints::rooms::create));

    let protected_router = Router::new()
        .merge(file_router)
        .nest("/api/room/", room_router)
        .route("/account/logout", post(endpoints::account::logout))
        .route("/chat/{room_id}", get(endpoints::chat::page))
        .route("/chat/{room_id}/websocket", any(endpoints::chat::websocket))
        .route_layer(from_extractor_with_state::<auth::Session, _>(state.clone()));

    let router = Router::new()
        .merge(protected_router)
        .route("/", get(|| async { Redirect::to("/chat/1") }))
        .route("/account", get(endpoints::account::page))
        .route("/account/form/submit", post(endpoints::account::submit))
        .layer(layers::trace_layer())
        .with_state(state);

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
