use crate::models;
use crate::state::SharedState;
use askama::Template;
use axum::body::Body;
use axum::extract::ws::{Message, Utf8Bytes};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{Response, StatusCode};
use axum::response::{Html, IntoResponse};
use chrono::Local;
use futures::{SinkExt, StreamExt};
use tracing::instrument;

pub mod account;

#[derive(Template)]
#[template(path = "chat.html")]
pub struct ChatTemplate {
    pub title: &'static str,
    pub messages: Vec<crate::models::Message>,
}

#[axum::debug_handler]
pub async fn root(
    State(shared_state): State<SharedState>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::trace!("Serving root page");
    let mut messages = shared_state.messages.read().await.clone();
    messages.reverse();
    let template = ChatTemplate {
        title: env!("CARGO_CRATE_NAME"),
        messages,
    };

    template
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[axum::debug_handler]
#[instrument]
pub async fn websocket(
    State(shared_state): State<SharedState>,
    websocket_upgrade: WebSocketUpgrade,
) -> Response<Body> {
    websocket_upgrade.on_upgrade(|socket| async move {
        let broadcast_tx = shared_state.broadcast_tx.clone();
        let mut broadcast_rx = broadcast_tx.subscribe();
        let (mut websocket_tx, mut websocket_rx) = socket.split();

        tokio::spawn(async move {
            while let Ok(message) = broadcast_rx.recv().await {
                tracing::debug!(data = ?message, "RECV on local broadcast");
                let utf8_bytes = Utf8Bytes::from(message.to_string());
                match websocket_tx.send(Message::Text(utf8_bytes)).await {
                    Ok(()) => tracing::debug!("Websocket TX ok"),
                    Err(error) => {
                        tracing::error!(?error, "Websocket TX failed");
                        break;
                    }
                }
            }
        });

        while let Some(Ok(Message::Text(message))) = websocket_rx.next().await {
            tracing::debug!(data = ?message, "RECV on websocket");

            let current_time = Local::now();
            let message = models::Message {
                timestamp: current_time,
                text: message.to_string(),
            };

            shared_state.messages.write().await.push(message.clone());
            let _ = broadcast_tx
                .send(message)
                .inspect(|recv_count| tracing::debug!(?recv_count, "Sent data to local broadcast"))
                .inspect_err(|error| tracing::error!(?error, "Local broadcast TX failed"));
        }
    })
}
