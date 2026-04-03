use crate::proto::{
    issue_ref, CentyIssueRef, DaemonConfig, GitHubIssueRef, IssueRef, JiraIssueRef, Task,
    WorkerInfo,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    workers_count: i32,
    log_level: String,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            workers_count: 4,
            log_level: "info".to_string(),
        }
    }
}

impl From<ConfigFile> for DaemonConfig {
    fn from(f: ConfigFile) -> Self {
        DaemonConfig {
            workers_count: f.workers_count,
            log_level: f.log_level,
        }
    }
}

impl From<&DaemonConfig> for ConfigFile {
    fn from(c: &DaemonConfig) -> Self {
        ConfigFile {
            workers_count: c.workers_count,
            log_level: c.log_level.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IssueRefJson {
    Github {
        organization: String,
        repository: String,
        number: String,
    },
    Centy {
        organization: String,
        repository: String,
        number: String,
    },
    Jira {
        id: String,
    },
    Link {
        url: String,
    },
}

impl From<IssueRef> for IssueRefJson {
    fn from(r: IssueRef) -> Self {
        match r.r#ref {
            Some(issue_ref::Ref::Github(g)) => IssueRefJson::Github {
                organization: g.organization,
                repository: g.repository,
                number: g.number,
            },
            Some(issue_ref::Ref::Centy(c)) => IssueRefJson::Centy {
                organization: c.organization,
                repository: c.repository,
                number: c.number,
            },
            Some(issue_ref::Ref::Jira(j)) => IssueRefJson::Jira { id: j.id },
            None => unreachable!("IssueRef always has a ref variant"),
        }
    }
}

fn issue_ref_json_to_proto(j: IssueRefJson) -> Option<IssueRef> {
    let r = match j {
        IssueRefJson::Github {
            organization,
            repository,
            number,
        } => issue_ref::Ref::Github(GitHubIssueRef {
            organization,
            repository,
            number,
        }),
        IssueRefJson::Centy {
            organization,
            repository,
            number,
        } => issue_ref::Ref::Centy(CentyIssueRef {
            organization,
            repository,
            number,
        }),
        IssueRefJson::Jira { id } => issue_ref::Ref::Jira(JiraIssueRef { id }),
        // Link refs were valid in old queue files but are no longer stored.
        IssueRefJson::Link { .. } => return None,
    };
    Some(IssueRef { r#ref: Some(r) })
}

#[derive(Serialize, Deserialize)]
struct TaskJson {
    id: String,
    issue_ref: Option<IssueRefJson>,
    agent: String,
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
