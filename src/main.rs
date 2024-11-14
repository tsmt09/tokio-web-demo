mod blockers;
mod channel;
mod chat;
mod cpu_loadgen;
mod rediskeys;
mod sleeper;
use axum::{
    body::HttpBody,
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use chat::Chat;
use redis::{InfoDict, RedisResult};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use sysinfo::{ProcessExt, SystemExt};
use tera::{Context, Tera};
use tokio_util::context;

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
        let tasks = metrics.num_alive_tasks();
        let sync_threads = metrics.num_blocking_threads();
        let now = chrono::Utc::now();
        let memory = process.memory() / 1_000_000;
        let cpu = process.cpu_usage();
        tokio::time::sleep(Duration::from_millis(5)).await;
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
            "tasks": tasks,
            "sync_threads": sync_threads,
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
        .build()
        .unwrap();
    rt.block_on(async {
        let _ = async_main().await;
    });
}

async fn async_main() -> Result<(), std::io::Error> {
    console_subscriber::init();
    let chat = Arc::new(Chat::new(1000));
    let app = Router::new()
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .route(
            "/",
            get(|| async {
                let tera = Tera::new("templates/**/*").unwrap();
                let interval_ms: u64 = std::env::var("WS_REFRESH_INTERVAL_MS")
                    .unwrap_or("1000".into())
                    .parse()
                    .unwrap_or(1000);
                let metrics = tokio::runtime::Handle::current().metrics();
                let mut context = Context::new();
                context.insert("threads", &metrics.num_workers());
                context.insert("blocking_threads", &metrics.num_blocking_threads());
                context.insert("interval_ms", &interval_ms);
                let rendered = tera.render("base.html", &context).unwrap();
                Html::from(rendered)
            }),
        )
        .route("/ws", get(websocket_handler))
        .route("/ws/chat/:id", get(chat::websocket_handler))
        .route("/sleeper", post(sleeper::sleeper))
        .route("/cpuloadgen", post(cpu_loadgen::load_gen_threads))
        .route("/channel", post(channel::channel))
        .route("/blockers", post(blockers::blockers))
        .route("/rediskeys", post(rediskeys::rediskeys))
        .route("/chat", post(chat::chat))
        .with_state(chat);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8123").await.unwrap();
    log::info!("starting tokio web demo at http://127.0.0.1:8123");
    axum::serve(listener, app).await
}
