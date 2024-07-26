mod blockers;
mod channel;
mod chat;
mod rediskeys;
mod sleeper;

use askama::Template;
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use chat::Chat;
use redis::{InfoDict, RedisResult};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use sysinfo::{ProcessExt, SystemExt};

#[derive(Template)]
#[template(path = "base.html")]
struct Index {
    threads: usize,
    blocking_threads: usize,
    interval_ms: u64,
}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn get_redis_keys_from_result(response: &RedisResult<InfoDict>) -> u64 {
    if let Ok(response) = response {
        let db0: Option<String> = response.get("db0");
        if let Some(db0) = db0 {
            // unwraps "keys=123,bla=123,blabla=123..."
            let x: Option<u64> = db0
                .split_once(',')
                .map(|(x, _)| x.split_once('=').map(|(_, x)| x.parse().ok()))
                .unwrap_or(Some(Some(0)))
                .unwrap_or(Some(0));
            x.unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    }
}

async fn handle_socket(mut socket: WebSocket) {
    let interval_ms: u64 = std::env::var("WS_REFRESH_INTERVAL_MS")
        .unwrap_or("1000".into())
        .parse()
        .unwrap_or(1000);
    let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
    let metrics = tokio::runtime::Handle::current().metrics();
    let current_pid = sysinfo::get_current_pid().expect("cannot get pid");
    let url = std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());
    let client = redis::Client::open(url).expect("cannot create redis client");
    let mut conn = client.get_async_connection().await;
    let mut system = sysinfo::System::new_all();
    loop {
        interval.tick().await;
        system.refresh_process(current_pid);
        let process = system
            .process(current_pid)
            .expect("cannot get current process from system");
        let tasks = metrics.active_tasks_count();
        let now = chrono::Utc::now();
        let memory = process.memory() / 1_000_000;
        let cpu = process.cpu_usage();
        let keys = if let Ok(ref mut conn) = conn {
            let response: RedisResult<InfoDict> =
                redis::cmd("INFO").arg("KEYSPACE").query_async(conn).await;
            get_redis_keys_from_result(&response).await
        } else {
            0
        };
        let message = json!({
            "time": now.to_rfc3339(),
            "tasks": tasks, // Random value between 0-100
            "memory": memory,
            "cpu": cpu,
            "keys": keys
        })
        .to_string();

        if socket
            .send(axum::extract::ws::Message::Text(message))
            .await
            .is_err()
        {
            break;
        }
    }
}

fn main() {
    let _ = dotenv::dotenv();
    pretty_env_logger::init_timed();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_keep_alive(Duration::from_millis(10))
        .build()
        .unwrap();
    rt.block_on(async {
        let _ = async_main().await;
    });
}

async fn async_main() -> Result<(), std::io::Error> {
    let chat = Arc::new(Chat::new(1000));
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                let interval_ms: u64 = std::env::var("WS_REFRESH_INTERVAL_MS")
                    .unwrap_or("1000".into())
                    .parse()
                    .unwrap_or(1000);
                let metrics = tokio::runtime::Handle::current().metrics();
                let threads = metrics.num_blocking_threads();
                let blocking_threads = metrics.num_blocking_threads();
                Html::from(
                    Index {
                        threads,
                        blocking_threads,
                        interval_ms,
                    }
                    .render()
                    .unwrap(),
                )
            }),
        )
        .route("/ws", get(websocket_handler))
        .route("/ws/chat/:id", get(chat::websocket_handler))
        .route("/sleeper", post(sleeper::sleeper))
        .route("/channel", post(channel::channel))
        .route("/blockers", post(blockers::blockers))
        .route("/rediskeys", post(rediskeys::rediskeys))
        .route("/chat", post(chat::chat))
        .with_state(chat);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8123").await.unwrap();
    log::info!("starting tokio web demo at http://127.0.0.1:8123");
    axum::serve(listener, app).await
}
