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

    pub fn reload_config(&mut self) {
        self.config = Self::load_config(&self.config_path);
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
        // Prune stale terminal tasks (>7 days old) before persisting.
        self.prune_old_tasks(7 * 24 * 3600);
        let json_tasks: Vec<TaskJson> = self.queue.iter().cloned().map(TaskJson::from).collect();
        let contents = serde_json::to_string_pretty(&json_tasks).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&self.queue_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.queue_path, contents).map_err(|e| e.to_string())
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
        state.reload_config();
        assert_eq!(state.config.workers_count, 8);
        assert_eq!(state.config.log_level, "debug");
        assert_eq!(state.config.enabled_agents, vec!["review", "qa"]);
        let _ = std::fs::remove_file(&path);
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
        state.reload_config(); // file doesn't exist → falls back to defaults
        assert_eq!(state.config.workers_count, 4); // default
        assert_eq!(state.config.log_level, "info"); // default
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
}
