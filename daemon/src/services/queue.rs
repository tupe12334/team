use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::link_resolver;
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
            Some(issue_ref_input::Ref::Jira(_)) => {
                return Ok(Response::new(EnqueueResponse {
                    result: Some(enqueue_response::Result::Error(
                        "Jira issues are not supported by worktree-io; use GitHub or Centy".to_string(),
                    )),
                }));
            }
            Some(issue_ref_input::Ref::Link(l)) => {
                match link_resolver::resolve(&l.url).await {
                    Ok(resolved) => {
                        if matches!(&resolved.r#ref, Some(issue_ref::Ref::Jira(_))) {
                            return Ok(Response::new(EnqueueResponse {
                                result: Some(enqueue_response::Result::Error(
                                    "Jira issues are not supported by worktree-io; use GitHub or Centy".to_string(),
                                )),
                            }));
                        }
                        Some(resolved)
                    }
                    Err(msg) => {
                        return Ok(Response::new(EnqueueResponse {
                            result: Some(enqueue_response::Result::Error(msg)),
                        }));
                    }
                }
            }
            None => {
                return Ok(Response::new(EnqueueResponse {
                    result: Some(enqueue_response::Result::Error(
                        "issue_ref is required".to_string(),
                    )),
                }));
            }
        };
        // Validate agent name before acquiring the state lock.
        if let Some(ref agent_name) = req.agent
            && !agent_name.is_empty()
            && !crate::gstack_agents::is_known(agent_name)
        {
            return Ok(Response::new(EnqueueResponse {
                result: Some(enqueue_response::Result::Error(
                    format!("unknown agent '{agent_name}'; use GetAvailableAgents to list valid agents"),
                )),
            }));
        }

        if req.priority.is_some_and(|p| p < 0) {
            return Ok(Response::new(EnqueueResponse {
                result: Some(enqueue_response::Result::Error(
                    "priority must be >= 0".to_string(),
                )),
            }));
        }

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
        // If enabled_agents is configured, the chosen agent must be in the allowed set.
        if let Some(ref agent_name) = task.agent
            && !agent_name.is_empty()
            && !state.config.enabled_agents.is_empty()
            && !state.config.enabled_agents.iter().any(|a| a == agent_name)
        {
            return Ok(Response::new(EnqueueResponse {
                result: Some(enqueue_response::Result::Error(
                    format!("agent '{agent_name}' is not in the enabled agents list"),
                )),
            }));
        }
        state.queue.push(task.clone());
        if let Err(e) = state.save_queue() {
            state.queue.pop(); // Roll back: must not dispatch a task that was not persisted
            return Err(Status::internal(format!("failed to persist queue: {e}")));
        }
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
        // Extract enabled_agents before taking the mutable task borrow.
        let enabled_agents = state.config.enabled_agents.clone();
        let task = state
            .queue
            .iter_mut()
            .find(|t| t.id == req.task_id)
            .ok_or_else(|| Status::not_found("task not found"))?;

        // Snapshot original values so we can roll back if the disk write fails.
        let old_agent = task.agent.clone();
        let old_priority = task.priority;
        let old_updated_at = task.updated_at;

        if let Some(agent) = req.agent {
            if !agent.is_empty() && !crate::gstack_agents::is_known(&agent) {
                return Ok(Response::new(UpdateTaskResponse {
                    result: Some(update_task_response::Result::Error(
                        format!("unknown agent '{agent}'; use GetAvailableAgents to list valid agents"),
                    )),
                }));
            }
            if !agent.is_empty()
                && !enabled_agents.is_empty()
                && !enabled_agents.iter().any(|a| a == &agent)
            {
                return Ok(Response::new(UpdateTaskResponse {
                    result: Some(update_task_response::Result::Error(
                        format!("agent '{agent}' is not in the enabled agents list"),
                    )),
                }));
            }
            task.agent = Some(agent);
        }
        if let Some(priority) = req.priority {
            if priority < 0 {
                return Ok(Response::new(UpdateTaskResponse {
                    result: Some(update_task_response::Result::Error(
                        "priority must be >= 0".to_string(),
                    )),
                }));
            }
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
        if let Err(e) = state.save_queue() {
            // Roll back in-memory mutations so the queue stays consistent with disk.
            if let Some(t) = state.queue.iter_mut().find(|t| t.id == task.id) {
                t.agent = old_agent;
                t.priority = old_priority;
                t.updated_at = old_updated_at;
            }
            return Err(Status::internal(format!("failed to persist queue: {e}")));
        }
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
        // Refuse to delete a task that a worker is actively executing.
        if let Some(task) = state.queue.iter().find(|t| t.id == req.task_id)
            && task.status == TaskStatus::Running as i32
        {
            return Err(Status::failed_precondition(
                "cannot remove a running task; wait for it to complete or restart the daemon to re-queue it",
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{
        DaemonConfig, GitHubIssueRef, IssueRefInput, JiraIssueRef, LinkRef,
        UpdateTaskRequest, RemoveTaskRequest, ListQueueRequest,
        issue_ref_input, list_queue_response, update_task_response, remove_task_response,
    };
    use crate::proto::queue_service_server::QueueService;

    fn make_state() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            config_path: "/tmp/queue-svc-test.toml".into(),
            queue_path: "/tmp/queue-svc-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec![] },
        }))
    }

    fn enqueue_req(r: issue_ref_input::Ref) -> Request<EnqueueRequest> {
        Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(r) }),
            agent: None,
            priority: None,
        })
    }

    fn assert_error(result: Option<enqueue_response::Result>, substr: &str) {
        match result.expect("no result") {
            enqueue_response::Result::Error(msg) => {
                assert!(msg.contains(substr), "expected '{substr}' in error: {msg}");
            }
            enqueue_response::Result::Task(_) => panic!("expected error, got task"),
        }
    }

    #[tokio::test]
    async fn enqueue_jira_ref_is_rejected() {
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Jira(JiraIssueRef { id: "PROJ-123".into() }))).await.unwrap().into_inner();
        assert_error(res.result, "Jira");
    }

    #[tokio::test]
    async fn enqueue_jira_link_is_rejected() {
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Link(LinkRef { url: "https://acme.atlassian.net/browse/PROJ-99".into() }))).await.unwrap().into_inner();
        assert_error(res.result, "Jira");
    }

    #[tokio::test]
    async fn enqueue_missing_issue_ref_is_rejected() {
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(EnqueueRequest { issue_ref: None, agent: None, priority: None });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        assert_error(res.result, "required");
    }

    #[tokio::test]
    async fn enqueue_github_ref_succeeds() {
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Github(GitHubIssueRef {
            organization: "acme".into(), repository: "app".into(), number: "42".into(),
        }))).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => {
                assert!(!t.id.is_empty());
                assert_eq!(t.status, TaskStatus::Queued as i32);
            }
            enqueue_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    async fn enqueue_github(svc: &QueueServiceImpl, number: &str) -> String {
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Github(GitHubIssueRef {
            organization: "acme".into(), repository: "app".into(), number: number.into(),
        }))).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => t.id,
            enqueue_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    #[tokio::test]
    async fn update_task_changes_agent_and_priority() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state);
        let task_id = enqueue_github(&svc, "42").await;

        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: Some("review".into()),
            priority: Some(10),
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        let updated = match res.result.unwrap() {
            update_task_response::Result::Task(t) => t,
            update_task_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(updated.id, task_id);
        assert_eq!(updated.agent, Some("review".into()));
        assert_eq!(updated.priority, 10);
    }

    #[tokio::test]
    async fn update_task_not_found_returns_error() {
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(UpdateTaskRequest {
            task_id: "nonexistent-id".into(),
            agent: None,
            priority: Some(5),
        });
        assert!(svc.update_task(req).await.is_err());
    }

    #[tokio::test]
    async fn remove_task_removes_from_queue() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "99").await;

        let req = Request::new(RemoveTaskRequest { task_id: task_id.clone() });
        let res = svc.remove_task(req).await.unwrap().into_inner();
        assert!(matches!(res.result.unwrap(), remove_task_response::Result::Ok(())));

        let locked = state.lock().await;
        assert!(!locked.queue.iter().any(|t| t.id == task_id));
    }

    #[tokio::test]
    async fn remove_task_not_found_returns_error() {
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(RemoveTaskRequest { task_id: "no-such-task".into() });
        assert!(svc.remove_task(req).await.is_err());
    }

    #[tokio::test]
    async fn enqueue_unknown_agent_is_rejected() {
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "1".into(),
            })) }),
            agent: Some("not-a-real-agent".into()),
            priority: None,
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        assert_error(res.result, "unknown agent");
    }

    #[tokio::test]
    async fn enqueue_known_agent_is_accepted() {
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "2".into(),
            })) }),
            agent: Some("review".into()),
            priority: None,
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => assert_eq!(t.agent, Some("review".into())),
            enqueue_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    #[tokio::test]
    async fn enqueue_disabled_agent_is_rejected() {
        let state = Arc::new(Mutex::new(AppState {
            config_path: "/tmp/queue-svc-disabled-test.toml".into(),
            queue_path: "/tmp/queue-svc-disabled-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec!["qa".into()] },
        }));
        let svc = QueueServiceImpl::new(state);
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "3".into(),
            })) }),
            agent: Some("review".into()), // valid gstack agent but not in enabled_agents
            priority: None,
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        assert_error(res.result, "not in the enabled agents list");
    }

    #[tokio::test]
    async fn remove_running_task_is_rejected() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "7").await;
        // Mark the task as RUNNING
        {
            let mut s = state.lock().await;
            s.queue.iter_mut().find(|t| t.id == task_id).unwrap().status = TaskStatus::Running as i32;
        }
        let req = Request::new(RemoveTaskRequest { task_id });
        assert!(svc.remove_task(req).await.is_err(), "should reject deletion of a running task");
    }

    #[tokio::test]
    async fn enqueue_negative_priority_is_rejected() {
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "1".into(),
            })) }),
            agent: None,
            priority: Some(-1),
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        assert_error(res.result, "priority must be >= 0");
    }

    #[tokio::test]
    async fn update_task_negative_priority_is_rejected() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state);
        let task_id = enqueue_github(&svc, "99").await;
        let req = Request::new(UpdateTaskRequest {
            task_id,
            agent: None,
            priority: Some(-5),
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Error(msg) => assert!(msg.contains("priority must be >= 0")),
            update_task_response::Result::Task(_) => panic!("expected error, got task"),
        }
    }

    #[tokio::test]
    async fn update_task_disabled_agent_is_rejected() {
        // update_task must honour enabled_agents just like enqueue does.
        let state = Arc::new(Mutex::new(AppState {
            config_path: "/tmp/queue-svc-disabled-update-test.toml".into(),
            queue_path: "/tmp/queue-svc-disabled-update-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec!["qa".into()] },
        }));
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "10").await;
        // "review" is a known agent but not in enabled_agents
        let req = Request::new(UpdateTaskRequest {
            task_id,
            agent: Some("review".into()),
            priority: None,
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Error(msg) => {
                assert!(msg.contains("not in the enabled agents list"), "got: {msg}");
            }
            update_task_response::Result::Task(_) => panic!("expected error, got task"),
        }
    }

    #[tokio::test]
    async fn update_task_unknown_agent_is_rejected() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state);
        let task_id = enqueue_github(&svc, "99").await;
        let req = Request::new(UpdateTaskRequest {
            task_id,
            agent: Some("bogus-skill".into()),
            priority: None,
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Error(msg) => assert!(msg.contains("unknown agent")),
            update_task_response::Result::Task(_) => panic!("expected error, got task"),
        }
    }

    #[tokio::test]
    async fn enqueue_rolls_back_queue_on_save_failure() {
        // Use a path that cannot be written to force save_queue to fail.
        let state = Arc::new(Mutex::new(AppState {
            config_path: "/tmp/enqueue-rollback-test.toml".into(),
            queue_path: "/nonexistent-dir/queue.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec![] },
        }));
        let svc = QueueServiceImpl::new(state.clone());
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "1".into(),
            })) }),
            agent: None,
            priority: None,
        });
        let res = svc.enqueue(req).await;
        // Must return a gRPC error because save failed
        assert!(res.is_err(), "expected Status::internal when save_queue fails");
        // Queue must be empty — the push must have been rolled back
        let locked = state.lock().await;
        assert!(locked.queue.is_empty(), "queue must be empty after rollback on save failure");
    }

    #[tokio::test]
    async fn update_task_rolls_back_on_save_failure() {
        // Enqueue with a writable path first, then swap to a path that cannot be written.
        let state = make_state();
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "99").await;

        // Set the original agent and priority so we have known values to check.
        {
            let mut s = state.lock().await;
            let t = s.queue.iter_mut().find(|t| t.id == task_id).unwrap();
            t.agent = Some("qa".into());
            t.priority = 5;
        }

        // Now break the queue_path so save_queue will fail on the next write.
        state.lock().await.queue_path = "/nonexistent-dir/queue.json".into();

        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: Some("review".into()),
            priority: Some(99),
        });
        let res = svc.update_task(req).await;
        // Must return a gRPC error because save failed
        assert!(res.is_err(), "expected Status::internal when save_queue fails");
        // In-memory task must still have the original values
        let locked = state.lock().await;
        let t = locked.queue.iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(t.agent, Some("qa".into()), "agent must be rolled back");
        assert_eq!(t.priority, 5, "priority must be rolled back");
    }

    #[tokio::test]
    async fn list_queue_returns_all_tasks() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state);
        enqueue_github(&svc, "1").await;
        enqueue_github(&svc, "2").await;

        let res = svc.list_queue(Request::new(ListQueueRequest {})).await.unwrap().into_inner();
        let tasks = match res.result.unwrap() {
            list_queue_response::Result::Ok(tl) => tl.tasks,
            list_queue_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(tasks.len(), 2);
    }
}
