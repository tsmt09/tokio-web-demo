use redis::{InfoDict, RedisResult};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use sysinfo::{CpuExt, ProcessExt, SystemExt};
use tokio::{sync::RwLock, task::JoinHandle, time::Interval};

#[derive(Clone)]
pub struct Stats {
    data: Arc<RwLock<VecDeque<Value>>>,
    capacity: usize,
}

impl Stats {
    pub fn new(capacity: usize) -> Self {
        let data = Arc::new(RwLock::new(VecDeque::with_capacity(capacity)));
        Self { data, capacity }
    }
    async fn push(&self, value: Value) {
        let mut write_locked_data = self.data.write().await;
        if write_locked_data.len() >= self.capacity {
            write_locked_data.pop_front();
        }
        write_locked_data.push_back(value);
    }
    async fn all(&self) -> VecDeque<Value> {
        self.data.read().await.clone()
    }
}

pub struct StatsCollector {
    interval: Duration,
    stats: Stats,
    bc: tokio::sync::broadcast::Sender<Value>,
    shutdown: tokio::sync::oneshot::Sender<bool>,
    updater_handle: JoinHandle<()>,
}

impl StatsCollector {
    pub fn new(interval: Duration, capacity: usize) -> StatsCollector {
        let (bc_tx, _) = tokio::sync::broadcast::channel::<Value>(capacity);
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel::<bool>();
        let bc = bc_tx.clone();
        let stats = Stats::new(capacity);
        let updater_interval = tokio::time::interval(interval.clone());
        let updater_stats = stats.clone();
        let updater_handle = tokio::spawn(async move {
            Self::updater(bc_tx, shutdown_rx, updater_stats, updater_interval).await;
        });
        StatsCollector {
            interval,
            stats,
            bc,
            shutdown,
            updater_handle,
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Value> {
        self.bc.subscribe()
    }

    pub async fn get_history(&self) -> VecDeque<Value> {
        self.stats.all().await
    }

    async fn updater(
        bc: tokio::sync::broadcast::Sender<Value>,
        mut shutdown_rx: tokio::sync::oneshot::Receiver<bool>,
        stats: Stats,
        mut interval: Interval,
    ) {
        log::info!("Stats collector task starting");
        let metrics = tokio::runtime::Handle::current().metrics();
        let current_pid = sysinfo::get_current_pid().expect("cannot get pid");
        let url = std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());
        let client = redis::Client::open(url).expect("cannot create redis client");
        let mut conn = client.get_async_connection().await;
        let mut system = sysinfo::System::new_all();
        loop {
            system.refresh_process(current_pid);
            system.refresh_cpu();
            system.refresh_memory();
            interval.tick().await;
            system.refresh_process(current_pid);
            system.refresh_cpu();
            system.refresh_memory();
            let process = system
                .process(current_pid)
                .expect("cannot get current process from system");
            let tasks = metrics.num_alive_tasks();
            let sync_threads = metrics.num_blocking_threads();
            let now = chrono::Utc::now();
            let memory = process.memory() / (1024 * 1024);
            let memory_sys = system.used_memory() / (1024 * 1024);
            let cpu_global = system.global_cpu_info().cpu_usage();
            let cpu_process =
                ((process.cpu_usage() / system.cpus().len() as f32) * 1000.0).round() / 1000.0;
            let keys = if let Ok(ref mut conn) = conn {
                let response: RedisResult<InfoDict> =
                    redis::cmd("INFO").arg("KEYSPACE").query_async(conn).await;
                get_redis_keys_from_result(&response)
            } else {
                0
            };
            let message = json!({
                "time": now.to_rfc3339(),
                "tasks": tasks,
                "sync_threads": sync_threads,
                "mem": memory_sys,
                "mem_proc": memory,
                "cpu": cpu_global,
                "cpu_proc": cpu_process,
                "keys": keys
            });
            log::trace!("{:?}", message);
            stats.push(message.clone()).await;
            let _ = bc.send(message);

            if let Ok(true) = shutdown_rx.try_recv() {
                log::info!("Closing stats updater");
                return;
            }
        }
    }
}

fn get_redis_keys_from_result(response: &RedisResult<InfoDict>) -> u64 {
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
