use crate::proto::{Task, WorkerInfo};

pub struct AppState {
    pub config_path: String,
    pub queue: Vec<Task>,
    pub workers: Vec<WorkerInfo>,
}

impl AppState {
    pub fn new(config_path: String) -> Self {
        Self {
            config_path,
            queue: Vec::new(),
            workers: Vec::new(),
        }
    }
}
