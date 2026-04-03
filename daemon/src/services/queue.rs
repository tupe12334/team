use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::proto::queue_service_server::QueueService;
use crate::proto::{
    EnqueueRequest, ListQueueRequest, ListQueueResponse, RemoveTaskRequest, Task, TaskStatus,
    UpdateTaskRequest,
};
use crate::state::AppState;

pub struct QueueServiceImpl {
    state: Arc<Mutex<AppState>>,
}

impl QueueServiceImpl {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl QueueService for QueueServiceImpl {
    async fn enqueue(
        &self,
        request: Request<EnqueueRequest>,
    ) -> Result<Response<Task>, Status> {
        let req = request.into_inner();
        let now = prost_types::Timestamp {
            seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };
        let task = Task {
            id: Uuid::new_v4().to_string(),
            issue_ref: req.issue_ref,
            agent: req.agent.unwrap_or_default(),
            status: TaskStatus::Queued as i32,
            priority: 0,
            created_at: Some(now.clone()),
            updated_at: Some(now),
        };
        let mut state = self.state.lock().await;
        state.queue.push(task.clone());
        Ok(Response::new(task))
    }

    async fn list_queue(
        &self,
        _request: Request<ListQueueRequest>,
    ) -> Result<Response<ListQueueResponse>, Status> {
        let state = self.state.lock().await;
        Ok(Response::new(ListQueueResponse {
            tasks: state.queue.clone(),
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<Task>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().await;
        let task = state
            .queue
            .iter_mut()
            .find(|t| t.id == req.task_id)
            .ok_or_else(|| Status::not_found("task not found"))?;

        if let Some(agent) = req.agent {
            task.agent = agent;
        }
        if let Some(priority) = req.priority {
            task.priority = priority;
        }
        task.updated_at = Some(prost_types::Timestamp {
            seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        });
        Ok(Response::new(task.clone()))
    }

    async fn remove_task(
        &self,
        request: Request<RemoveTaskRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().await;
        let before = state.queue.len();
        state.queue.retain(|t| t.id != req.task_id);
        if state.queue.len() == before {
            return Err(Status::not_found("task not found"));
        }
        Ok(Response::new(()))
    }
}
