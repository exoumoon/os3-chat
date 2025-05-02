use crate::auth::Session;
use crate::{models, state::SharedState};
use askama::Template;
use axum::extract::ws::{Message, Utf8Bytes};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{Response, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::{body::Body, debug_handler};
use chrono::Local;
use futures::{SinkExt, StreamExt};
use tracing::instrument;

pub mod account;

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate;

#[derive(Template)]
#[template(path = "chat.html")]
pub struct ChatTemplate<'a> {
    pub logged_in_as: &'a str,
    pub title: &'a str,
    pub messages: Vec<crate::models::Message>,
}

#[instrument(skip_all)]
#[debug_handler]
pub async fn root() -> Result<impl IntoResponse, StatusCode> {
    AccountTemplate
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[instrument(skip_all, fields(account = ?account))]
#[debug_handler]
pub async fn chat(
    State(shared_state): State<SharedState>,
    Session(account): Session,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::trace!("Serving chat page");
    let mut messages = shared_state.messages.read().await.clone();
    messages.reverse();
    let template = ChatTemplate {
        logged_in_as: &account.username,
        title: env!("CARGO_CRATE_NAME"),
        messages,
    };

    template
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[debug_handler]
#[instrument]
pub async fn websocket(
    State(shared_state): State<SharedState>,
    Session(account): Session,
    websocket_upgrade: WebSocketUpgrade,
) -> Response<Body> {
    websocket_upgrade.on_upgrade(|socket| async move {
        let broadcast_tx = shared_state.broadcast_tx.clone();
        let mut broadcast_rx = broadcast_tx.subscribe();
        let (mut websocket_tx, mut websocket_rx) = socket.split();

        tokio::spawn(async move {
            while let Ok(message) = broadcast_rx.recv().await {
                tracing::trace!(data = ?message, "RECV on local broadcast");
                let utf8_bytes = Utf8Bytes::from(message.to_string());
                match websocket_tx.send(Message::Text(utf8_bytes)).await {
                    Ok(()) => tracing::trace!("Websocket TX ok"),
                    Err(error) => {
                        tracing::warn!(?error, "Websocket TX failed (likely disconnect)");
                        break;
                    }
                }
            }
        });

        while let Some(Ok(Message::Text(message))) = websocket_rx.next().await {
            tracing::trace!(data = ?message, "RECV on websocket");

            let current_time = Local::now();
            let message = models::Message {
                text: message.to_string(),
                sender_username: account.username.clone(),
                timestamp: current_time,
            };

            shared_state.messages.write().await.push(message.clone());
            let _ = broadcast_tx
                .send(message)
                .inspect(|recv_count| tracing::trace!(?recv_count, "Sent data to local broadcast"))
                .inspect_err(|error| tracing::error!(?error, "Local broadcast TX failed"));
        }
    })
}
