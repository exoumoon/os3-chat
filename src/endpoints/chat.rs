use crate::auth::Session;
use crate::repository;
use crate::state::SharedState;
use askama::Template;
use axum::body::Body;
use axum::debug_handler;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws;
use axum::extract::ws::Utf8Bytes;
use axum::http::Response;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use chrono::Local;
use futures::SinkExt;
use futures::StreamExt;
use tracing::instrument;

#[derive(Template)]
#[template(path = "chat.html")]
pub struct ChatTemplate<'a> {
    pub logged_in_as: &'a str,
    pub title: &'a str,
    pub room_name: &'a str,
    pub room_id: i64,
    pub messages: Vec<repository::message::Message>,
}

#[instrument(skip_all, fields(account = ?account))]
#[debug_handler]
pub async fn page(
    State(shared_state): State<SharedState>,
    Session(account): Session,
    Path(room_id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::trace!("Serving chat page");
    let mut messages = shared_state.messages.read().await.clone();
    messages.retain(|msg| msg.room_id == room_id);
    messages.reverse();
    let template = ChatTemplate {
        logged_in_as: &account.username,
        title: env!("CARGO_CRATE_NAME"),
        room_name: "FIXME_hardcoded",
        room_id,
        messages,
    };

    template
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[debug_handler]
#[instrument(skip_all, fields(username = account.username, room_id = room_id))]
pub async fn websocket(
    State(shared_state): State<SharedState>,
    Session(account): Session,
    websocket_upgrade: WebSocketUpgrade,
    Path(room_id): Path<i64>,
) -> Response<Body> {
    websocket_upgrade.on_upgrade(move |socket| async move {
        let broadcast_tx = shared_state.broadcast_tx.clone();
        let mut broadcast_rx = broadcast_tx.subscribe();
        let (mut websocket_tx, mut websocket_rx) = socket.split();

        tokio::spawn(async move {
            while let Ok(message) = broadcast_rx.recv().await {
                if message.room_id != room_id {
                    tracing::debug!("Message does not belong to this room, skipping");
                    continue;
                }

                tracing::trace!(data = ?message, "RECV on local broadcast");
                let utf8_bytes = Utf8Bytes::from(message.to_string());
                match websocket_tx.send(ws::Message::Text(utf8_bytes)).await {
                    Ok(()) => tracing::trace!("Websocket TX ok"),
                    Err(error) => {
                        tracing::warn!(?error, "Websocket TX failed (likely disconnect)");
                        break;
                    }
                }
            }
        });

        while let Some(Ok(ws::Message::Text(message))) = websocket_rx.next().await {
            tracing::trace!(data = ?message, "RECV on websocket");

            let current_time = Local::now();
            let message = repository::message::Message {
                id: 1234,
                sender_account_id: account.id,
                room_id,
                content: message.to_string(),
                sent_at: current_time,
            };

            shared_state.messages.write().await.push(message.clone());
            let _ = broadcast_tx
                .send(message)
                .inspect(|recv_count| tracing::trace!(?recv_count, "Sent data to local broadcast"))
                .inspect_err(|error| tracing::error!(?error, "Local broadcast TX failed"));
        }
    })
}
