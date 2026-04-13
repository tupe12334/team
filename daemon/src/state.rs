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

    pub fn save_queue(&self) -> Result<(), String> {
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
