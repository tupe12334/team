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

        // --- Validate all inputs before making any mutations ---
        // This prevents partial in-memory state updates when a later validation fails.
        if let Some(ref agent) = req.agent {
            if !agent.is_empty() && !crate::gstack_agents::is_known(agent) {
                return Ok(Response::new(UpdateTaskResponse {
                    result: Some(update_task_response::Result::Error(
                        format!("unknown agent '{agent}'; use GetAvailableAgents to list valid agents"),
                    )),
                }));
            }
            if !agent.is_empty()
                && !enabled_agents.is_empty()
                && !enabled_agents.iter().any(|a| a == agent)
            {
                return Ok(Response::new(UpdateTaskResponse {
                    result: Some(update_task_response::Result::Error(
                        format!("agent '{agent}' is not in the enabled agents list"),
                    )),
                }));
            }
        }
        if req.priority.is_some_and(|p| p < 0) {
            return Ok(Response::new(UpdateTaskResponse {
                result: Some(update_task_response::Result::Error(
                    "priority must be >= 0".to_string(),
                )),
            }));
        }

        let task = state
            .queue
            .iter_mut()
            .find(|t| t.id == req.task_id)
            .ok_or_else(|| Status::not_found("task not found"))?;

        // Snapshot original values so we can roll back if the disk write fails.
        let old_agent = task.agent.clone();
        let old_priority = task.priority;
        let old_updated_at = task.updated_at;

        // All inputs are valid — apply mutations.
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
        let removed_task = state.queue.iter().find(|t| t.id == req.task_id).cloned();
        let before = state.queue.len();
        state.queue.retain(|t| t.id != req.task_id);
        if state.queue.len() == before {
            return Err(Status::not_found("task not found"));
        }
        if let Err(e) = state.save_queue() {
            // Roll back: task must not disappear from memory when the disk write failed.
            if let Some(task) = removed_task {
                state.queue.push(task);
            }
            return Err(Status::internal(format!("failed to persist queue: {e}")));
        }
        Ok(Response::new(RemoveTaskResponse {
            result: Some(remove_task_response::Result::Ok(())),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{
        CentyIssueRef, DaemonConfig, GitHubIssueRef, IssueRefInput, JiraIssueRef, LinkRef,
        UpdateTaskRequest, RemoveTaskRequest, ListQueueRequest,
        issue_ref, issue_ref_input, list_queue_response, update_task_response, remove_task_response,
    };
    use crate::proto::queue_service_server::QueueService;

    fn make_state() -> Arc<Mutex<AppState>> {
        // Unique path per call so parallel tests cannot corrupt each other's queue file.
        let id = uuid::Uuid::new_v4();
        Arc::new(Mutex::new(AppState {
            config_path: format!("/tmp/queue-svc-test-{id}.toml"),
            queue_path: format!("/tmp/queue-svc-test-{id}.json"),
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
    async fn enqueue_issue_ref_with_no_inner_ref_is_rejected() {
        // `issue_ref` is present but its oneof `ref` field is None (the client sent an
        // empty IssueRefInput without setting any variant).
        // `req.issue_ref.and_then(|r| r.r#ref)` yields None via the inner None path —
        // a different code path to the same "issue_ref is required" arm than when
        // `issue_ref` itself is None (outer None, covered by enqueue_missing_issue_ref_is_rejected).
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: None }),
            agent: None,
            priority: None,
        });
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

    #[tokio::test]
    async fn enqueue_github_link_succeeds() {
        // A GitHub URL is resolved synchronously by the link resolver — no network needed.
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Link(LinkRef {
            url: "https://github.com/acme/app/issues/99".into(),
        }))).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => {
                assert!(!t.id.is_empty());
                assert_eq!(t.status, TaskStatus::Queued as i32);
                match t.issue_ref.unwrap().r#ref.unwrap() {
                    issue_ref::Ref::Github(g) => {
                        assert_eq!(g.organization, "acme");
                        assert_eq!(g.repository, "app");
                        assert_eq!(g.number, "99");
                    }
                    other => panic!("expected Github ref from link resolver, got {:?}", other),
                }
            }
            enqueue_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    #[tokio::test]
    async fn enqueue_centy_link_succeeds() {
        // A Centy URL with a plain display number resolves synchronously.
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Link(LinkRef {
            url: "https://app.centy.io/acme/proj/issues/5".into(),
        }))).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => {
                assert!(!t.id.is_empty());
                assert_eq!(t.status, TaskStatus::Queued as i32);
                match t.issue_ref.unwrap().r#ref.unwrap() {
                    issue_ref::Ref::Centy(c) => {
                        assert_eq!(c.organization, "acme");
                        assert_eq!(c.repository, "proj");
                        assert_eq!(c.number, "5");
                    }
                    other => panic!("expected Centy ref from link resolver, got {:?}", other),
                }
            }
            enqueue_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    #[tokio::test]
    async fn enqueue_unresolvable_link_returns_error() {
        // An unknown provider URL fails the link resolver and surfaces as an error response.
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Link(LinkRef {
            url: "https://gitlab.com/acme/app/-/issues/1".into(),
        }))).await.unwrap().into_inner();
        assert_error(res.result, "cannot resolve link");
    }

    #[tokio::test]
    async fn enqueue_centy_ref_succeeds() {
        let svc = QueueServiceImpl::new(make_state());
        let res = svc.enqueue(enqueue_req(issue_ref_input::Ref::Centy(CentyIssueRef {
            organization: "acme".into(), repository: "proj".into(), number: "7".into(),
        }))).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => {
                assert!(!t.id.is_empty());
                assert_eq!(t.status, TaskStatus::Queued as i32);
                match t.issue_ref.unwrap().r#ref.unwrap() {
                    issue_ref::Ref::Centy(c) => {
                        assert_eq!(c.organization, "acme");
                        assert_eq!(c.repository, "proj");
                        assert_eq!(c.number, "7");
                    }
                    other => panic!("expected Centy ref, got {:?}", other),
                }
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
    async fn remove_task_rolls_back_on_save_failure() {
        // Enqueue with a writable path, then point to a path that cannot be written.
        let state = make_state();
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "55").await;
        state.lock().await.queue_path = "/nonexistent-dir/queue.json".into();

        let req = Request::new(RemoveTaskRequest { task_id: task_id.clone() });
        let res = svc.remove_task(req).await;
        assert!(res.is_err(), "expected Status::internal when save_queue fails");
        // Task must still be present in memory after rollback
        let locked = state.lock().await;
        assert!(
            locked.queue.iter().any(|t| t.id == task_id),
            "task must be restored in memory after rollback"
        );
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

    /// Regression: update_task used to mutate task.agent before validating priority,
    /// leaving in-memory state inconsistent with disk when priority was invalid.
    /// Both fields must remain unchanged when any validation fails.
    #[tokio::test]
    async fn update_task_no_in_memory_mutation_when_priority_invalid() {
        let state = make_state();
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "42").await;

        // Confirm initial state: agent is None
        {
            let s = state.lock().await;
            let t = s.queue.iter().find(|t| t.id == task_id).unwrap();
            assert!(t.agent.is_none(), "precondition: task has no agent");
        }

        // Send a valid agent but an invalid (negative) priority
        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: Some("review".into()),
            priority: Some(-99),
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Error(msg) => {
                assert!(msg.contains("priority must be >= 0"), "got: {msg}");
            }
            update_task_response::Result::Task(_) => panic!("expected error, got task"),
        }

        // In-memory task must be completely unchanged — the failed validation
        // must not have partially applied the agent mutation.
        let s = state.lock().await;
        let t = s.queue.iter().find(|t| t.id == task_id).unwrap();
        assert!(t.agent.is_none(), "agent must not have been mutated when priority validation failed");
        assert_eq!(t.priority, 0, "priority must not have changed");
    }

    #[tokio::test]
    async fn update_task_zero_priority_is_accepted() {
        // priority = 0 is the minimum valid value; must not be rejected.
        let state = make_state();
        let svc = QueueServiceImpl::new(state);
        let task_id = enqueue_github(&svc, "10").await;
        let req = Request::new(UpdateTaskRequest {
            task_id,
            agent: None,
            priority: Some(0),
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Task(t) => assert_eq!(t.priority, 0),
            update_task_response::Result::Error(e) => panic!("expected ok for priority=0, got: {e}"),
        }
    }

    #[tokio::test]
    async fn enqueue_zero_priority_is_accepted() {
        // priority=0 is explicitly provided and is the minimum valid value.
        // This differs from None (which also defaults to 0) — Some(0) exercises the
        // `req.priority.is_some_and(|p| p < 0)` guard boundary directly.
        let svc = QueueServiceImpl::new(make_state());
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "1".into(),
            })) }),
            agent: None,
            priority: Some(0),
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => assert_eq!(t.priority, 0),
            enqueue_response::Result::Error(e) => panic!("expected ok for priority=0, got: {e}"),
        }
    }

    #[tokio::test]
    async fn update_task_empty_agent_string_sets_agent_without_validation() {
        // agent="" skips the is_known check (the guard is `if !agent.is_empty() && ...`)
        // and stores Some("") on the task, which execute_agent treats as "no agent".
        let state = make_state();
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "55").await;

        // First set a non-empty agent so we can verify "" clears it
        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: Some("review".into()),
            priority: None,
        });
        svc.update_task(req).await.unwrap();

        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: Some("".into()),
            priority: None,
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Task(t) => {
                assert_eq!(t.agent, Some("".into()), "empty agent string must be stored as Some(\"\")");
            }
            update_task_response::Result::Error(e) => panic!("expected ok for empty agent, got: {e}"),
        }
    }

    /// When `agent = Some("")` and `enabled_agents` is non-empty, the `!agent.is_empty()`
    /// guard short-circuits the enabled_agents check in update_task — just as in enqueue.
    /// `update_task_empty_agent_string_sets_agent_without_validation` uses `make_state()`
    /// (enabled_agents empty), so there the second `&&` short-circuits instead.  This test
    /// isolates the first `&&` specifically: non-empty enabled list + empty agent string.
    #[tokio::test]
    async fn update_task_empty_agent_bypasses_enabled_agents_check() {
        let id = uuid::Uuid::new_v4();
        let state = Arc::new(Mutex::new(AppState {
            config_path: format!("/tmp/update-empty-agent-{id}.toml"),
            queue_path: format!("/tmp/update-empty-agent-{id}.json"),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec!["qa".into()] },
        }));
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "77").await;
        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: Some("".into()), // empty → !is_empty() = false → enabled_agents check skipped
            priority: None,
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Task(t) => {
                assert_eq!(t.agent, Some("".into()), "empty agent must be stored even when enabled_agents is configured");
            }
            update_task_response::Result::Error(e) => {
                panic!("empty agent must bypass enabled_agents check, got: {e}");
            }
        }
    }

    /// When enabled_agents is configured and the request has no agent (agent=None),
    /// the `if let Some(ref agent_name) = task.agent` guard short-circuits immediately
    /// and the task is accepted — not setting an agent is always allowed regardless of
    /// what the enabled list contains.
    #[tokio::test]
    async fn enqueue_task_without_agent_accepted_when_enabled_agents_configured() {
        let state = Arc::new(Mutex::new(AppState {
            config_path: format!("/tmp/enqueue-no-agent-{}.toml", uuid::Uuid::new_v4()),
            queue_path: format!("/tmp/enqueue-no-agent-{}.json", uuid::Uuid::new_v4()),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec!["qa".into()] },
        }));
        let svc = QueueServiceImpl::new(state);
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "99".into(),
            })) }),
            agent: None, // no agent — must not be rejected by the enabled_agents check
            priority: None,
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => {
                assert_eq!(t.agent, None, "task must have no agent when none was requested");
                assert_eq!(t.status, TaskStatus::Queued as i32);
            }
            enqueue_response::Result::Error(e) => {
                panic!("expected task to be accepted when agent is None, got error: {e}");
            }
        }
    }

    /// When enabled_agents is configured and update_task is called with agent=None
    /// (meaning "don't change the current agent"), the outer `if let Some(ref agent)`
    /// guard short-circuits — no validation is run and the task is updated successfully.
    #[tokio::test]
    async fn update_task_without_agent_accepted_when_enabled_agents_configured() {
        let state = Arc::new(Mutex::new(AppState {
            config_path: format!("/tmp/update-no-agent-{}.toml", uuid::Uuid::new_v4()),
            queue_path: format!("/tmp/update-no-agent-{}.json", uuid::Uuid::new_v4()),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec!["qa".into()] },
        }));
        let svc = QueueServiceImpl::new(state.clone());
        let task_id = enqueue_github(&svc, "42").await;
        // agent=None: caller is only changing priority, not the agent field.
        // This must not be rejected by the enabled_agents guard.
        let req = Request::new(UpdateTaskRequest {
            task_id: task_id.clone(),
            agent: None,
            priority: Some(5),
        });
        let res = svc.update_task(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_task_response::Result::Task(t) => {
                assert_eq!(t.priority, 5, "priority must be updated");
                // agent was not set in the original enqueue_github call → still None
                assert_eq!(t.agent, None, "agent must remain unchanged when not specified");
            }
            update_task_response::Result::Error(e) => {
                panic!("expected task to be updated when agent is None, got error: {e}");
            }
        }
    }

    /// When `agent = Some("")` and `enabled_agents` is configured, the
    /// `!agent_name.is_empty()` guard in the enabled_agents check short-circuits to false
    /// — skipping the check entirely — so the task is accepted.  This is a distinct code
    /// path from both `agent: None` (outer `if let Some` fails) and `agent: Some("review")`
    /// with a non-matching enabled list (all three `&&` conditions are true → rejected).
    /// The update_task variant is covered by `update_task_empty_agent_string_sets_agent_without_validation`;
    /// this test covers the parallel branch in the enqueue handler.
    #[tokio::test]
    async fn enqueue_empty_agent_string_bypasses_enabled_agents_check() {
        let id = uuid::Uuid::new_v4();
        let state = Arc::new(Mutex::new(AppState {
            config_path: format!("/tmp/enqueue-empty-agent-{id}.toml"),
            queue_path: format!("/tmp/enqueue-empty-agent-{id}.json"),
            queue: Vec::new(),
            workers: Vec::new(),
            // "qa" is the only enabled agent — "" is not in this list but must not be rejected.
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec!["qa".into()] },
        }));
        let svc = QueueServiceImpl::new(state);
        let req = Request::new(EnqueueRequest {
            issue_ref: Some(IssueRefInput { r#ref: Some(issue_ref_input::Ref::Github(GitHubIssueRef {
                organization: "acme".into(), repository: "app".into(), number: "7".into(),
            })) }),
            agent: Some("".into()), // empty string — !is_empty() = false → enabled_agents check skipped
            priority: None,
        });
        let res = svc.enqueue(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            enqueue_response::Result::Task(t) => {
                assert_eq!(t.agent, Some("".into()), "empty agent must be stored as-is");
                assert_eq!(t.status, TaskStatus::Queued as i32, "task must be queued");
            }
            enqueue_response::Result::Error(e) => {
                panic!("empty agent with enabled_agents configured must not be rejected, got: {e}");
            }
        }
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
