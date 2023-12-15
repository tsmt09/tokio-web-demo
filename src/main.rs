use std::time::Duration;
use askama::Template;
use axum::{routing::get, Router, response::{Html, IntoResponse}, extract::{ws::WebSocket, WebSocketUpgrade}};
use serde_json::json;

#[derive(Template)]
#[template(path = "base.html")]
struct Index{}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let tasks = tokio::runtime::Handle::current().metrics().active_tasks_count();
        let now = chrono::Utc::now();
        let message = json!({
            "time": now.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "value": tasks // Random value between 0-100
        }).to_string();

        if socket.send(axum::extract::ws::Message::Text(message)).await.is_err() {
            break;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::init_timed();
    log::info!("starting tokio web demo");

    let app = Router::new().route("/", get(|| async { 
        Html::from(Index{}.render().unwrap())
    })).route("/ws", get(websocket_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await
}
