use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::proto::worker_service_server::WorkerService;
use crate::proto::{WorkerStatus, WorkerStatusResponse};
use crate::state::AppState;

pub struct WorkerServiceImpl {
    state: Arc<Mutex<AppState>>,
}

impl WorkerServiceImpl {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl WorkerService for WorkerServiceImpl {
    async fn get_worker_status(
        &self,
        _request: Request<()>,
    ) -> Result<Response<WorkerStatusResponse>, Status> {
        let state = self.state.lock().await;
        let workers = state.workers.clone();
        let busy = workers
            .iter()
            .filter(|w| w.status == WorkerStatus::Busy as i32)
            .count() as i32;
        let total = workers.len() as i32;
        Ok(Response::new(WorkerStatusResponse {
            total,
            busy,
            idle: total - busy,
            workers,
        }))
    }
}
