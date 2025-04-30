use axum::Router;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::response::Html;
use axum::routing::get;
use color_eyre::eyre::Report;
use futures::SinkExt;
use futures::stream::StreamExt;
use std::net::Ipv4Addr;
use tokio::{net::TcpListener, sync::broadcast};

#[tokio::main]
async fn main() -> Result<(), Report> {
    let (tx, _rx) = broadcast::channel::<String>(100);

    let app = Router::new().route("/", get(show_chat_page)).route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let tx = tx.clone();
            async move { ws.on_upgrade(move |socket| handle_socket(socket, tx)) }
        }),
    );

    let addr = Ipv4Addr::UNSPECIFIED;
    let listener = TcpListener::bind((addr, 3000)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_socket(socket: WebSocket, tx: broadcast::Sender<String>) {
    let mut rx = tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            let utf8_bytes = Utf8Bytes::from(message);
            if sender.send(Message::Text(utf8_bytes)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(Message::Text(message))) = receiver.next().await {
        let _ = tx.send(message.to_string());
    }
}

async fn show_chat_page() -> Html<&'static str> {
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
