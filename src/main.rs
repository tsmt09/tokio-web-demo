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
use sysinfo::{CpuExt, LoadAvg, ProcessExt, SystemExt};

#[derive(Template)]
#[template(path = "base.html")]
struct Index {}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut interval = tokio::time::interval(Duration::from_millis(1000));
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

#[derive(Template)]
#[template(path = "stats.html")]
struct Stats {
    active_tasks: usize,
    num_workers: usize,
    num_blocking_threads: usize,
    io_driver_ready_count: u64,
    load: LoadAvg,
    cpus: Vec<f32>,
    mem: u64,
    mem_free: u64,
    mem_available: u64,
}

async fn stats() -> Html<String> {
    let metrics = tokio::runtime::Handle::current().metrics();
    let active_tasks = metrics.active_tasks_count();
    let num_workers = metrics.num_workers();
    let num_blocking_threads = metrics.num_blocking_threads();
    let io_driver_ready_count = metrics.io_driver_ready_count();
    let sysinfo = sysinfo::System::new_all();
    let load = sysinfo.load_average();
    let cpus: Vec<f32> = sysinfo
        .cpus()
        .iter()
        .map(|cpu| cpu.cpu_usage().round())
        .collect();
    let mem = sysinfo.total_memory() / 1_000_000;
    let mem_free = sysinfo.free_memory() / 1_000_000;
    let mem_available = sysinfo.available_memory() / 1_000_000;
    Stats {
        active_tasks,
        num_workers,
        num_blocking_threads,
        io_driver_ready_count,
        load,
        cpus,
        mem,
        mem_free,
        mem_available,
    }
    .render()
    .unwrap()
    .into()
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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    std::env::set_var("RUST_LOG", "DEBUG");
    // console_subscriber::init();
    pretty_env_logger::init_timed();
    log::info!("starting tokio web demo at http://127.0.0.1:8123");

    let app = Router::new()
        .route(
            "/",
            get(|| async { Html::from(Index {}.render().unwrap()) }),
        )
        .route("/ws", get(websocket_handler))
        .route("/stats", get(stats))
        .route("/sleeper", post(sleeper))
        .route("/channel", post(channel))
        .route("/rediskeys", post(rediskeys));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8123").await.unwrap();
    axum::serve(listener, app).await
}
