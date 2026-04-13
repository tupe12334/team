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
            Some((task_id, issue_ref, worker_id)) => {
                let state_clone = state.clone();
                tokio::spawn(run_task(state_clone, task_id, issue_ref, worker_id));
            }
            None => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

/// Finds the highest-priority QUEUED task that fits within the worker concurrency limit,
/// atomically marks it RUNNING, registers the worker, and returns (task_id, issue_ref, worker_id).
async fn pick_next_task(
    state: &Arc<Mutex<AppState>>,
) -> Option<(String, Option<IssueRef>, String)> {
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
        current_agent: agent.unwrap_or_default(),
        task_started_at: Some(prost_types::Timestamp {
            seconds: now_seconds(),
            nanos: 0,
        }),
    });

    let _ = s.save_queue();
    Some((task_id, issue_ref, worker_id))
}

async fn run_task(
    state: Arc<Mutex<AppState>>,
    task_id: String,
    issue_ref: Option<IssueRef>,
    worker_id: String,
) {
    let success = execute_agent(issue_ref).await;
    finish_task(&state, &task_id, &worker_id, success).await;
}

/// Converts an internal IssueRef to the ref format accepted by `worktree open`.
///
/// Supported mappings:
/// - GitHub `org/repo#42`  — `worktree open` accepts `owner/repo#number`
/// - Centy  `centy:<id>`   — `worktree open` accepts `centy:<number>`
/// - Jira   — not directly supported by `worktree open`; returns None
fn to_worktree_ref(r: IssueRef) -> Option<String> {
    match r.r#ref? {
        crate::proto::issue_ref::Ref::Github(g) => {
            Some(format!("{}/{}#{}", g.organization, g.repository, g.number))
        }
        crate::proto::issue_ref::Ref::Centy(c) => {
            Some(format!("centy:{}", c.number))
        }
        crate::proto::issue_ref::Ref::Jira(_) => None,
    }
}

/// Delegates execution to worktree-io: `worktree open <ref>`.
/// Returns false if the issue ref cannot be converted, or the process exits non-zero.
async fn execute_agent(issue_ref: Option<IssueRef>) -> bool {
    let Some(r) = issue_ref else { return false };
    let Some(ref_str) = to_worktree_ref(r) else { return false };

    match tokio::process::Command::new("worktree")
        .arg("open")
        .arg(&ref_str)
        .status()
        .await
    {
        Ok(status) => status.success(),
        Err(_) => false,
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
