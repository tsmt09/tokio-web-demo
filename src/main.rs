mod blockers;
mod channel;
mod chat;
mod cpu_loadgen;
mod rediskeys;
mod sleeper;
mod soccer_field;
mod stats_collector;

use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    http::Response,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use chat::Chat;
use minijinja::{context, path_loader};
use serde_json::json;
use soccer_field::SoccerFieldThread;
use stats_collector::StatsCollector;
use std::{sync::Arc, time::Duration};
use sysinfo::{System, SystemExt};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(app): State<Arc<AppState>>,
) -> impl IntoResponse {
    let stats_collector = Arc::clone(&app.stats);
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

struct AppState {
    stats: Arc<StatsCollector>,
    chat: Arc<Chat>,
    minij: Arc<minijinja::Environment<'static>>,
    soccer_thread: Arc<SoccerFieldThread>,
}

async fn async_main() -> Result<(), std::io::Error> {
    console_subscriber::init();
    let chat = Arc::new(Chat::new(1000));
    let stats = Arc::new(StatsCollector::new(
        Duration::from_millis(updater_interval()),
        message_count_max(),
    ));
    let mut minij = minijinja::Environment::new();
    minij.set_loader(path_loader("templates"));
    let soccer_thread = Arc::new(soccer_field::SoccerFieldThread::spawn());
    let minij = Arc::new(minij);
    let state = Arc::new(AppState {
        stats,
        chat,
        minij,
        soccer_thread,
    });
    let chat = Router::new()
        .route("/", post(chat::chat))
        .route("/ws/:id", get(chat::websocket_handler))
        .with_state(Arc::clone(&state));
    let mut app = Router::new()
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .route("/", get(root))
        .route("/soccer_field", get(soccer_field::get_field))
        .route("/stats/ws", get(websocket_handler))
        .route("/sleeper", post(sleeper::sleeper))
        .route("/channel", post(channel::channel))
        .route("/soccer_field/ws", get(soccer_field::websocket_handler))
        .with_state(state);
    if chat_enabled() {
        app = app
            .route("/cpuloadgen", post(cpu_loadgen::load_gen_threads))
            .route("/blockers", post(blockers::blockers))
            .route("/rediskeys", post(rediskeys::rediskeys));
    }
    if chat_enabled() {
        app = app.nest("/chat", chat);
    }
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8123").await.unwrap();
    log::info!("starting tokio web demo at http://127.0.0.1:8123");
    axum::serve(listener, app).await
}

async fn root(State(app): State<Arc<AppState>>) -> impl IntoResponse {
    // TODO: Temporary, has to be replaced with auto reloader somewhere
    let mut minij = minijinja::Environment::new();
    minij.set_loader(path_loader("templates"));
    // TODO
    let ctx = context! {
        chat => &chat_enabled(),
        is_redis => &is_redis(),
        sysinfo => &get_systeminformation(),
        statsHistory => &serde_json::to_string(&app.stats.get_history().await).unwrap(),
        messageCountMax => &message_count_max()
    };
    let rendered = minij
        .get_template("base.html")
        .unwrap()
        .render(ctx)
        .unwrap();
    Response::builder()
        .header("Cache-Control", "no-store")
        .body(rendered)
        .unwrap()
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

pub fn chat_enabled() -> bool {
    matches!(
        std::env::var("CHAT")
            .map(|s| s.to_lowercase())
            .unwrap_or("false".into())
            .as_str(),
        "true" | "1"
    )
}

pub fn is_redis() -> bool {
    std::env::var("REDIS_URL").is_ok()
}
