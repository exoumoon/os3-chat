use crate::models;
use crate::state::SharedState;
use axum::body::Body;
use axum::extract::ws::{Message, Utf8Bytes};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::Response;
use axum::response::Html;
use chrono::Local;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::instrument;

#[axum::debug_handler]
pub async fn root() -> Html<&'static str> {
    tracing::trace!("Serving root page");
    let html = indoc::indoc! {r#"
        <!DOCTYPE html>
        <html>
        <body>
            <input id="message_text_input" placeholder="...">
            <ul id="messages"></ul>
            <script>
                const websocket = new WebSocket("ws://" + location.host + "/websocket");
                const chat = document.getElementById("messages");
                const input = document.getElementById("message_text_input");

                websocket.onmessage = (event) => {
                    const new_message = document.createElement("li");
                    new_message.textContent = event.data;
                    chat.prepend(new_message);
                };

                input.addEventListener("keydown", event => {
                    if (event.key === "Enter" && input.value) {
                        websocket.send(input.value);
                        input.value = "";
                    }
                });
            </script>
        </body>
        </html>
        "#
    };

    Html(html)
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
