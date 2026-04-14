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

    // Find highest-priority QUEUED task (higher priority value = pick first).
    // Tie-break by queue index so that equal-priority tasks dispatch in FIFO order
    // (lower index = enqueued earlier = picked first).  max_by_key alone would
    // return the *last* equal-priority element, which is counter-intuitive LIFO.
    let idx = s
        .queue
        .iter()
        .enumerate()
        .filter(|(_, t)| t.status == TaskStatus::Queued as i32)
        .max_by(|(i, a), (j, b)| {
            a.priority
                .cmp(&b.priority)
                .then(j.cmp(i)) // lower index wins on tie → FIFO
        })
        .map(|(i, _)| i)?;

    let task = &mut s.queue[idx];
    let task_id = task.id.clone();
    let agent = task.agent.clone();
    let issue_ref = task.issue_ref.clone();
    let old_updated_at = task.updated_at;
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

    if let Err(e) = s.save_queue() {
        // Roll back: revert task to QUEUED and remove the worker slot so the dispatch
        // loop retries cleanly on the next tick rather than dispatching a task whose
        // RUNNING state was never persisted (which would cause it to re-queue on restart
        // while the subprocess is still running — duplicate execution).
        if let Some(task) = s.queue.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Queued as i32;
            task.updated_at = old_updated_at;
        }
        s.workers.retain(|w| w.worker_id != worker_id);
        eprintln!(
            "[worker_pool] save_queue failed after marking task {task_id} RUNNING: {e}; rolled back to QUEUED"
        );
        return None;
    }
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
/// - Centy  `centy:<n>`    — `worktree open` accepts `centy:<display_number>`
/// - Jira   — not directly supported by `worktree open`; returns None
///
/// Centy refs whose `number` field contains a UUID (stored by older code or via direct
/// API enqueue) are resolved to the integer display number via the centy CLI before
/// building the ref string.  This is the definitive last-resort fix: it fires regardless
/// of how the UUID reached the queue (URL link, direct CentyIssueRef, persisted state).
async fn to_worktree_ref(r: &IssueRef) -> Option<String> {
    match r.r#ref.as_ref()? {
        crate::proto::issue_ref::Ref::Github(g) => {
            Some(format!("{}/{}#{}", g.organization, g.repository, g.number))
        }
        crate::proto::issue_ref::Ref::Centy(c) => {
            let number = if crate::centy_resolver::is_uuid(&c.number) {
                match crate::centy_resolver::resolve_centy_uuid(&c.number).await {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!(
                            "[worker_pool] failed to resolve Centy UUID {}: {e}",
                            c.number
                        );
                        return None;
                    }
                }
            } else {
                c.number.clone()
            };
            Some(format!("centy:{number}"))
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
    let Some(ref_str) = to_worktree_ref(r).await else {
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
    // Update in-memory state first, then release the lock so the daemon stays
    // responsive while we retry the persist below.
    {
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
        } else {
            eprintln!(
                "[worker_pool] finish_task: task {task_id} not found in queue (removed while running?); worker slot freed"
            );
        }
        s.workers.retain(|w| w.worker_id != worker_id);
    }

    // Retry persist with short exponential backoff. Releasing the lock between
    // attempts keeps the daemon responsive. If all attempts fail, the in-memory
    // state is still correct; the risk is that a crash before another mutation
    // triggers a successful save would cause the task to re-execute on restart.
    for delay_ms in [0u64, 10, 30, 100] {
        if delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
        let mut s = state.lock().await;
        match s.save_queue() {
            Ok(()) => return,
            Err(e) => eprintln!(
                "[worker_pool] save_queue failed after finishing task {task_id}: {e}{}",
                if delay_ms == 100 {
                    "; giving up — task may re-execute on daemon restart"
                } else {
                    "; retrying"
                }
            ),
        }
    }
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
        // Unique path per call so parallel tests cannot corrupt each other's queue file.
        let id = uuid::Uuid::new_v4();
        Arc::new(Mutex::new(AppState {
            config_path: format!("/tmp/worker-pool-test-{id}.toml"),
            queue_path: format!("/tmp/worker-pool-test-{id}.json"),
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

    #[tokio::test]
    async fn worktree_ref_github() {
        assert_eq!(
            to_worktree_ref(&github_ref("acme", "my-repo", "42")).await,
            Some("acme/my-repo#42".into())
        );
    }

    #[tokio::test]
    async fn worktree_ref_centy() {
        assert_eq!(to_worktree_ref(&centy_ref("7")).await, Some("centy:7".into()));
    }

    #[tokio::test]
    async fn worktree_ref_jira_returns_none() {
        assert_eq!(to_worktree_ref(&jira_ref("PROJ-123")).await, None);
    }

    #[tokio::test]
    async fn worktree_ref_empty_issue_ref_returns_none() {
        let empty = IssueRef { r#ref: None };
        assert_eq!(to_worktree_ref(&empty).await, None);
    }

    /// UUID-valued Centy number must either resolve to a valid `centy:<int>` ref or
    /// return None (when the centy CLI isn't available in the test environment).
    /// It must never produce `centy:<uuid>` — that is the original bug.
    #[tokio::test]
    async fn worktree_ref_centy_uuid_never_passes_uuid_to_worktree() {
        let result = to_worktree_ref(&centy_ref("6f4853a9-3d82-4013-b909-c2d637f44541")).await;
        if let Some(ref_str) = result {
            assert!(
                !ref_str.contains("6f4853a9-3d82-4013-b909-c2d637f44541"),
                "UUID must not reach worktree open, got: {ref_str}"
            );
            let n = ref_str.strip_prefix("centy:").expect("must start with centy:");
            assert!(n.parse::<u32>().is_ok(), "resolved part must be a positive integer, got: {n}");
        }
        // None is also valid (resolution failed because centy isn't available)
    }

    // --- pick_next_task tests ---

    #[tokio::test]
    async fn pick_dispatches_fifo_for_equal_priority_tasks() {
        // All tasks have the same priority — the first-enqueued one must be picked first.
        // max_by_key alone would return the *last* element in case of ties, which is LIFO.
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            s.queue.push(queued_task("first", 0));
            s.queue.push(queued_task("second", 0));
            s.queue.push(queued_task("third", 0));
        }
        let (task_id, _, _, _) = pick_next_task(&state).await.expect("should pick a task");
        assert_eq!(task_id, "first", "equal-priority tasks must be dispatched in FIFO order");
    }

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
    async fn pick_rolls_back_to_queued_on_save_failure() {
        // Use a queue_path that cannot be written so save_queue fails.
        let state = Arc::new(Mutex::new(AppState {
            config_path: "/tmp/pick-rollback-test.toml".into(),
            queue_path: "/nonexistent-dir/queue.json".into(),
            queue: vec![queued_task("t1", 5)],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec![] },
        }));
        let result = pick_next_task(&state).await;
        // Must return None — not safe to dispatch when persist failed
        assert!(result.is_none(), "should return None when save_queue fails");
        let s = state.lock().await;
        // Task must be rolled back to QUEUED
        let task = s.queue.iter().find(|t| t.id == "t1").expect("task must still exist");
        assert_eq!(task.status, TaskStatus::Queued as i32, "task must be rolled back to QUEUED");
        // Worker slot must not be leaked
        assert!(s.workers.is_empty(), "worker slot must not be registered when save fails");
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

    #[tokio::test]
    async fn finish_task_handles_save_failure_gracefully() {
        // Even when save_queue always fails, the in-memory task status must be
        // set to COMPLETED/FAILED and the worker slot must be freed so capacity
        // is not permanently lost.
        let state = Arc::new(Mutex::new(AppState {
            config_path: "/tmp/finish-task-fail-test.toml".into(),
            queue_path: "/nonexistent-dir/queue.json".into(),
            queue: vec![{
                let mut t = queued_task("t1", 0);
                t.status = TaskStatus::Running as i32;
                t
            }],
            workers: vec![WorkerInfo {
                worker_id: "w1".into(),
                status: WorkerStatus::Busy as i32,
                current_task_id: "t1".into(),
                current_agent: String::new(),
                task_started_at: None,
            }],
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec![] },
        }));
        finish_task(&state, "t1", "w1", true).await;
        let s = state.lock().await;
        let task = s.queue.iter().find(|t| t.id == "t1").expect("task must exist");
        assert_eq!(task.status, TaskStatus::Completed as i32, "task must be COMPLETED in memory");
        assert!(s.workers.is_empty(), "worker slot must be freed even when save fails");
    }

    // --- execute_agent tests ---

    #[tokio::test]
    async fn execute_agent_returns_false_when_no_issue_ref() {
        // First branch: issue_ref is None → immediate false without spawning anything.
        let result = execute_agent(None, None).await;
        assert!(!result, "execute_agent must return false when there is no issue_ref");
    }

    #[tokio::test]
    async fn execute_agent_returns_false_for_jira_ref() {
        // Jira refs are not supported by worktree-io (to_worktree_ref returns None),
        // so execute_agent must return false without attempting to spawn a process.
        let result = execute_agent(Some(jira_ref("PROJ-42")), None).await;
        assert!(!result, "execute_agent must return false for Jira refs");
    }

    #[tokio::test]
    async fn execute_agent_exercises_spawn_path_for_github_ref() {
        // Exercises lines that no other test reaches: the `match cmd.status().await`
        // arms (both Ok and Err), the `cmd.env("TEAM_AGENT", a)` branch, and the
        // `eprintln!` on non-zero exit / spawn failure.
        //
        // The result depends on the environment:
        //   - `worktree` not in PATH → Err(spawn failed) → false
        //   - `worktree` in PATH but `open` fails → Ok(non-zero) → false
        // In both cases execute_agent must return false without panicking.
        let result = execute_agent(
            Some(github_ref("team-ci-nonexistent", "repo", "99999")),
            Some("review".into()),
        ).await;
        assert!(!result, "execute_agent must return false when worktree is unavailable or open fails");
    }

    #[tokio::test]
    async fn finish_task_frees_worker_even_when_task_already_removed() {
        // Simulate a task that was deleted while it was running (admin forced removal).
        // The worker slot must still be freed so capacity is not permanently lost.
        let state = make_state(4);
        {
            let mut s = state.lock().await;
            // No task in queue — already removed
            s.workers.push(WorkerInfo {
                worker_id: "w1".into(),
                status: WorkerStatus::Busy as i32,
                current_task_id: "ghost-task".into(),
                current_agent: String::new(),
                task_started_at: None,
            });
        }
        finish_task(&state, "ghost-task", "w1", true).await;
        let s = state.lock().await;
        assert!(s.workers.is_empty(), "worker slot must be freed even when task is missing");
    }
}
