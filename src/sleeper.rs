use std::time::Duration;

// A simple struct that spawns a number of tokio tasks, which are just sleeping
pub struct Sleeper {
    tasks: u64,
    time: Duration,
}

impl Sleeper {
    pub fn new(tasks: u64, time: Duration) -> Self {
        Self { tasks, time }
    }

    pub async fn spawn(&self) {
        for _ in 0..self.tasks {
            let t = self.time;
            tokio::spawn(async move {
                tokio::time::sleep(t).await;
            });
        }
    }
}
