use std::{thread::sleep, time::Duration};

use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockersForm {
    tasks: u64,
    time: u64,
}

pub async fn blockers(Form(f): Form<BlockersForm>) {
    for i in 0..f.tasks {
        tokio::spawn(async move {
            log::info!("[blockers] task {i} starting");
            sleep(Duration::from_secs(f.time));
            log::info!("[blockers] task {i} ending");
        });
    }
}
