use askama::Template;
use axum::body::Body;
use axum::debug_handler;
use axum::extract::ws::Utf8Bytes;
use axum::extract::{Path, State, WebSocketUpgrade, ws};
use axum::http::{Response, StatusCode};
use axum::response::{Html, IntoResponse};
use chrono::NaiveDateTime;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::auth::Session;
use crate::state::SharedState;

#[derive(Deserialize, Clone, Debug)]
#[must_use]
pub struct IncomingMessage {
    pub room_id: i64,
    pub text: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[must_use]
pub struct EchoedMessage {
    pub id: i64,
    pub sender: String,
    pub room_id: i64,
    pub text: Option<String>,
    pub sent_at: NaiveDateTime,
    pub upload_filename: Option<String>,
    pub upload_url: Option<String>,
}

#[derive(Template)]
#[template(path = "chat.html")]
pub struct ChatTemplate<'a> {
    pub title: &'a str,
    pub logged_in_as: &'a str,
    pub room_name: &'a str,
    pub room_id: i64,
    pub initial_messages_json: String,
}

#[instrument(skip_all, fields(account = ?account))]
#[debug_handler]
pub async fn page(
    State(state): State<SharedState>,
    Session(account): Session,
    Path(room_id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::trace!("Serving chat page");
    let room = state
        .repository
        .rooms
        .find_by_id(room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut echoed_messages = vec![];
    let is_member = room
        .get_members(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .any(|m| m.username == account.username);
    if is_member {
        for message in room
            .get_messages(&state.db_pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .filter(|msg| msg.room_id == room_id)
        {
            let echoed_message = message
                .to_echoed_message(&state)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            echoed_messages.push(echoed_message);
        }
    } else {
        tracing::warn!("User is not a member of this room, retuning no messages");
    }

    let initial_messages_json =
        serde_json::to_string(&echoed_messages).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = ChatTemplate {
        logged_in_as: &account.username,
        title: env!("CARGO_CRATE_NAME"),
        room_name: &room.name,
        room_id,
        initial_messages_json,
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
    let is_member = room
        .get_members(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .any(|m| m.username == account.username);

    if !is_member {
        tracing::warn!("User is not a member of this room, rejecting websocket");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let callback = move |socket: ws::WebSocket| async move {
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
                let json_repr = serde_json::to_string(&message).unwrap();
                let utf8_bytes = Utf8Bytes::from(json_repr);
                match websocket_tx.send(ws::Message::Text(utf8_bytes)).await {
                    Ok(()) => tracing::trace!("Websocket TX ok"),
                    Err(error) => {
                        tracing::warn!(?error, "Websocket TX failed (likely disconnect)");
                        break;
                    }
                }
            }
        });

        while let Some(Ok(ws::Message::Text(incoming_json))) = websocket_rx.next().await {
            tracing::trace!(data = ?incoming_json, "RECV on websocket");

            // NOTE: Здесь мы декодируем сырое сообщение через WebSocket от клиента. В нём
            // известно только содержимое сообщения и ID комнаты, в которой должно оказаться
            // это сообщение. ID отправителя мы уже знаем по сессии.
            let incoming_message = serde_json::from_str::<IncomingMessage>(&incoming_json).unwrap();

            assert_eq!(incoming_message.room_id, room.id);

            // NOTE: Сохраняем полученные данные в БД, получая обратно полноценное
            // отображение новой строки со временем отправки и другими данными.
            let repo_message = room
                .send_new_message(&state.db_pool, &account.username, incoming_message.text)
                .await
                .unwrap();

            // NOTE: Дополняем "строчку из БД", полученную ранее всеми данными, которые
            // необходимы клиенту для отрисовки сообщения. Далее оно отправится в локальный
            // поток сообщений, где все активные слушатели данной комнаты получат его и
            // отправят в соответствующие WebSocketы.
            let echoed_message = repo_message.to_echoed_message(&state).await.unwrap();

            let _ = broadcast_tx
                .send(echoed_message)
                .inspect(|recv_count| tracing::trace!(?recv_count, "Sent data to local broadcast"))
                .inspect_err(|error| tracing::error!(?error, "Local broadcast TX failed"));
        }
    };

    Ok(websocket_upgrade.on_upgrade(callback))
}
