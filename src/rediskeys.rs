use axum::Form;
use redis::{AsyncCommands, RedisResult};
use serde::Deserialize;
use tokio::task::JoinHandle;

#[derive(Deserialize)]
pub struct RedisKeysForm {
    tasks: u64,
    keys: u64,
    delete: Option<String>,
}

impl RedisKeysForm {
    pub fn delete(&self) -> bool {
        let Some(delete) = &self.delete else {
            return false;
        };
        matches!(delete.to_lowercase().as_str(), "on" | "true" | "1")
    }
}

pub async fn rediskeys(Form(f): Form<RedisKeysForm>) {
    let mut handles: Vec<JoinHandle<()>> = vec![];
    for task_nr in 0..f.tasks {
        let keys = f.keys;
        let handle = tokio::spawn(async move {
            log::debug!("task {} inserting {} keys.", task_nr, keys);
            let url = std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());
            let client = redis::Client::open(url).expect("cannot create redis client");
            let mut con = client
                .get_async_connection()
                .await
                .expect("cannot get async connection");
            for key_nr in 0..keys {
                let key = format!("{task_nr}:{key_nr}");
                let _: RedisResult<String> = con.set(key.as_str(), key.as_str()).await;
            }
            log::debug!("task {} done inserting", task_nr);
        });
        handles.push(handle);
    }
    for handle in handles.into_iter() {
        let _ = handle.await;
    }
    let mut handles: Vec<JoinHandle<()>> = vec![];
    log::debug!(
        "done inserting {} keys with {} workers.",
        f.keys * f.tasks,
        f.tasks
    );
    if !f.delete() {
        return;
    }
    for task_nr in 0..f.tasks {
        let keys = f.keys;
        let handle = tokio::spawn(async move {
            log::debug!("task {} deleting {} keys.", task_nr, keys);
            let url = std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());
            let client = redis::Client::open(url).expect("cannot create redis client");
            let mut con = client
                .get_async_connection()
                .await
                .expect("cannot get async connection");
            for key_nr in 0..keys {
                let key = format!("{task_nr}:{key_nr}");
                let _: RedisResult<String> = con.get_del(key.as_str()).await;
            }
            log::debug!("task {} done deleting", task_nr);
        });
        handles.push(handle);
    }
    for handle in handles.into_iter() {
        let _ = handle.await;
    }
    log::debug!(
        "done deleting {} keys with {} workers.",
        f.keys * f.tasks,
        f.tasks
    );
}
