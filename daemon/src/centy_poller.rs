//! Polls `centy list issues --status "in queue" --global --json` every 30 s
//! and auto-enqueues any issue not already active in the task queue.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::proto::{issue_ref, CentyIssueRef, IssueRef, Task, TaskStatus};
use crate::state::AppState;

pub fn start(state: Arc<Mutex<AppState>>) {
    tokio::spawn(poll_loop(state));
}

async fn poll_loop(state: Arc<Mutex<AppState>>) {
    // Initial delay so the daemon finishes startup before the first poll.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        poll_once(&state).await;
    }
}

async fn poll_once(state: &Arc<Mutex<AppState>>) {
    let issues = match fetch_in_queue_issues().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[centy_poller] {e}");
            return;
        }
    };

    if issues.is_empty() {
        return;
    }

    let mut s = state.lock().await;
    let mut enqueued = 0u32;

    for issue in issues {
        // Skip if the same centy issue is already QUEUED or RUNNING.
        let active = s.queue.iter().any(|t| {
            if let Some(IssueRef { r#ref: Some(issue_ref::Ref::Centy(ref c)) }) = t.issue_ref {
                c.number == issue.number
                    && (t.status == TaskStatus::Queued as i32
                        || t.status == TaskStatus::Running as i32)
            } else {
                false
            }
        });
        if active {
            continue;
        }

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let ts = prost_types::Timestamp { seconds: now_secs, nanos: 0 };

        s.queue.push(Task {
            id: uuid::Uuid::new_v4().to_string(),
            issue_ref: Some(IssueRef {
                r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
                    organization: issue.organization.clone(),
                    repository: issue.repository.clone(),
                    number: issue.number.clone(),
                })),
            }),
            agent: None,
            status: TaskStatus::Queued as i32,
            priority: issue.priority,
            created_at: Some(ts),
            updated_at: Some(ts),
        });
        enqueued += 1;
        eprintln!(
            "[centy_poller] auto-enqueued centy issue #{} ({}/{})",
            issue.number, issue.organization, issue.repository
        );
    }

    if enqueued > 0 {
        let _ = s.save_queue();
    }
}

struct CentyIssue {
    organization: String,
    repository: String,
    /// display number as string (used in IssueRef)
    number: String,
    priority: i32,
}

async fn fetch_in_queue_issues() -> Result<Vec<CentyIssue>, String> {
    let output = tokio::process::Command::new("centy")
        .args(["list", "issues", "--status", "in queue", "--global", "--json"])
        .output()
        .await
        .map_err(|e| format!("failed to spawn centy: {e}"))?;

    // centy may exit non-zero if there are no projects; treat that as empty.
    let stdout = String::from_utf8_lossy(&output.stdout);

    // centy sometimes prints warnings to stdout before the JSON array.
    let json_start = match stdout.find('[') {
        Some(pos) => pos,
        None => return Ok(Vec::new()),
    };

    let items: Vec<serde_json::Value> =
        serde_json::from_str(&stdout[json_start..])
            .map_err(|e| format!("failed to parse centy JSON: {e}"))?;

    let mut result = Vec::new();
    for item in items {
        let display_number = match item["metadata"]["displayNumber"].as_i64() {
            Some(n) => n,
            None => continue,
        };
        let priority = item["metadata"]["priority"].as_i64().unwrap_or(0) as i32;
        let project_path = item["projectPath"].as_str().unwrap_or("");
        let (org, repo) = extract_org_repo(project_path);
        result.push(CentyIssue {
            organization: org,
            repository: repo,
            number: display_number.to_string(),
            priority,
        });
    }

    Ok(result)
}

/// Extracts (organization, repository) from a local file-system project path.
/// `/home/user/dev/github/acme/my-repo` → `("acme", "my-repo")`
fn extract_org_repo(path: &str) -> (String, String) {
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    match parts.as_slice() {
        [.., org, repo] if !org.is_empty() && !repo.is_empty() => {
            ((*org).to_string(), (*repo).to_string())
        }
        _ => ("unknown".to_string(), "unknown".to_string()),
    }
}
