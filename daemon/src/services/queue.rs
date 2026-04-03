use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::proto::queue_service_server::QueueService;
use crate::proto::{
    enqueue_response, issue_ref, issue_ref_input, list_queue_response, remove_task_response,
    update_task_response, EnqueueRequest, EnqueueResponse, IssueRef, ListQueueRequest,
    ListQueueResponse, RemoveTaskRequest, RemoveTaskResponse, Task, TaskList, TaskStatus,
    UpdateTaskRequest, UpdateTaskResponse,
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
    ) -> Result<Response<EnqueueResponse>, Status> {
        let req = request.into_inner();
        let now = prost_types::Timestamp {
            seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };
        let issue_ref = match req.issue_ref.and_then(|r| r.r#ref) {
            Some(issue_ref_input::Ref::Github(g)) => {
                Some(IssueRef { r#ref: Some(issue_ref::Ref::Github(g)) })
            }
            Some(issue_ref_input::Ref::Centy(c)) => {
                Some(IssueRef { r#ref: Some(issue_ref::Ref::Centy(c)) })
            }
            Some(issue_ref_input::Ref::Jira(j)) => {
                Some(IssueRef { r#ref: Some(issue_ref::Ref::Jira(j)) })
            }
            Some(issue_ref_input::Ref::Link(_)) => {
                return Ok(Response::new(EnqueueResponse {
                    result: Some(enqueue_response::Result::Error(
                        "link refs must be resolved to a concrete issue before enqueueing"
                            .to_string(),
                    )),
                }));
            }
            None => {
                return Ok(Response::new(EnqueueResponse {
                    result: Some(enqueue_response::Result::Error(
                        "issue_ref is required".to_string(),
                    )),
                }));
            }
        };
        let task = Task {
            id: Uuid::new_v4().to_string(),
            issue_ref,
            agent: req.agent,
            status: TaskStatus::Queued as i32,
            priority: req.priority.unwrap_or(0),
            created_at: Some(now),
            updated_at: Some(now),
        };
        let mut state = self.state.lock().await;
        state.queue.push(task.clone());
        state.save_queue().map_err(Status::internal)?;
        Ok(Response::new(EnqueueResponse {
            result: Some(enqueue_response::Result::Task(task)),
        }))
    }

    async fn list_queue(
        &self,
        _request: Request<ListQueueRequest>,
    ) -> Result<Response<ListQueueResponse>, Status> {
        let state = self.state.lock().await;
        Ok(Response::new(ListQueueResponse {
            result: Some(list_queue_response::Result::Ok(TaskList {
                tasks: state.queue.clone(),
            })),
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<UpdateTaskResponse>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().await;
        let task = state
            .queue
            .iter_mut()
            .find(|t| t.id == req.task_id)
            .ok_or_else(|| Status::not_found("task not found"))?;

        if let Some(agent) = req.agent {
            task.agent = Some(agent);
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
        let task = task.clone();
        state.save_queue().map_err(Status::internal)?;
        Ok(Response::new(UpdateTaskResponse {
            result: Some(update_task_response::Result::Task(task)),
        }))
    }

    async fn remove_task(
        &self,
        request: Request<RemoveTaskRequest>,
    ) -> Result<Response<RemoveTaskResponse>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().await;
        let before = state.queue.len();
        state.queue.retain(|t| t.id != req.task_id);
        if state.queue.len() == before {
            return Err(Status::not_found("task not found"));
        }
        state.save_queue().map_err(Status::internal)?;
        Ok(Response::new(RemoveTaskResponse {
            result: Some(remove_task_response::Result::Ok(())),
        }))
    }
}
