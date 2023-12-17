use std::time::Duration;
use tokio_util::task::TaskTracker;

#[derive(PartialEq)]
enum Message {
    Ping,
    Terminate,
}

pub struct Channel {
    tasks: u64,
    time: Duration,
    repeat: u64,
}

impl Channel {
    pub fn new(tasks: u64, time: Duration, repeat: u64) -> Self {
        Self {
            tasks,
            time,
            repeat,
        }
    }
    pub async fn spawn(&self) {
        let (tx, rx) = tokio::sync::watch::channel(Message::Ping);
        let mut count = self.repeat;
        let wait = self.time;
        // spawn sending task
        tokio::task::spawn(async move {
            while count > 0 {
                tx.send(Message::Ping).expect("error sending ping command");
                log::debug!("sent ping #{count}");
                tokio::time::sleep(wait).await;
                count -= 1;
            }
            tx.send(Message::Terminate)
                .expect("error sending Terminate command");
        });
        // spawn multiple receiving tasks
        for _ in 0..self.tasks {
            let mut rx = rx.clone();
            tokio::task::spawn(async move {
                let _ = rx.wait_for(|x| x == &Message::Terminate).await;
            });
        }
    }
}
