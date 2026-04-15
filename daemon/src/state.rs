use crate::issue_ref_json::{issue_ref_json_to_proto, IssueRefJson};
use crate::proto::{DaemonConfig, Task, TaskStatus, WorkerInfo};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    workers_count: i32,
    log_level: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    enabled_agents: Vec<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            workers_count: 4,
            log_level: "info".to_string(),
            enabled_agents: Vec::new(),
        }
    }
}

impl From<ConfigFile> for DaemonConfig {
    fn from(f: ConfigFile) -> Self {
        DaemonConfig {
            workers_count: f.workers_count,
            log_level: f.log_level,
            enabled_agents: f.enabled_agents,
        }
    }
}

impl From<&DaemonConfig> for ConfigFile {
    fn from(c: &DaemonConfig) -> Self {
        ConfigFile {
            workers_count: c.workers_count,
            log_level: c.log_level.clone(),
            enabled_agents: c.enabled_agents.clone(),
        }
    }
}


#[derive(Serialize, Deserialize)]
struct TaskJson {
    id: String,
    issue_ref: Option<IssueRefJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    status: i32,
    priority: i32,
    created_at: Option<i64>,
    updated_at: Option<i64>,
}

impl From<Task> for TaskJson {
    fn from(t: Task) -> Self {
        TaskJson {
            id: t.id,
            issue_ref: t.issue_ref.map(IssueRefJson::from),
            agent: t.agent,
            status: t.status,
            priority: t.priority,
            created_at: t.created_at.map(|ts| ts.seconds),
            updated_at: t.updated_at.map(|ts| ts.seconds),
        }
    }
}

impl From<TaskJson> for Task {
    fn from(j: TaskJson) -> Self {
        Task {
            id: j.id,
            issue_ref: j.issue_ref.and_then(issue_ref_json_to_proto),
            agent: j.agent,
            status: j.status,
            priority: j.priority,
            created_at: j.created_at.map(|s| prost_types::Timestamp {
                seconds: s,
                nanos: 0,
            }),
            updated_at: j.updated_at.map(|s| prost_types::Timestamp {
                seconds: s,
                nanos: 0,
            }),
        }
    }
}

pub struct AppState {
    pub config_path: String,
    pub queue_path: String,
    pub queue: Vec<Task>,
    pub workers: Vec<WorkerInfo>,
    pub config: DaemonConfig,
}

impl AppState {
    pub fn new(config_path: String) -> Self {
        let queue_path = derive_queue_path(&config_path);
        let config = Self::load_config(&config_path);
        let queue = Self::load_queue(&queue_path);
        Self {
            config_path,
            queue_path,
            queue,
            workers: Vec::new(),
            config,
        }
    }

    fn load_config(path: &str) -> DaemonConfig {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str::<ConfigFile>(&contents)
                .unwrap_or_default()
                .into(),
            Err(_) => ConfigFile::default().into(),
        }
    }

    pub fn save_config(&self) -> Result<(), String> {
        let file: ConfigFile = (&self.config).into();
        let contents = toml::to_string_pretty(&file).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&self.config_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.config_path, contents).map_err(|e| e.to_string())
    }

    /// Reload config from disk. Returns an error if the file exists but is malformed.
    /// A missing file is treated as "use defaults" (same as startup) and returns Ok.
    pub fn reload_config(&mut self) -> Result<(), String> {
        match std::fs::read_to_string(&self.config_path) {
            Ok(contents) => {
                self.config = toml::from_str::<ConfigFile>(&contents)
                    .map_err(|e| format!("malformed config at '{}': {e}", self.config_path))?
                    .into();
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.config = ConfigFile::default().into();
                Ok(())
            }
            Err(e) => Err(format!("failed to read config at '{}': {e}", self.config_path)),
        }
    }

    fn load_queue(path: &str) -> Vec<Task> {
        match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str::<Vec<TaskJson>>(&contents)
                .unwrap_or_default()
                .into_iter()
                .map(Task::from)
                .map(|mut t| {
                    // Reset tasks that were RUNNING when the daemon last exited —
                    // those workers are gone, so re-queue them to be dispatched again.
                    if t.status == TaskStatus::Running as i32 {
                        t.status = TaskStatus::Queued as i32;
                    }
                    t
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Remove completed/failed tasks whose `updated_at` is older than `max_age_secs`.
    pub fn prune_old_tasks(&mut self, max_age_secs: i64) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - max_age_secs;
        self.queue.retain(|t| {
            let done = t.status == TaskStatus::Completed as i32
                || t.status == TaskStatus::Failed as i32;
            if !done {
                return true; // keep active tasks always
            }
            // Keep if updated_at is recent enough (or missing — defensive).
            t.updated_at.as_ref().is_none_or(|ts| ts.seconds >= cutoff)
        });
    }

    pub fn save_queue(&mut self) -> Result<(), String> {
        // Build the serializable task list, excluding stale terminal tasks (>7 days old).
        // Crucially, we do NOT mutate self.queue yet: only prune in memory AFTER a
        // successful write so that a failed write leaves the in-memory queue untouched
        // for rollback callers (enqueue, update_task, remove_task, worker_pool).
        const MAX_AGE_SECS: i64 = 7 * 24 * 3600;
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - MAX_AGE_SECS;
        let json_tasks: Vec<TaskJson> = self
            .queue
            .iter()
            .filter(|t| {
                let terminal = t.status == TaskStatus::Completed as i32
                    || t.status == TaskStatus::Failed as i32;
                if !terminal {
                    return true;
                }
                // Keep recent terminal tasks; drop stale ones from the persisted file.
                t.updated_at.as_ref().is_none_or(|ts| ts.seconds >= cutoff)
            })
            .cloned()
            .map(TaskJson::from)
            .collect();
        let contents = serde_json::to_string_pretty(&json_tasks).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&self.queue_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.queue_path, &contents).map_err(|e| e.to_string())?;
        // Write succeeded — now prune in memory to match what was written to disk.
        self.prune_old_tasks(MAX_AGE_SECS);
        Ok(())
    }
}

fn derive_queue_path(config_path: &str) -> String {
    let parent = std::path::Path::new(config_path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{parent}/queue.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(status: TaskStatus, updated_secs: Option<i64>) -> Task {
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            issue_ref: None,
            agent: None,
            status: status as i32,
            priority: 0,
            created_at: None,
            updated_at: updated_secs.map(|s| prost_types::Timestamp { seconds: s, nanos: 0 }),
        }
    }

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    #[test]
    fn prune_removes_old_completed_tasks() {
        let mut state = AppState {
            config_path: "/tmp/test-config.toml".into(),
            queue_path: "/tmp/test-queue.json".into(),
            queue: vec![
                make_task(TaskStatus::Completed, Some(now_secs() - 8 * 24 * 3600)), // 8 days old, pruned
                make_task(TaskStatus::Completed, Some(now_secs() - 6 * 24 * 3600)), // 6 days old, kept
                make_task(TaskStatus::Queued, Some(now_secs() - 100 * 24 * 3600)),  // active, kept
                make_task(TaskStatus::Running, None),                                // active, kept
            ],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        state.prune_old_tasks(7 * 24 * 3600);
        assert_eq!(state.queue.len(), 3);
        assert!(state.queue.iter().all(|t| t.status != TaskStatus::Completed as i32 || {
            t.updated_at.as_ref().map_or(true, |ts| ts.seconds > now_secs() - 7 * 24 * 3600)
        }));
    }

    #[test]
    fn prune_removes_old_failed_tasks() {
        let mut state = AppState {
            config_path: "/tmp/test-config.toml".into(),
            queue_path: "/tmp/test-queue.json".into(),
            queue: vec![
                make_task(TaskStatus::Failed, Some(now_secs() - 8 * 24 * 3600)),
                make_task(TaskStatus::Failed, Some(now_secs() - 1)),
            ],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        state.prune_old_tasks(7 * 24 * 3600);
        assert_eq!(state.queue.len(), 1);
    }

    #[test]
    fn prune_keeps_tasks_with_no_updated_at() {
        let mut state = AppState {
            config_path: "/tmp/test-config.toml".into(),
            queue_path: "/tmp/test-queue.json".into(),
            queue: vec![make_task(TaskStatus::Completed, None)],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        state.prune_old_tasks(7 * 24 * 3600);
        // Tasks without updated_at are kept defensively
        assert_eq!(state.queue.len(), 1);
    }

    #[test]
    fn prune_empty_queue_is_noop() {
        let mut state = AppState {
            config_path: "/tmp/test-config.toml".into(),
            queue_path: "/tmp/test-queue.json".into(),
            queue: vec![],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        state.prune_old_tasks(7 * 24 * 3600);
        assert_eq!(state.queue.len(), 0);
    }

    #[test]
    fn derive_queue_path_sibling_of_config() {
        assert_eq!(derive_queue_path("/home/user/.config/team/config.toml"), "/home/user/.config/team/queue.json");
    }

    #[test]
    fn derive_queue_path_root_config() {
        assert_eq!(derive_queue_path("config.toml"), "/queue.json");
    }

    #[test]
    fn derive_queue_path_empty_config_uses_default() {
        // Path::new("").parent() returns None — map() gives None — unwrap_or_default()
        // takes the None arm and returns String::default() = "".
        assert_eq!(derive_queue_path(""), "/queue.json");
    }

    #[test]
    fn config_save_and_reload_round_trip() {
        let path = format!("/tmp/state-test-config-{}.toml", uuid::Uuid::new_v4());
        let mut state = AppState {
            config_path: path.clone(),
            queue_path: "/tmp/state-test-queue.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig {
                workers_count: 8,
                log_level: "debug".into(),
                enabled_agents: vec!["review".into(), "qa".into()],
            },
        };
        state.save_config().expect("save_config must succeed");
        state.config = DaemonConfig::default(); // wipe in-memory config
        state.reload_config().expect("reload_config must succeed for valid file");
        assert_eq!(state.config.workers_count, 8);
        assert_eq!(state.config.log_level, "debug");
        assert_eq!(state.config.enabled_agents, vec!["review", "qa"]);
        let _ = std::fs::remove_file(&path);
    }

    /// save_config must return Err when create_dir_all fails for the parent directory.
    /// The happy-path test (config_save_and_reload_round_trip) never triggers line 125
    /// because /tmp already exists.  Using "/dev/null/config.toml" as the config path
    /// makes parent = "/dev/null" — a character device, not a directory — so
    /// create_dir_all fails (cannot create a directory where a special file exists),
    /// and the `?` at line 125 propagates the error back to the caller.
    #[test]
    fn save_config_returns_error_when_parent_dir_creation_fails() {
        let state = AppState {
            config_path: "/dev/null/config.toml".into(),
            queue_path: "/tmp/save-config-err-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 4, log_level: "info".into(), enabled_agents: vec![] },
        };
        let result = state.save_config();
        assert!(result.is_err(), "save_config must return Err when create_dir_all fails for the parent");
    }

    #[test]
    fn config_reload_falls_back_to_defaults_when_file_missing() {
        let path = "/tmp/nonexistent-config-state-test.toml".to_string();
        let _ = std::fs::remove_file(&path);
        let mut state = AppState {
            config_path: path.clone(),
            queue_path: "/tmp/state-test-queue.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 99, log_level: "trace".into(), enabled_agents: vec![] },
        };
        state.reload_config().expect("missing file must not be an error"); // falls back to defaults
        assert_eq!(state.config.workers_count, 4); // default
        assert_eq!(state.config.log_level, "info"); // default
    }

    #[test]
    fn reload_config_returns_err_for_malformed_toml() {
        let path = format!("/tmp/team-reload-malformed-{}.toml", uuid::Uuid::new_v4());
        std::fs::write(&path, "this is [[[not valid toml").unwrap();
        let mut state = AppState {
            config_path: path.clone(),
            queue_path: "/tmp/state-test-queue.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 7, log_level: "warn".into(), enabled_agents: vec![] },
        };
        let err = state.reload_config().expect_err("malformed TOML must return Err");
        assert!(err.contains("malformed config"), "error should describe the problem, got: {err}");
        // Config must not have changed — still the original values
        assert_eq!(state.config.workers_count, 7, "config must be unchanged after failed reload");
        let _ = std::fs::remove_file(&path);
    }

    /// save_queue must NOT prune in memory when the disk write fails.
    /// Violating this invariant would silently corrupt rollback callers: their
    /// own rollback restores the specific task they mutated, but pruned tasks
    /// can never be restored — leaving memory with fewer tasks than disk.
    #[test]
    fn save_queue_does_not_prune_in_memory_when_write_fails() {
        let old_ts = now_secs() - 8 * 24 * 3600; // 8 days ago — prune-eligible
        let mut state = AppState {
            config_path: "/tmp/prune-write-fail-test.toml".into(),
            queue_path: "/nonexistent-dir/queue.json".into(), // path that cannot be written
            queue: vec![make_task(TaskStatus::Completed, Some(old_ts))],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        assert_eq!(state.queue.len(), 1, "setup: queue must have the old terminal task");
        let result = state.save_queue();
        assert!(result.is_err(), "save_queue must fail with a path that cannot be written");
        assert_eq!(
            state.queue.len(), 1,
            "old terminal task must survive in memory after a failed write — \
             rollback callers depend on this invariant"
        );
    }

    /// save_queue keeps a terminal task with no updated_at in both the JSON output and
    /// in memory after a successful write.  The filter uses `is_none_or(...)` which
    /// returns true for None — the defensive case.  `prune_keeps_tasks_with_no_updated_at`
    /// covers the same None arm for `prune_old_tasks` called directly, but here we verify
    /// that save_queue's own filter (distinct code path) also handles None correctly.
    #[test]
    fn save_queue_keeps_terminal_task_with_no_updated_at() {
        let path = format!("/tmp/state-prune-no-ts-{}.json", uuid::Uuid::new_v4());
        let mut state = AppState {
            config_path: "/tmp/prune-no-ts-test.toml".into(),
            queue_path: path.clone(),
            queue: vec![make_task(TaskStatus::Completed, None)],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        state.save_queue().expect("save_queue must succeed with a writable path");
        // Terminal task with no updated_at is kept defensively in both the JSON
        // and in memory — is_none_or(None) returns true so it is not filtered out.
        assert_eq!(state.queue.len(), 1, "terminal task with no updated_at must not be pruned");
        let _ = std::fs::remove_file(&path);
    }

    /// Complement of the above: after a SUCCESSFUL write, stale terminal tasks
    /// should be pruned from memory to match what was written to disk.
    #[test]
    fn save_queue_prunes_in_memory_after_successful_write() {
        let path = format!("/tmp/state-prune-success-{}.json", uuid::Uuid::new_v4());
        let old_ts = now_secs() - 8 * 24 * 3600;
        let mut state = AppState {
            config_path: "/tmp/prune-success-test.toml".into(),
            queue_path: path.clone(),
            queue: vec![make_task(TaskStatus::Completed, Some(old_ts))],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        assert_eq!(state.queue.len(), 1);
        state.save_queue().expect("save_queue must succeed with a writable path");
        assert!(
            state.queue.is_empty(),
            "old terminal task must be pruned from memory after a successful write"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// load_config (used at startup via AppState::new) silently falls back to defaults
    /// when the TOML is valid syntax but has a type mismatch — unlike reload_config
    /// which returns an error for the same input. This is intentional: startup must
    /// not abort if the config file is slightly wrong.
    #[test]
    fn load_config_type_mismatch_uses_defaults() {
        let dir = format!("/tmp/team-state-type-mismatch-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        // Valid TOML syntax but workers_count is a string, not an integer.
        std::fs::write(&config_path, r#"workers_count = "four"\nlog_level = "debug""#).unwrap();
        let state = AppState::new(config_path);
        // Falls back to defaults — workers_count=4, log_level="info"
        assert_eq!(state.config.workers_count, 4, "type-mismatch config must fall back to default workers_count");
        assert_eq!(state.config.log_level, "info", "type-mismatch config must fall back to default log_level");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reload_config_returns_err_for_non_notfound_io_error() {
        // Passing a directory path causes read_to_string to fail with an error whose
        // kind is NOT NotFound (IsADirectory / Other on macOS+Linux).
        // reload_config must propagate this as Err("failed to read config...") rather
        // than silently falling back to defaults (which is only correct for missing files).
        let mut state = AppState {
            config_path: "/tmp".into(), // existing directory, not a file
            queue_path: "/tmp/state-test-queue.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 7, log_level: "warn".into(), enabled_agents: vec![] },
        };
        let result = state.reload_config();
        // Config must remain unchanged when reload fails.
        assert_eq!(state.config.workers_count, 7, "config must not change on IO error");
        let err = result.expect_err("reading a directory must return Err, not fall back to defaults");
        assert!(err.contains("failed to read config"), "got: {err}");
    }

    #[test]
    fn load_queue_with_malformed_json_returns_empty_queue() {
        // load_queue calls `serde_json::from_str(...).unwrap_or_default()` — if the queue
        // file contains invalid JSON the daemon must not panic; it silently starts with an
        // empty queue so dispatch can still proceed.
        let dir = format!("/tmp/team-state-malformed-queue-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        let queue_path = format!("{dir}/queue.json");
        std::fs::write(&queue_path, "this is not valid json [{{").unwrap();
        let state = AppState::new(config_path);
        assert!(state.queue.is_empty(), "malformed queue JSON must result in an empty queue, not a panic");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn running_tasks_are_re_queued_on_load() {
        // Create a unique temp dir so config path and queue path are both isolated.
        let dir = format!("/tmp/team-state-test-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        let queue_path = format!("{dir}/queue.json"); // matches derive_queue_path output

        // Write a queue file with a RUNNING task (status=1)
        let json = r#"[{"id":"t1","issue_ref":null,"status":1,"priority":0,"created_at":null,"updated_at":null}]"#;
        std::fs::write(&queue_path, json).unwrap();

        let state = AppState::new(config_path);
        assert_eq!(state.queue.len(), 1);
        assert_eq!(state.queue[0].status, TaskStatus::Queued as i32, "RUNNING tasks must be reset to QUEUED on load");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Verifies that the `agent` field in the queue JSON is deserialized to `Some(String)`
    /// when it is present.  Every other load test omits the `agent` key entirely, which
    /// hits the `#[serde(default)]` → `None` path.  This test hits the `Some` path for
    /// the first time, completing the two-arm coverage of the agent field.
    #[test]
    fn load_queue_preserves_explicit_agent_field() {
        let dir = format!("/tmp/team-state-agent-rt-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        let queue_path = format!("{dir}/queue.json");
        // Include an explicit `agent` field — exercises the Some(_) deserialization arm.
        let json = r#"[{"id":"t1","issue_ref":null,"agent":"review","status":0,"priority":0,"created_at":null,"updated_at":null}]"#;
        std::fs::write(&queue_path, json).unwrap();
        let state = AppState::new(config_path);
        assert_eq!(state.queue.len(), 1);
        assert_eq!(
            state.queue[0].agent,
            Some("review".into()),
            "agent field present in JSON must deserialize to Some(\"review\")"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Covers the `Some(IssueRefJson)` arm of `j.issue_ref.and_then(issue_ref_json_to_proto)`
    /// in `Task::from(TaskJson)`.  Every other load test uses `"issue_ref":null` which takes
    /// the outer-None path of `and_then`.  This test writes a non-null ref string and verifies
    /// the full parse → proto conversion chain fires correctly through `load_queue`.
    #[test]
    fn load_queue_deserializes_issue_ref_string() {
        let dir = format!("/tmp/team-state-issue-ref-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        let queue_path = format!("{dir}/queue.json");
        let json = r#"[{"id":"t1","issue_ref":"github:acme/backend#42","status":0,"priority":0,"created_at":null,"updated_at":null}]"#;
        std::fs::write(&queue_path, json).unwrap();
        let state = AppState::new(config_path);
        assert_eq!(state.queue.len(), 1);
        let r = state.queue[0].issue_ref.as_ref().expect("issue_ref must be Some after parsing");
        match &r.r#ref {
            Some(crate::proto::issue_ref::Ref::Github(g)) => {
                assert_eq!(g.organization, "acme");
                assert_eq!(g.repository, "backend");
                assert_eq!(g.number, "42");
            }
            other => panic!("expected Github ref, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Exercises the `Ok(contents) → toml::from_str succeeds → .into()` arm of `load_config`,
    /// which is the full success path reached only when a well-formed TOML config file exists at
    /// startup.  All other `AppState::new` tests either hit the missing-file arm (config doesn't
    /// exist → `Err(_) → ConfigFile::default()`) or the parse-failure arm (file exists but
    /// `toml::from_str` fails → `unwrap_or_default()`).  Neither exercises the branch where the
    /// file is readable AND parseable — the normal daemon-restart scenario.
    #[test]
    fn load_config_success_path_reads_valid_toml_config() {
        let dir = format!("/tmp/team-state-load-config-ok-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        std::fs::write(
            &config_path,
            "workers_count = 8\nlog_level = \"debug\"\nenabled_agents = [\"review\", \"qa\"]\n",
        )
        .unwrap();
        let state = AppState::new(config_path);
        assert_eq!(state.config.workers_count, 8, "workers_count must be loaded from TOML");
        assert_eq!(state.config.log_level, "debug", "log_level must be loaded from TOML");
        assert_eq!(
            state.config.enabled_agents,
            vec!["review", "qa"],
            "enabled_agents must be loaded from TOML"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Covers the `Some(_)` arm of `j.created_at.map(...)` and `j.updated_at.map(...)` in
    /// `Task::from(TaskJson)`.  Every other load test writes `null` for both timestamp fields,
    /// so only the `None` → `None` path is exercised there.  Here both are non-null integers
    /// and must round-trip to `Some(prost_types::Timestamp{seconds, nanos:0})`.
    #[test]
    fn load_queue_preserves_timestamps() {
        let dir = format!("/tmp/team-state-timestamps-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        let queue_path = format!("{dir}/queue.json");
        let json = r#"[{"id":"t1","issue_ref":null,"status":0,"priority":0,"created_at":1700000000,"updated_at":1700000001}]"#;
        std::fs::write(&queue_path, json).unwrap();
        let state = AppState::new(config_path);
        assert_eq!(state.queue.len(), 1);
        let task = &state.queue[0];
        assert_eq!(
            task.created_at,
            Some(prost_types::Timestamp { seconds: 1700000000, nanos: 0 }),
            "created_at must deserialize from a JSON integer"
        );
        assert_eq!(
            task.updated_at,
            Some(prost_types::Timestamp { seconds: 1700000001, nanos: 0 }),
            "updated_at must deserialize from a JSON integer"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Exercises the false arm of `if t.status == TaskStatus::Running` in load_queue:
    /// non-Running tasks (Queued=0, Completed=2) must be loaded with their status unchanged.
    /// The existing `running_tasks_are_re_queued_on_load` only tests the true arm (Running→Queued).
    #[test]
    fn non_running_tasks_are_loaded_with_original_status() {
        let dir = format!("/tmp/team-state-test-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = format!("{dir}/config.toml");
        let queue_path = format!("{dir}/queue.json");
        // status=0 is Queued, status=2 is Completed
        let json = r#"[{"id":"t1","issue_ref":null,"status":0,"priority":0,"created_at":null,"updated_at":null},
                       {"id":"t2","issue_ref":null,"status":2,"priority":0,"created_at":null,"updated_at":null}]"#;
        std::fs::write(&queue_path, json).unwrap();
        let state = AppState::new(config_path);
        assert_eq!(state.queue.len(), 2);
        assert_eq!(state.queue[0].status, TaskStatus::Queued as i32, "queued task must remain queued");
        assert_eq!(state.queue[1].status, TaskStatus::Completed as i32, "completed task must remain completed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Exercises the `!terminal { return true }` early-return arm of `save_queue`'s filter.
    /// Every other `save_queue` test puts only terminal (Completed/Failed) tasks in the queue,
    /// so the non-terminal path is never reached in those tests.
    ///
    /// This test places a QUEUED task alongside a stale Completed task and verifies:
    ///   1. The Completed task is pruned from both the JSON file and in-memory queue.
    ///   2. The Queued task passes through the `!terminal { return true }` arm and is
    ///      preserved in both the JSON file and in-memory queue.
    #[test]
    fn save_queue_preserves_non_terminal_tasks_and_prunes_stale_completed() {
        let path = format!("/tmp/state-save-queue-non-terminal-{}.json", uuid::Uuid::new_v4());
        let old_ts = now_secs() - 8 * 24 * 3600; // 8 days ago — prune-eligible
        let queued_task = make_task(TaskStatus::Queued, Some(now_secs())); // non-terminal: always kept
        let stale_task = make_task(TaskStatus::Completed, Some(old_ts));  // terminal + stale: pruned
        let queued_id = queued_task.id.clone();
        let mut state = AppState {
            config_path: "/tmp/save-queue-non-terminal-test.toml".into(),
            queue_path: path.clone(),
            queue: vec![queued_task, stale_task],
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 1, log_level: "info".into(), enabled_agents: vec![] },
        };
        assert_eq!(state.queue.len(), 2, "setup: both tasks must be present");
        state.save_queue().expect("save_queue must succeed");
        // The stale Completed task is pruned; the Queued task survives.
        assert_eq!(state.queue.len(), 1, "stale completed task must be pruned");
        assert_eq!(state.queue[0].id, queued_id, "queued task must survive (!terminal early-return arm)");
        assert_eq!(state.queue[0].status, TaskStatus::Queued as i32);
        // Verify the JSON file also only contains the queued task.
        let contents = std::fs::read_to_string(&path).expect("queue file must exist");
        assert!(contents.contains(&queued_id), "queued task id must appear in the JSON output");
        let _ = std::fs::remove_file(&path);
    }
}
