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
    enqueue_new_issues(state, issues).await;
}

/// Enqueues any issues from `issues` that are not already present in the queue.
/// If the queue cannot be persisted after appending new tasks, all newly-added
/// tasks are rolled back so in-memory state stays consistent with disk.
async fn enqueue_new_issues(state: &Arc<Mutex<AppState>>, issues: Vec<CentyIssue>) {
    if issues.is_empty() {
        return;
    }

    let mut s = state.lock().await;
    let before_len = s.queue.len();
    let mut enqueued = 0u32;

    for issue in issues {
        // Skip if the centy issue is already in the queue in ANY state.
        // COMPLETED/FAILED tasks are retained for 7 days (see save_queue pruning),
        // which prevents re-dispatch of recently finished work.  Once pruned, a
        // still-"in queue" centy issue will be picked up again on the next poll.
        if is_centy_issue_present(&s.queue, &issue) {
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

    if enqueued > 0
        && let Err(e) = s.save_queue()
    {
        // Roll back all newly pushed tasks so in-memory state stays consistent with disk.
        s.queue.truncate(before_len);
        eprintln!("[centy_poller] failed to persist {enqueued} auto-enqueued task(s): {e}; rolled back");
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

    // Log stderr when centy exits non-zero (common for "no projects configured",
    // but also catches real failures that would otherwise be invisible).
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "[centy_poller] centy exited with code {} — stderr: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        );
        // Fall through: non-zero exit may still produce valid JSON.
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // centy sometimes prints warnings to stdout before the JSON array.
    let json_start = match stdout.find('[') {
        Some(pos) => pos,
        None => return Ok(Vec::new()),
    };

    let items: Vec<serde_json::Value> =
        serde_json::from_str(&stdout[json_start..])
            .map_err(|e| format!("failed to parse centy JSON: {e}"))?;

    Ok(items.iter().filter_map(parse_centy_item).collect())
}

/// Parse a single centy JSON item into a `CentyIssue`.
///
/// Returns `None` (and logs) when the item is missing required fields or the
/// project path cannot be resolved to an org/repo pair — both indicate data we
/// cannot safely route, so we skip rather than enqueue with wrong metadata.
fn parse_centy_item(item: &serde_json::Value) -> Option<CentyIssue> {
    let display_number = item["metadata"]["displayNumber"].as_i64()?;
    let priority = item["metadata"]["priority"].as_i64().unwrap_or(0) as i32;
    let project_path = item["projectPath"].as_str().unwrap_or("");
    let (org, repo) = extract_org_repo(project_path);
    if org == "unknown" && repo == "unknown" {
        eprintln!(
            "[centy_poller] skipping issue #{display_number}: cannot determine org/repo from projectPath {project_path:?}"
        );
        return None;
    }
    Some(CentyIssue { organization: org, repository: repo, number: display_number.to_string(), priority })
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
    use crate::proto::{CentyIssueRef, DaemonConfig, GitHubIssueRef};

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
    fn parse_item_returns_some_with_valid_data() {
        let item = serde_json::json!({
            "metadata": { "displayNumber": 42, "priority": 5 },
            "projectPath": "/home/user/dev/github/acme/backend"
        });
        let issue = parse_centy_item(&item).expect("should parse");
        assert_eq!(issue.number, "42");
        assert_eq!(issue.organization, "acme");
        assert_eq!(issue.repository, "backend");
        assert_eq!(issue.priority, 5);
    }

    #[test]
    fn parse_item_defaults_priority_to_zero_when_missing() {
        let item = serde_json::json!({
            "metadata": { "displayNumber": 3 },
            "projectPath": "/home/user/dev/org/repo"
        });
        let issue = parse_centy_item(&item).expect("should parse");
        assert_eq!(issue.priority, 0);
    }

    #[test]
    fn parse_item_returns_none_when_display_number_missing() {
        let item = serde_json::json!({
            "metadata": { "priority": 5 },
            "projectPath": "/home/user/dev/github/acme/backend"
        });
        assert!(parse_centy_item(&item).is_none());
    }

    #[test]
    fn parse_item_returns_none_when_project_path_unresolvable() {
        // Empty path → extract_org_repo returns ("unknown","unknown") → skipped
        let item = serde_json::json!({
            "metadata": { "displayNumber": 7, "priority": 0 },
            "projectPath": ""
        });
        assert!(parse_centy_item(&item).is_none());
    }

    #[test]
    fn parse_item_returns_none_when_project_path_single_component() {
        let item = serde_json::json!({
            "metadata": { "displayNumber": 8 },
            "projectPath": "noslash"
        });
        assert!(parse_centy_item(&item).is_none());
    }

    #[test]
    fn parse_item_returns_none_when_display_number_is_string() {
        // as_i64() returns None for JSON strings — even if the string looks like a number.
        let item = serde_json::json!({
            "metadata": { "displayNumber": "42", "priority": 0 },
            "projectPath": "/home/user/dev/acme/backend"
        });
        assert!(parse_centy_item(&item).is_none());
    }

    #[test]
    fn parse_item_returns_none_when_display_number_is_null() {
        let item = serde_json::json!({
            "metadata": { "displayNumber": null },
            "projectPath": "/home/user/dev/acme/backend"
        });
        assert!(parse_centy_item(&item).is_none());
    }

    #[test]
    fn parse_item_returns_none_when_project_path_is_non_string_type() {
        // `item["projectPath"].as_str()` returns None for non-string JSON values
        // (number, bool, array, object, null) — `unwrap_or("")` takes the None arm,
        // giving "" to extract_org_repo, which falls back to ("unknown","unknown") → None.
        // The existing parse_item_returns_none_when_project_path_unresolvable uses
        // `"projectPath": ""` (a string), so as_str() returns Some("") there.
        // This test hits the distinct unwrap_or default arm.
        let item = serde_json::json!({
            "metadata": { "displayNumber": 42 },
            "projectPath": 123
        });
        assert!(parse_centy_item(&item).is_none());
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

    #[test]
    fn extract_root_slash_falls_back() {
        // "/" trims to "" → single empty element → no org/repo → fallback
        assert_eq!(
            extract_org_repo("/"),
            ("unknown".into(), "unknown".into())
        );
    }

    #[test]
    fn extract_single_component_with_leading_slash_falls_back() {
        // "/repo" splits to ["", "repo"] → org="" fails the !org.is_empty() guard → fallback
        assert_eq!(
            extract_org_repo("/repo"),
            ("unknown".into(), "unknown".into())
        );
    }

    #[test]
    fn extract_no_leading_slash_with_two_components() {
        // "org/repo" splits to ["org", "repo"] — no leading empty string.
        // The [.., org, repo] pattern matches with ..=[], org="org", repo="repo".
        // This is a distinct slice size from "/org/repo" which splits to
        // ["", "org", "repo"] (..=[""]); both reach the same match arm but the
        // existing extract_short_path test only covers the 3-element case.
        assert_eq!(
            extract_org_repo("org/repo"),
            ("org".into(), "repo".into())
        );
    }

    fn make_state_with_path(queue_path: &str) -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            config_path: "/tmp/test.toml".into(),
            queue_path: queue_path.into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig {
                workers_count: 1,
                log_level: "info".into(),
                enabled_agents: vec![],
            },
        }))
    }

    #[tokio::test]
    async fn enqueue_new_issues_is_noop_for_empty_list() {
        // Exercises the `if issues.is_empty() { return; }` guard at line 40 —
        // no existing test calls enqueue_new_issues with zero issues.
        let state = make_state_with_path("/tmp/centy-poller-empty-noop.json");
        enqueue_new_issues(&state, vec![]).await;
        assert_eq!(state.lock().await.queue.len(), 0, "empty issues list must not touch the queue");
    }

    #[tokio::test]
    async fn enqueue_new_issues_adds_task_to_queue() {
        let path = format!("/tmp/test-centy-poller-{}.json", uuid::Uuid::new_v4());
        let state = make_state_with_path(&path);
        let issues = vec![centy_issue("acme", "backend", "42")];
        enqueue_new_issues(&state, issues).await;
        let s = state.lock().await;
        assert_eq!(s.queue.len(), 1);
        let task = &s.queue[0];
        match task.issue_ref.as_ref().and_then(|r| r.r#ref.as_ref()) {
            Some(issue_ref::Ref::Centy(c)) => {
                assert_eq!(c.organization, "acme");
                assert_eq!(c.repository, "backend");
                assert_eq!(c.number, "42");
            }
            other => panic!("expected Centy ref, got {other:?}"),
        }
        assert_eq!(task.status, TaskStatus::Queued as i32);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn enqueue_new_issues_propagates_priority_to_task() {
        let path = format!("/tmp/test-centy-poller-{}.json", uuid::Uuid::new_v4());
        let state = make_state_with_path(&path);
        let issues = vec![CentyIssue {
            organization: "acme".into(),
            repository: "backend".into(),
            number: "77".into(),
            priority: 9,
        }];
        enqueue_new_issues(&state, issues).await;
        let s = state.lock().await;
        assert_eq!(s.queue.len(), 1);
        assert_eq!(s.queue[0].priority, 9, "task priority must match the issue priority");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn enqueue_new_issues_skips_already_present() {
        let path = format!("/tmp/test-centy-poller-{}.json", uuid::Uuid::new_v4());
        let state = make_state_with_path(&path);
        // Pre-populate the queue with the same issue.
        state.lock().await.queue.push(centy_task("acme", "backend", "42"));
        let issues = vec![centy_issue("acme", "backend", "42")];
        enqueue_new_issues(&state, issues).await;
        // Queue length must stay at 1 — no duplicate added.
        assert_eq!(state.lock().await.queue.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn enqueue_new_issues_rolls_back_on_save_failure() {
        // /dev/null is a character device, so create_dir_all("/dev/null") fails,
        // which makes save_queue return an Err — triggering the rollback branch.
        let state = make_state_with_path("/dev/null/queue.json");
        let issues = vec![centy_issue("acme", "backend", "99")];
        enqueue_new_issues(&state, issues).await;
        // The task must be rolled back since persistence failed.
        assert_eq!(state.lock().await.queue.len(), 0, "queue must be rolled back when save fails");
    }

    /// When the issues list contains a mix of already-present and new issues, only the
    /// new ones should be enqueued.  The existing tests each cover either all-skipped or
    /// all-new; this test exercises the `continue` branch and the add branch in the same
    /// loop iteration — confirming the filter and push interact correctly.
    #[tokio::test]
    async fn enqueue_new_issues_skips_present_and_adds_new_in_same_call() {
        let path = format!("/tmp/test-centy-mixed-{}.json", uuid::Uuid::new_v4());
        let state = make_state_with_path(&path);
        // Pre-populate with two already-tracked issues.
        state.lock().await.queue.push(centy_task("acme", "backend", "1"));
        state.lock().await.queue.push(centy_task("acme", "backend", "3"));

        let issues = vec![
            centy_issue("acme", "backend", "1"), // already present → skip
            centy_issue("acme", "backend", "2"), // new → add
            centy_issue("acme", "backend", "3"), // already present → skip
        ];
        enqueue_new_issues(&state, issues).await;

        let s = state.lock().await;
        // Only issue #2 was new; queue must have exactly 3 tasks (2 existing + 1 new).
        assert_eq!(s.queue.len(), 3, "only new issues must be added; existing ones must be skipped");
        let numbers: Vec<&str> = s.queue.iter()
            .filter_map(|t| t.issue_ref.as_ref()?.r#ref.as_ref())
            .filter_map(|r| if let issue_ref::Ref::Centy(c) = r { Some(c.number.as_str()) } else { None })
            .collect();
        assert!(numbers.contains(&"2"), "new issue #2 must be enqueued");
        let _ = std::fs::remove_file(&path);
    }

    /// When multiple new issues are enqueued but save_queue then fails, truncate must
    /// remove ALL newly-pushed tasks — not just one.  The existing rollback test only
    /// adds a single task; this test adds two to verify the truncate(before_len) removes
    /// the correct slice regardless of how many entries were appended.
    #[tokio::test]
    async fn enqueue_new_issues_rolls_back_multiple_tasks_on_save_failure() {
        // /dev/null makes save_queue fail so both pushes must be rolled back.
        let state = make_state_with_path("/dev/null/queue.json");
        let issues = vec![
            centy_issue("acme", "backend", "10"),
            centy_issue("acme", "backend", "11"),
        ];
        enqueue_new_issues(&state, issues).await;
        assert_eq!(
            state.lock().await.queue.len(), 0,
            "all newly-pushed tasks must be removed by truncate when save fails"
        );
    }
}
