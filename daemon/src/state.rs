use crate::proto::{DaemonConfig, Task, WorkerInfo};

pub struct AppState {
    pub config_path: String,
    pub queue: Vec<Task>,
    pub workers: Vec<WorkerInfo>,
    pub config: DaemonConfig,
}

impl AppState {
    pub fn new(config_path: String) -> Self {
        Self {
            config_path,
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig {
                workers_count: 4,
                log_level: "info".to_string(),
            },
        }
    }
}
