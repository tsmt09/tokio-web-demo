use std::{thread::sleep, time::Duration};

use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockersForm {
    tasks: u64,
    time: u64,
    spawn_blocking: Option<String>,
}

impl BlockersForm {
    fn spawn_blocking(&self) -> bool {
        let Some(blocking) = &self.spawn_blocking else {
            return false;
        };
        matches!(blocking.to_lowercase().as_str(), "on" | "true" | "1")
    }
}

pub async fn blockers(Form(f): Form<BlockersForm>) {
    log::info!("test");
    log::info!(
        "[blockers] spawning {} tasks for {} seconds in {} mode",
        f.tasks,
        f.time,
        if f.spawn_blocking() {
            "blocking"
        } else {
            "nonblocking"
        }
    );
    for i in 0..f.tasks {
        if f.spawn_blocking() {
            tokio::runtime::Handle::current().spawn_blocking(move || {
                log::info!("[blockers] task {i} spawned (blocking)");
                sleep(Duration::from_secs(f.time));
                log::info!("[blockers] task {i} ending");
            });
        } else {
            tokio::spawn(async move {
                log::info!("[blockers] task {i} spawned (blocking)");
                sleep(Duration::from_secs(f.time));
                log::info!("[blockers] task {i} ending");
            });
        }
    }
}
