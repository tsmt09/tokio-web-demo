mod blockers;
mod channel;
mod chat;
mod cpu_loadgen;
mod rediskeys;
mod sleeper;
mod stats_collector;
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use chat::Chat;
use redis::{InfoDict, RedisResult};
use serde_json::json;
use stats_collector::StatsCollector;
use std::{sync::Arc, time::Duration};
use sysinfo::{CpuExt, ProcessExt, System, SystemExt};
use tera::{Context, Tera};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(stats_collector): State<Arc<StatsCollector>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, stats_collector))
}

async fn handle_socket(mut socket: WebSocket, stats_collector: Arc<StatsCollector>) {
    let mut subscribe = stats_collector.subscribe();
    loop {
        let result = subscribe.recv().await;
        match result {
            Ok(message) => {
                if socket
                    .send(axum::extract::ws::Message::Text(message.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                log::error!("error receiving message from stats udpater: {:?}", error)
            }
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
    let stats_collector = Arc::new(StatsCollector::new(
        Duration::from_millis(updater_interval()),
        message_count_max(),
    ));
    let app = Router::new()
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .nest(
            "/chat",
            Router::new()
                .route("/", post(chat::chat))
                .route("/ws/:id", get(chat::websocket_handler))
                .with_state(chat),
        )
        .route("/", get(root))
        .route("/stats/ws", get(websocket_handler))
        .route("/sleeper", post(sleeper::sleeper))
        .route("/cpuloadgen", post(cpu_loadgen::load_gen_threads))
        .route("/channel", post(channel::channel))
        .route("/blockers", post(blockers::blockers))
        .route("/rediskeys", post(rediskeys::rediskeys))
        .with_state(stats_collector);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8123").await.unwrap();
    log::info!("starting tokio web demo at http://127.0.0.1:8123");
    axum::serve(listener, app).await
}

async fn root(State(stats_collector): State<Arc<StatsCollector>>) -> impl IntoResponse {
    let tera = Tera::new("templates/**/*").unwrap();
    let stats_history = stats_collector.get_history().await;
    let mut context = Context::new();
    let sysinfo = get_systeminformation();
    log::debug!("{:?}", sysinfo);
    context.insert("sysinfo", &sysinfo);
    context.insert(
        "statsHistory",
        &serde_json::to_string(&stats_history).unwrap(),
    );
    context.insert("messageCountMax", &message_count_max());
    let rendered = tera.render("base.html", &context).unwrap();
    Html::from(rendered)
}

fn get_systeminformation() -> serde_json::Value {
    let mut system = System::new_all();
    let metrics = tokio::runtime::Handle::current().metrics();
    system.refresh_all();
    json!({
        "cpu": system.global_cpu_info(),
        "cpus": system.cpus(),
        "mem": system.total_memory(),
        "uptime": system.uptime(),
        "os": system.long_os_version(),
        "hostname": system.host_name(),
        "workers": metrics.num_workers(),
        "spawned_tasks": metrics.spawned_tasks_count()
    })
}

fn message_count_max() -> usize {
    std::env::var("WS_HISTORY_MESSAGE_COUNT_MAX")
        .unwrap_or("2048".into())
        .parse()
        .unwrap_or(2048)
}

fn updater_interval() -> u64 {
    std::env::var("WS_REFRESH_INTERVAL_MS")
        .unwrap_or("1000".into())
        .parse()
        .unwrap_or(1000)
}
