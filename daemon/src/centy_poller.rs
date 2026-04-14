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
        // Skip if the centy issue is already in the queue in ANY state.
        // COMPLETED/FAILED tasks are retained for 7 days (see save_queue pruning),
        // which prevents re-dispatch of recently finished work.  Once pruned, a
        // still-"in queue" centy issue will be picked up again on the next poll.
        let already_present = is_centy_issue_present(&s.queue, &issue);
        if already_present {
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
    // CENTY_CWD is read natively by the centy CLI's startup hook
    // (process.env['CENTY_CWD'] ?? process.cwd()) — no current_dir() override needed.
    // Setting current_dir to a host path would fail inside Docker where that path
    // does not exist; the env var is sufficient and is inherited by the child process.
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

/// Returns true if a task for this exact centy issue (org + repo + number) is already in the queue.
fn is_centy_issue_present(queue: &[Task], issue: &CentyIssue) -> bool {
    queue.iter().any(|t| {
        matches!(
            &t.issue_ref,
            Some(IssueRef { r#ref: Some(issue_ref::Ref::Centy(c)) })
                if c.number == issue.number
                    && c.organization == issue.organization
                    && c.repository == issue.repository
        )
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{CentyIssueRef, GitHubIssueRef};

    fn centy_task(org: &str, repo: &str, number: &str) -> Task {
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            issue_ref: Some(IssueRef {
                r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
                    organization: org.into(),
                    repository: repo.into(),
                    number: number.into(),
                })),
            }),
            agent: None,
            status: crate::proto::TaskStatus::Queued as i32,
            priority: 0,
            created_at: None,
            updated_at: None,
        }
    }

    fn github_task(org: &str, repo: &str, number: &str) -> Task {
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            issue_ref: Some(IssueRef {
                r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
                    organization: org.into(),
                    repository: repo.into(),
                    number: number.into(),
                })),
            }),
            agent: None,
            status: crate::proto::TaskStatus::Queued as i32,
            priority: 0,
            created_at: None,
            updated_at: None,
        }
    }

    fn centy_issue(org: &str, repo: &str, number: &str) -> CentyIssue {
        CentyIssue { organization: org.into(), repository: repo.into(), number: number.into(), priority: 0 }
    }

    #[test]
    fn is_present_matches_exact_centy_issue() {
        let queue = vec![centy_task("acme", "backend", "7")];
        assert!(is_centy_issue_present(&queue, &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_false_for_different_repo() {
        let queue = vec![centy_task("acme", "frontend", "7")];
        assert!(!is_centy_issue_present(&queue, &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_false_for_different_org() {
        let queue = vec![centy_task("other", "backend", "7")];
        assert!(!is_centy_issue_present(&queue, &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_false_for_different_number() {
        let queue = vec![centy_task("acme", "backend", "8")];
        assert!(!is_centy_issue_present(&queue, &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_false_for_github_task_with_same_number() {
        // A GitHub issue with the same number should NOT block a centy issue
        let queue = vec![github_task("acme", "backend", "7")];
        assert!(!is_centy_issue_present(&queue, &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_false_for_empty_queue() {
        assert!(!is_centy_issue_present(&[], &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_true_for_running_task() {
        // A task in RUNNING state must block re-queuing (daemon restart could duplicate it).
        let mut task = centy_task("acme", "backend", "7");
        task.status = crate::proto::TaskStatus::Running as i32;
        assert!(is_centy_issue_present(&[task], &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_true_for_completed_task() {
        // Completed tasks are retained for 7 days — must block re-queuing within that window.
        let mut task = centy_task("acme", "backend", "7");
        task.status = crate::proto::TaskStatus::Completed as i32;
        assert!(is_centy_issue_present(&[task], &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn is_present_true_for_failed_task() {
        let mut task = centy_task("acme", "backend", "7");
        task.status = crate::proto::TaskStatus::Failed as i32;
        assert!(is_centy_issue_present(&[task], &centy_issue("acme", "backend", "7")));
    }

    #[test]
    fn extract_typical_path() {
        assert_eq!(
            extract_org_repo("/home/user/dev/github/acme/my-repo"),
            ("acme".into(), "my-repo".into())
        );
    }

    #[test]
    fn extract_trailing_slash() {
        assert_eq!(
            extract_org_repo("/home/user/dev/git/github/acme/backend/"),
            ("acme".into(), "backend".into())
        );
    }

    #[test]
    fn extract_short_path() {
        assert_eq!(
            extract_org_repo("/org/repo"),
            ("org".into(), "repo".into())
        );
    }

    #[test]
    fn extract_empty_path_falls_back() {
        assert_eq!(
            extract_org_repo(""),
            ("unknown".into(), "unknown".into())
        );
    }

    #[test]
    fn extract_single_component_falls_back() {
        assert_eq!(
            extract_org_repo("noslash"),
            ("unknown".into(), "unknown".into())
        );
    }
}
