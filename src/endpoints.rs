use crate::SharedState;
use axum::body::Body;
use axum::extract::ws::{Message, Utf8Bytes};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::Response;
use axum::response::Html;
use chrono::Local;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::instrument;

#[instrument]
pub async fn root() -> Html<&'static str> {
    tracing::trace!("Serving root page");
    Html(
        r#"<!DOCTYPE html>
        <html>
        <body>
            <ul id="chat"></ul>
            <input id="msg" placeholder="type a message">
            <script>
                const ws = new WebSocket("ws://" + location.host + "/ws");
                const chat = document.getElementById("chat");
                const input = document.getElementById("msg");

                ws.onmessage = (event) => {
                    const li = document.createElement("li");
                    li.textContent = event.data;
                    chat.appendChild(li);
                };

                input.addEventListener("keydown", e => {
                    if (e.key === "Enter") {
                        ws.send(input.value);
                        input.value = "";
                    }
                });
            </script>
        </body>
        </html>"#,
    )
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

        let messages = Arc::clone(&shared_state.messages);
        tokio::spawn(async move {
            for message in messages.read().await.iter() {
                let utf8_bytes = Utf8Bytes::from(message.to_string());
                let _ = websocket_tx.send(Message::Text(utf8_bytes)).await;
            }

            while let Ok(message) = broadcast_rx.recv().await {
                tracing::debug!(data = ?message, "RECV on local broadcast");
                let text = format!("{}: {}", message.timestamp.to_rfc2822(), message.text);
                let utf8_bytes = Utf8Bytes::from(text);
                if websocket_tx
                    .send(Message::Text(utf8_bytes))
                    .await
                    .inspect(|()| tracing::debug!("Websocket TX ok"))
                    .inspect_err(|error| tracing::error!(?error, "Websocket TX failed"))
                    .is_err()
                {
                    break;
                }
            }
        });

        while let Some(Ok(Message::Text(message))) = websocket_rx.next().await {
            tracing::debug!(data = ?message, "RECV on websocket");

            let current_time = Local::now();
            let message = crate::Message {
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
