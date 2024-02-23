mod channel;
mod rediskeys;
mod sleeper;

use askama::Template;
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use sysinfo::{ProcessExt, SystemExt};

#[derive(Template)]
#[template(path = "base.html")]
struct Index {}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    let metrics = tokio::runtime::Handle::current().metrics();
    let current_pid = sysinfo::get_current_pid().expect("cannot get pid");
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
        let message = json!({
            "time": now.to_rfc3339(),
            "tasks": tasks, // Random value between 0-100
            "memory": memory,
            "cpu": cpu
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

#[derive(Deserialize)]
struct SleeperForm {
    tasks: u64,
    time: u64,
}

async fn sleeper(Form(f): Form<SleeperForm>) {
    let time = Duration::from_secs(f.time);
    let s = sleeper::Sleeper::new(f.tasks, time);
    s.spawn().await;
}

#[derive(Deserialize)]
struct ChannelForm {
    tasks: u64,
    time: u64,
    repeat: u64,
}

async fn channel(Form(f): Form<ChannelForm>) {
    let time = Duration::from_secs(f.time);
    let c = channel::Channel::new(f.tasks, time, f.repeat);
    c.spawn().await;
}

#[derive(Deserialize)]
struct RedisKeysForm {
    tasks: u64,
    keys: u64,
}

async fn rediskeys(Form(f): Form<RedisKeysForm>) {
    let r = rediskeys::RedisKeys::new(f.tasks, f.keys);
    r.spawn().await;
}

fn main() {
    std::env::set_var("RUST_LOG", "DEBUG");
    // console_subscriber::init();
    pretty_env_logger::init_timed();
    log::info!("starting tokio web demo at http://127.0.0.1:8123");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .max_blocking_threads(1)
        .thread_keep_alive(Duration::from_millis(10))
        .build()
        .unwrap();
    rt.block_on(async {
        let _ = async_main().await;
    });
}

async fn async_main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html::from(Index {}.render().unwrap()) }),
        )
        .route("/ws", get(websocket_handler))
        .route("/sleeper", post(sleeper))
        .route("/channel", post(channel))
        .route("/rediskeys", post(rediskeys));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8123").await.unwrap();
    axum::serve(listener, app).await
}
