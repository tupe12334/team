use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::proto::worker_service_server::WorkerService;
use crate::proto::{
    worker_status_response, WorkerStatusData, WorkerStatusRequest, WorkerStatusResponse,
};
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
        _request: Request<WorkerStatusRequest>,
    ) -> Result<Response<WorkerStatusResponse>, Status> {
        let state = self.state.lock().await;
        let workers = state.workers.clone();
        let busy = workers.len() as i32;
        // total reflects the configured pool capacity, not just the running count.
        let total = state.config.workers_count;
        let idle = (total - busy).max(0);
        Ok(Response::new(WorkerStatusResponse {
            result: Some(worker_status_response::Result::Ok(WorkerStatusData {
                total,
                busy,
                idle,
                workers,
            })),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{DaemonConfig, WorkerInfo, WorkerStatus};

    fn make_state(workers_count: i32, running: usize) -> Arc<Mutex<AppState>> {
        let mut workers = Vec::new();
        for i in 0..running {
            workers.push(WorkerInfo {
                worker_id: format!("w{i}"),
                status: WorkerStatus::Busy as i32,
                current_task_id: format!("t{i}"),
                current_agent: String::new(),
                task_started_at: None,
            });
        }
        Arc::new(Mutex::new(AppState {
            config_path: "/tmp/test.toml".into(),
            queue_path: "/tmp/test-queue.json".into(),
            queue: Vec::new(),
            workers,
            config: DaemonConfig {
                workers_count,
                log_level: "info".into(),
                enabled_agents: vec![],
            },
        }))
    }

    #[tokio::test]
    async fn idle_workers_reflects_capacity_not_running_count() {
        let svc = WorkerServiceImpl::new(make_state(4, 1));
        let res = svc.get_worker_status(Request::new(WorkerStatusRequest {})).await.unwrap();
        let data = match res.into_inner().result.unwrap() {
            worker_status_response::Result::Ok(d) => d,
            worker_status_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(data.total, 4);
        assert_eq!(data.busy, 1);
        assert_eq!(data.idle, 3);
        assert_eq!(data.workers.len(), 1);
    }

    #[tokio::test]
    async fn all_idle_when_no_workers_running() {
        let svc = WorkerServiceImpl::new(make_state(2, 0));
        let res = svc.get_worker_status(Request::new(WorkerStatusRequest {})).await.unwrap();
        let data = match res.into_inner().result.unwrap() {
            worker_status_response::Result::Ok(d) => d,
            worker_status_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(data.total, 2);
        assert_eq!(data.busy, 0);
        assert_eq!(data.idle, 2);
    }

    #[tokio::test]
    async fn idle_does_not_go_negative_when_over_capacity() {
        // This shouldn't happen in practice, but guard against it defensively.
        let svc = WorkerServiceImpl::new(make_state(1, 3));
        let res = svc.get_worker_status(Request::new(WorkerStatusRequest {})).await.unwrap();
        let data = match res.into_inner().result.unwrap() {
            worker_status_response::Result::Ok(d) => d,
            worker_status_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(data.total, 1);
        assert_eq!(data.busy, 3);
        assert_eq!(data.idle, 0); // clamped at 0
    }
}
