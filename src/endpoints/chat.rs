use crate::auth::Session;
use crate::repository;
use crate::state::SharedState;
use askama::Template;
use axum::body::Body;
use axum::debug_handler;
use axum::extract::ws::Utf8Bytes;
use axum::extract::{Path, State, WebSocketUpgrade, ws};
use axum::http::{Response, StatusCode};
use axum::response::{Html, IntoResponse};
use futures::{SinkExt, StreamExt};
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
    let room = shared_state
        .repository
        .rooms
        .find_by_id(room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut messages = room
        .get_messages(&shared_state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    messages.retain(|msg| msg.room_id == room_id);
    messages.reverse();
    let template = ChatTemplate {
        logged_in_as: &account.username,
        title: env!("CARGO_CRATE_NAME"),
        room_name: &room.name,
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
    State(state): State<SharedState>,
    Session(account): Session,
    websocket_upgrade: WebSocketUpgrade,
    Path(room_id): Path<i64>,
) -> Result<Response<Body>, StatusCode> {
    let room = state
        .repository
        .rooms
        .find_by_id(room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(websocket_upgrade.on_upgrade(move |socket| async move {
        let broadcast_tx = state.broadcast_tx.clone();
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

            let message = room
                .send_new_message(&state.db_pool, account.id, Some(message.to_string()))
                .await
                .unwrap();

            let _ = broadcast_tx
                .send(message)
                .inspect(|recv_count| tracing::trace!(?recv_count, "Sent data to local broadcast"))
                .inspect_err(|error| tracing::error!(?error, "Local broadcast TX failed"));
        }
    }))
}
