use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::proto::{IssueRef, TaskStatus, WorkerInfo, WorkerStatus};
use crate::state::AppState;

pub fn start(state: Arc<Mutex<AppState>>) {
    tokio::spawn(dispatch_loop(state));
}

async fn dispatch_loop(state: Arc<Mutex<AppState>>) {
    loop {
        match pick_next_task(&state).await {
            Some((task_id, issue_ref, worker_id, agent)) => {
                let state_clone = state.clone();
                tokio::spawn(run_task(state_clone, task_id, issue_ref, worker_id, agent));
                // Yield to let the spawned task run before checking for the next one.
                tokio::task::yield_now().await;
            }
            None => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

/// Finds the highest-priority QUEUED task that fits within the worker concurrency limit,
/// atomically marks it RUNNING, registers the worker, and returns
/// (task_id, issue_ref, worker_id, agent).
async fn pick_next_task(
    state: &Arc<Mutex<AppState>>,
) -> Option<(String, Option<IssueRef>, String, Option<String>)> {
    let mut s = state.lock().await;
    let running = s.workers.len() as i32;
    if running >= s.config.workers_count {
        return None;
    }

    // Find highest-priority QUEUED task (higher priority value = pick first)
    let idx = s
        .queue
        .iter()
        .enumerate()
        .filter(|(_, t)| t.status == TaskStatus::Queued as i32)
        .max_by_key(|(_, t)| t.priority)
        .map(|(i, _)| i)?;

    let task = &mut s.queue[idx];
    let task_id = task.id.clone();
    let agent = task.agent.clone();
    let issue_ref = task.issue_ref.clone();
    task.status = TaskStatus::Running as i32;
    task.updated_at = Some(prost_types::Timestamp {
        seconds: now_seconds(),
        nanos: 0,
    });

    let worker_id = uuid::Uuid::new_v4().to_string();
    s.workers.push(WorkerInfo {
        worker_id: worker_id.clone(),
        status: WorkerStatus::Busy as i32,
        current_task_id: task_id.clone(),
        current_agent: agent.clone().unwrap_or_default(),
        task_started_at: Some(prost_types::Timestamp {
            seconds: now_seconds(),
            nanos: 0,
        }),
    });

    let _ = s.save_queue();
    Some((task_id, issue_ref, worker_id, agent))
}

async fn run_task(
    state: Arc<Mutex<AppState>>,
    task_id: String,
    issue_ref: Option<IssueRef>,
    worker_id: String,
    agent: Option<String>,
) {
    let success = execute_agent(issue_ref, agent).await;
    finish_task(&state, &task_id, &worker_id, success).await;
}

/// Converts an internal IssueRef to the ref format accepted by `worktree open`.
///
/// Supported mappings:
/// - GitHub `org/repo#42`  — `worktree open` accepts `owner/repo#number`
/// - Centy  `centy:<id>`   — `worktree open` accepts `centy:<number>`
/// - Jira   — not directly supported by `worktree open`; returns None
fn to_worktree_ref(r: &IssueRef) -> Option<String> {
    match r.r#ref.as_ref()? {
        crate::proto::issue_ref::Ref::Github(g) => {
            Some(format!("{}/{}#{}", g.organization, g.repository, g.number))
        }
        crate::proto::issue_ref::Ref::Centy(c) => {
            Some(format!("centy:{}", c.number))
        }
        crate::proto::issue_ref::Ref::Jira(j) => {
            eprintln!("[worker_pool] Jira refs are not supported by worktree-io (task will fail): {}", j.id);
            None
        }
    }
}

/// Delegates execution to worktree-io: `worktree open <ref>`.
///
/// Sets `TEAM_AGENT` in the child process environment so that worktree-io
/// hooks can route to the correct gstack skill (e.g. `review`, `qa`, `ship`).
///
/// Returns false if the issue ref cannot be converted, or the process exits non-zero.
async fn execute_agent(issue_ref: Option<IssueRef>, agent: Option<String>) -> bool {
    let Some(ref r) = issue_ref else {
        eprintln!("[worker_pool] task has no issue_ref — marking failed");
        return false;
    };
    let Some(ref_str) = to_worktree_ref(r) else {
        return false;
    };

    let mut cmd = tokio::process::Command::new("worktree");
    cmd.arg("open").arg(&ref_str);

    // Expose the chosen agent/skill to the worktree hooks via TEAM_AGENT.
    if let Some(a) = agent.filter(|a| !a.is_empty()) {
        cmd.env("TEAM_AGENT", a);
    }

    match cmd.status().await {
        Ok(status) => {
            if !status.success() {
                eprintln!(
                    "[worker_pool] `worktree open {}` exited with {}",
                    ref_str,
                    status.code().unwrap_or(-1)
                );
            }
            status.success()
        }
        Err(e) => {
            eprintln!("[worker_pool] failed to spawn `worktree open {}`: {e}", ref_str);
            false
        }
    }
}

async fn finish_task(
    state: &Arc<Mutex<AppState>>,
    task_id: &str,
    worker_id: &str,
    success: bool,
) {
    let mut s = state.lock().await;
    let final_status = if success {
        TaskStatus::Completed as i32
    } else {
        TaskStatus::Failed as i32
    };

    if let Some(task) = s.queue.iter_mut().find(|t| t.id == task_id) {
        task.status = final_status;
        task.updated_at = Some(prost_types::Timestamp {
            seconds: now_seconds(),
            nanos: 0,
        });
    }

    s.workers.retain(|w| w.worker_id != worker_id);
    let _ = s.save_queue();
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{issue_ref, CentyIssueRef, DaemonConfig, GitHubIssueRef, JiraIssueRef, Task, TaskStatus};
    use crate::state::AppState;

    fn make_state(workers_count: i32) -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            config_path: "/tmp/worker-pool-test.toml".into(),
            queue_path: "/tmp/worker-pool-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count, log_level: "info".into(), enabled_agents: vec![] },
        }))
    }

    fn queued_task(id: &str, priority: i32) -> Task {
        Task {
            id: id.into(),
            issue_ref: None,
            agent: None,
            status: TaskStatus::Queued as i32,
            priority,
            created_at: None,
            updated_at: None,
        }
    }

    fn github_ref(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        }))}
    }
    fn centy_ref(number: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
            organization: "acme".into(), repository: "proj".into(), number: number.into(),
        }))}
    }
    fn jira_ref(id: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Jira(JiraIssueRef { id: id.into() })) }
    }

    #[test]
    fn worktree_ref_github() {
        assert_eq!(to_worktree_ref(&github_ref("acme", "my-repo", "42")), Some("acme/my-repo#42".into()));
    }

    #[test]
    fn worktree_ref_centy() {
        assert_eq!(to_worktree_ref(&centy_ref("7")), Some("centy:7".into()));
    }

    #[test]
    fn worktree_ref_jira_returns_none() {
        // Jira is not supported by worktree-io
        assert_eq!(to_worktree_ref(&jira_ref("PROJ-123")), None);
    }

    #[test]
    fn worktree_ref_empty_issue_ref_returns_none() {
        let empty = IssueRef { r#ref: None };
        assert_eq!(to_worktree_ref(&empty), None);
    }

    // --- pick_next_task tests ---

    #[tokio::test]
    async fn pick_returns_highest_priority_queued_task() {
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            s.queue.push(queued_task("low", 1));
            s.queue.push(queued_task("high", 10));
            s.queue.push(queued_task("mid", 5));
        }
        let result = pick_next_task(&state).await;
        let (task_id, _, _, _) = result.expect("should pick a task");
        assert_eq!(task_id, "high");
    }

    #[tokio::test]
    async fn pick_sets_task_to_running() {
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            s.queue.push(queued_task("t1", 5));
        }
        let (task_id, _, _, _) = pick_next_task(&state).await.expect("should pick");
        let s = state.lock().await;
        let task = s.queue.iter().find(|t| t.id == task_id).expect("task must exist");
        assert_eq!(task.status, TaskStatus::Running as i32);
    }

    #[tokio::test]
    async fn pick_registers_worker() {
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            s.queue.push(queued_task("t1", 5));
        }
        pick_next_task(&state).await.expect("should pick");
        let s = state.lock().await;
        assert_eq!(s.workers.len(), 1);
        assert_eq!(s.workers[0].current_task_id, "t1");
    }

    #[tokio::test]
    async fn pick_respects_workers_count_limit() {
        let state = make_state(1);
        {
            let mut s = state.lock().await;
            s.queue.push(queued_task("t1", 5));
            s.queue.push(queued_task("t2", 3));
            // Simulate one worker already running
            s.workers.push(WorkerInfo {
                worker_id: "existing".into(),
                status: WorkerStatus::Busy as i32,
                current_task_id: "t1".into(),
                current_agent: String::new(),
                task_started_at: None,
            });
        }
        let result = pick_next_task(&state).await;
        assert!(result.is_none(), "should not pick when at capacity");
    }

    #[tokio::test]
    async fn pick_returns_none_when_no_queued_tasks() {
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            // Only a RUNNING task, no QUEUED
            let mut t = queued_task("t1", 5);
            t.status = TaskStatus::Running as i32;
            s.queue.push(t);
        }
        let result = pick_next_task(&state).await;
        assert!(result.is_none(), "should not pick a non-queued task");
    }

    #[tokio::test]
    async fn pick_returns_none_for_empty_queue() {
        let state = make_state(4);
        let result = pick_next_task(&state).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn finish_task_sets_completed_on_success() {
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            let mut t = queued_task("t1", 0);
            t.status = TaskStatus::Running as i32;
            s.queue.push(t);
            s.workers.push(WorkerInfo {
                worker_id: "w1".into(),
                status: WorkerStatus::Busy as i32,
                current_task_id: "t1".into(),
                current_agent: String::new(),
                task_started_at: None,
            });
        }
        finish_task(&state, "t1", "w1", true).await;
        let s = state.lock().await;
        let task = s.queue.iter().find(|t| t.id == "t1").expect("task must exist");
        assert_eq!(task.status, TaskStatus::Completed as i32);
        assert!(s.workers.is_empty(), "worker slot must be freed");
    }

    #[tokio::test]
    async fn finish_task_sets_failed_on_failure() {
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            let mut t = queued_task("t1", 0);
            t.status = TaskStatus::Running as i32;
            s.queue.push(t);
            s.workers.push(WorkerInfo {
                worker_id: "w1".into(),
                status: WorkerStatus::Busy as i32,
                current_task_id: "t1".into(),
                current_agent: String::new(),
                task_started_at: None,
            });
        }
        finish_task(&state, "t1", "w1", false).await;
        let s = state.lock().await;
        let task = s.queue.iter().find(|t| t.id == "t1").expect("task must exist");
        assert_eq!(task.status, TaskStatus::Failed as i32);
    }
}
