use std::time::Duration;

use tokio_util::task::TaskTracker;

// A simple struct that spawns a number of tokio tasks, which are just sleeping
pub struct Sleeper {
    tasks: u64,
    time: Duration,
    tracker: TaskTracker 
}

impl Sleeper {
    pub fn new(tasks: u64, time: Duration) -> Self {
        Self {
            tasks,
            time,
            tracker: TaskTracker::new()
        }
    }

    pub async fn spawn(&self) {
        for _ in 0..self.tasks {
            let duration = self.time.clone();
            self.tracker.spawn(async move {
                tokio::time::sleep(duration).await;
            });
        }
    }

    async fn count(&self) -> usize {
        self.tracker.len()
    }
}