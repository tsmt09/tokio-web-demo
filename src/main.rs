mod sleeper;

use askama::Template;
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade, Query},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router, Form,
};
use serde_json::json;
use tokio::time::Sleep;
use serde::Deserialize;
use std::time::Duration;
use sysinfo::{Cpu, CpuExt, LoadAvg, SystemExt, NetworksExt};

#[derive(Template)]
#[template(path = "base.html")]
struct Index {}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut interval = tokio::time::interval(Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let tasks = tokio::runtime::Handle::current()
            .metrics()
            .active_tasks_count();
        let now = chrono::Utc::now();
        let message = json!({
            "time": now.to_rfc3339(),
            "value": tasks // Random value between 0-100
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
    mem_available: u64
}

async fn stats() -> Html<String> {
    let metrics = tokio::runtime::Handle::current().metrics();
    let active_tasks = metrics.active_tasks_count();
    let num_workers = metrics.num_workers();
    let num_blocking_threads = metrics.num_blocking_threads();
    let io_driver_ready_count = metrics.io_driver_ready_count();
    let sysinfo = sysinfo::System::new_all();
    let load = sysinfo.load_average();
    let cpus: Vec<f32> = sysinfo.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
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
        mem_available
    }
    .render()
    .unwrap()
    .into()
}

#[derive(Deserialize)]
struct SleeperForm {
    tasks: u64,
    time: u64
}

async fn sleeper(Form(f): Form<SleeperForm>) {
    let time = Duration::from_secs(f.time);
    let s = sleeper::Sleeper::new(f.tasks, time);
    s.spawn().await;
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::init_timed();
    log::info!("starting tokio web demo");

    let app = Router::new()
        .route(
            "/",
            get(|| async { Html::from(Index {}.render().unwrap()) }),
        )
        .route("/ws", get(websocket_handler))
        .route("/stats", get(stats))
        .route("/sleeper", post(sleeper));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await
}
