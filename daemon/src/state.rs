use crate::proto::{DaemonConfig, Task, WorkerInfo};
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

pub struct AppState {
    pub config_path: String,
    pub queue: Vec<Task>,
    pub workers: Vec<WorkerInfo>,
    pub config: DaemonConfig,
}

impl AppState {
    pub fn new(config_path: String) -> Self {
        let config = Self::load_config(&config_path);
        Self {
            config_path,
            queue: Vec::new(),
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
}
