use std::time::Duration;

use axum::Form;
use serde::Deserialize;
use tokio::{
    sync::broadcast::{error::TryRecvError, Receiver},
    task::JoinHandle,
};

#[derive(Deserialize)]
pub struct CpuLoadGenForm {
    threads: u64,
    duration: u64,
}

pub async fn load_gen_threads(Form(f): Form<CpuLoadGenForm>) {
    tokio::task::spawn(async move {
        log::info!("spawning {} load threads", f.threads);
        let mut jobs: Vec<JoinHandle<()>> = vec![];
        let (tx, _) = tokio::sync::broadcast::channel(1);
        for _ in 0..(f.threads) {
            let mut rx: Receiver<bool> = tx.subscribe();
            jobs.push(tokio::runtime::Handle::current().spawn_blocking(move || {
                let mut f_n: u128 = 0;
                loop {
                    let n = f_n + 1337;
                    f_n = f_n * n;
                    // return if not empty or closed
                    match rx.try_recv() {
                        Err(TryRecvError::Empty) => {}
                        _ => return,
                    }
                }
            }));
        }
        tokio::time::sleep(Duration::from_secs(f.duration)).await;
        log::info!(
            "aborting {} threads after {} seconds",
            f.threads,
            f.duration
        );
        if let Err(e) = tx.send(true) {
            log::error!("{}", e);
        };
        for job in jobs {
            if let Err(e) = job.await {
                log::error!("{}", e);
            }
        }
    });
}
