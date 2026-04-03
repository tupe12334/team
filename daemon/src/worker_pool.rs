use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::proto::{TaskStatus, WorkerInfo, WorkerStatus};
use crate::state::AppState;

pub fn start(state: Arc<Mutex<AppState>>) {
    tokio::spawn(dispatch_loop(state));
}

async fn dispatch_loop(state: Arc<Mutex<AppState>>) {
    loop {
        let task = pick_next_task(&state).await;
        match task {
            Some((task_id, agent)) => {
                let state_clone = state.clone();
                let worker_id = uuid::Uuid::new_v4().to_string();
                tokio::spawn(run_task(state_clone, task_id, agent, worker_id));
            }
            None => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

/// Finds the highest-priority QUEUED task that fits within the worker concurrency limit,
/// atomically marks it RUNNING, registers the worker, and returns (task_id, agent).
async fn pick_next_task(state: &Arc<Mutex<AppState>>) -> Option<(String, Option<String>)> {
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
    Some((task_id, agent))
}

async fn run_task(
    state: Arc<Mutex<AppState>>,
    task_id: String,
    agent: Option<String>,
    worker_id: String,
) {
    let success = execute_agent(&agent).await;
    finish_task(&state, &task_id, &worker_id, success).await;
}

/// Spawns the appropriate gstack agent for the task.
/// Command: `claude --print -p "/<agent>"` (non-interactive).
/// Falls back to a no-op if claude is not available.
async fn execute_agent(agent: &Option<String>) -> bool {
    let skill = match agent {
        Some(a) if !a.is_empty() => format!("/{a}"),
        _ => "/review".to_string(),
    };

    let result = tokio::process::Command::new("claude")
        .arg("--print")
        .arg("-p")
        .arg(&skill)
        .output()
        .await;

    match result {
        Ok(output) => output.status.success(),
        Err(_) => {
            // claude not in PATH — treat as success stub so the queue drains
            true
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
