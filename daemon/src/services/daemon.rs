use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::proto::daemon_service_server::DaemonService;
use crate::proto::{
    get_config_response, get_info_response, reload_config_response, shutdown_response,
    update_config_response, DaemonConfig, DaemonInfo, GetConfigResponse, GetInfoResponse,
    ReloadConfigResponse, ShutdownResponse, UpdateConfigRequest, UpdateConfigResponse,
};
use crate::state::AppState;

pub struct DaemonServiceImpl {
    state: Arc<Mutex<AppState>>,
    start_time: Instant,
}

impl DaemonServiceImpl {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self {
            state,
            start_time: Instant::now(),
        }
    }
}

#[tonic::async_trait]
impl DaemonService for DaemonServiceImpl {
    async fn get_info(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GetInfoResponse>, Status> {
        let state = self.state.lock().await;
        let info = DaemonInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.start_time.elapsed().as_secs() as i64,
            config_path: state.config_path.clone(),
            workers_count: state.config.workers_count,
        };
        Ok(Response::new(GetInfoResponse {
            result: Some(get_info_response::Result::Ok(info)),
        }))
    }

    async fn shutdown(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ShutdownResponse>, Status> {
        tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            std::process::exit(0);
        });
        Ok(Response::new(ShutdownResponse {
            result: Some(shutdown_response::Result::Ok(())),
        }))
    }

    async fn get_config(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GetConfigResponse>, Status> {
        let state = self.state.lock().await;
        Ok(Response::new(GetConfigResponse {
            result: Some(get_config_response::Result::Ok(state.config.clone())),
        }))
    }

    async fn update_config(
        &self,
        request: Request<UpdateConfigRequest>,
    ) -> Result<Response<UpdateConfigResponse>, Status> {
        let mut new_config = request
            .into_inner()
            .config
            .unwrap_or_else(|| DaemonConfig {
                workers_count: 4,
                log_level: "info".to_string(),
                enabled_agents: Vec::new(),
            });
        if new_config.workers_count < 1 {
            return Ok(Response::new(UpdateConfigResponse {
                result: Some(update_config_response::Result::Error(
                    "workers_count must be at least 1".to_string(),
                )),
            }));
        }
        const VALID_LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];
        if !VALID_LOG_LEVELS.contains(&new_config.log_level.as_str()) {
            return Ok(Response::new(UpdateConfigResponse {
                result: Some(update_config_response::Result::Error(format!(
                    "invalid log_level '{}'; valid values: {}",
                    new_config.log_level,
                    VALID_LOG_LEVELS.join(", ")
                ))),
            }));
        }
        for agent_name in &new_config.enabled_agents {
            if !crate::gstack_agents::is_known(agent_name) {
                return Ok(Response::new(UpdateConfigResponse {
                    result: Some(update_config_response::Result::Error(
                        format!("unknown agent '{agent_name}'; use GetAvailableAgents to list valid agents"),
                    )),
                }));
            }
        }
        // Deduplicate enabled_agents so the stored list is canonical.
        let mut seen = std::collections::HashSet::new();
        new_config.enabled_agents.retain(|a| seen.insert(a.clone()));
        let mut state = self.state.lock().await;
        let old_config = state.config.clone();
        state.config = new_config.clone();
        if let Err(e) = state.save_config() {
            state.config = old_config; // roll back so memory and disk stay consistent
            return Err(Status::internal(format!("Failed to save config: {e}")));
        }
        Ok(Response::new(UpdateConfigResponse {
            result: Some(update_config_response::Result::Ok(new_config)),
        }))
    }

    async fn reload_config(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ReloadConfigResponse>, Status> {
        let mut state = self.state.lock().await;
        match state.reload_config() {
            Ok(()) => Ok(Response::new(ReloadConfigResponse {
                result: Some(reload_config_response::Result::Ok(())),
            })),
            Err(msg) => Ok(Response::new(ReloadConfigResponse {
                result: Some(reload_config_response::Result::Error(msg)),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::daemon_service_server::DaemonService;

    fn make_state(workers_count: i32) -> Arc<Mutex<crate::state::AppState>> {
        Arc::new(Mutex::new(crate::state::AppState {
            config_path: "/tmp/daemon-svc-test.toml".into(),
            queue_path: "/tmp/daemon-svc-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig {
                workers_count,
                log_level: "info".into(),
                enabled_agents: vec![],
            },
        }))
    }

    #[tokio::test]
    async fn get_info_returns_version_and_workers_count() {
        let svc = DaemonServiceImpl::new(make_state(4));
        let res = svc.get_info(Request::new(())).await.unwrap().into_inner();
        let info = match res.result.unwrap() {
            get_info_response::Result::Ok(i) => i,
            get_info_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert!(!info.version.is_empty());
        assert_eq!(info.workers_count, 4);
        assert!(info.uptime_seconds >= 0);
    }

    #[tokio::test]
    async fn get_config_returns_current_config() {
        let svc = DaemonServiceImpl::new(make_state(2));
        let res = svc.get_config(Request::new(())).await.unwrap().into_inner();
        let config = match res.result.unwrap() {
            get_config_response::Result::Ok(c) => c,
            get_config_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(config.workers_count, 2);
        assert_eq!(config.log_level, "info");
    }

    #[tokio::test]
    async fn update_config_persists_new_values() {
        let state = make_state(1);
        let svc = DaemonServiceImpl::new(state.clone());
        let new_config = DaemonConfig {
            workers_count: 8,
            log_level: "debug".into(),
            enabled_agents: vec!["review".into()],
        };
        let req = Request::new(UpdateConfigRequest { config: Some(new_config.clone()) });
        let res = svc.update_config(req).await.unwrap().into_inner();
        let updated = match res.result.unwrap() {
            update_config_response::Result::Ok(c) => c,
            update_config_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(updated.workers_count, 8);
        assert_eq!(updated.log_level, "debug");
        assert_eq!(updated.enabled_agents, vec!["review"]);
        // State should also reflect the update
        let locked = state.lock().await;
        assert_eq!(locked.config.workers_count, 8);
    }

    #[tokio::test]
    async fn update_config_with_no_config_uses_defaults() {
        let svc = DaemonServiceImpl::new(make_state(1));
        let req = Request::new(UpdateConfigRequest { config: None });
        let res = svc.update_config(req).await.unwrap().into_inner();
        let updated = match res.result.unwrap() {
            update_config_response::Result::Ok(c) => c,
            update_config_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(updated.workers_count, 4);
        assert_eq!(updated.log_level, "info");
    }

    #[tokio::test]
    async fn update_config_rejects_zero_workers_count() {
        let svc = DaemonServiceImpl::new(make_state(4));
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig { workers_count: 0, log_level: "info".into(), enabled_agents: vec![] }),
        });
        let res = svc.update_config(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_config_response::Result::Error(msg) => {
                assert!(msg.contains("workers_count"), "expected validation error, got: {msg}");
            }
            update_config_response::Result::Ok(_) => panic!("expected error but got ok"),
        }
    }

    #[tokio::test]
    async fn update_config_rejects_negative_workers_count() {
        let svc = DaemonServiceImpl::new(make_state(4));
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig { workers_count: -1, log_level: "info".into(), enabled_agents: vec![] }),
        });
        let res = svc.update_config(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_config_response::Result::Error(msg) => {
                assert!(msg.contains("workers_count"), "expected validation error, got: {msg}");
            }
            update_config_response::Result::Ok(_) => panic!("expected error but got ok"),
        }
    }

    #[tokio::test]
    async fn update_config_rolls_back_on_save_failure() {
        // Use a path that cannot be written to force save_config to fail.
        let state = Arc::new(Mutex::new(crate::state::AppState {
            config_path: "/nonexistent-dir/cannot-write.toml".into(),
            queue_path: "/tmp/rollback-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 2, log_level: "info".into(), enabled_agents: vec![] },
        }));
        let svc = DaemonServiceImpl::new(state.clone());
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig { workers_count: 8, log_level: "debug".into(), enabled_agents: vec![] }),
        });
        let res = svc.update_config(req).await;
        // The call must return a gRPC error (Status::internal) because save failed.
        assert!(res.is_err(), "expected internal error when save_config fails");
        // In-memory config must be rolled back to original values.
        let locked = state.lock().await;
        assert_eq!(locked.config.workers_count, 2, "rolled back workers_count");
        assert_eq!(locked.config.log_level, "info", "rolled back log_level");
    }

    #[tokio::test]
    async fn reload_config_returns_ok() {
        let svc = DaemonServiceImpl::new(make_state(4));
        let res = svc.reload_config(Request::new(())).await.unwrap().into_inner();
        assert!(matches!(res.result.unwrap(), reload_config_response::Result::Ok(())));
    }

    #[tokio::test]
    async fn reload_config_returns_error_for_malformed_toml() {
        let path = format!("/tmp/daemon-svc-reload-malformed-{}.toml", uuid::Uuid::new_v4());
        std::fs::write(&path, "this is [[[not valid toml").unwrap();
        let state = Arc::new(Mutex::new(crate::state::AppState {
            config_path: path.clone(),
            queue_path: "/tmp/daemon-svc-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig { workers_count: 3, log_level: "warn".into(), enabled_agents: vec![] },
        }));
        let svc = DaemonServiceImpl::new(state.clone());
        let res = svc.reload_config(Request::new(())).await.unwrap().into_inner();
        match res.result.unwrap() {
            reload_config_response::Result::Error(msg) => {
                assert!(msg.contains("malformed config"), "expected malformed-config error, got: {msg}");
            }
            reload_config_response::Result::Ok(_) => panic!("expected error for malformed TOML"),
        }
        // Config must be unchanged — workers_count still 3
        let locked = state.lock().await;
        assert_eq!(locked.config.workers_count, 3, "config must not change on failed reload");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn update_config_rejects_unknown_enabled_agent() {
        let state = make_state(4);
        let svc = DaemonServiceImpl::new(state.clone());
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig {
                workers_count: 4,
                log_level: "info".into(),
                enabled_agents: vec!["nonexistent-agent".into()],
            }),
        });
        let res = svc.update_config(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_config_response::Result::Error(msg) => {
                assert!(msg.contains("nonexistent-agent"), "error should name the bad agent, got: {msg}");
                assert!(msg.contains("GetAvailableAgents"), "error should point to remedy, got: {msg}");
            }
            update_config_response::Result::Ok(_) => panic!("expected error for unknown agent"),
        }
        // State must be unchanged
        let locked = state.lock().await;
        assert!(locked.config.enabled_agents.is_empty());
    }

    #[tokio::test]
    async fn update_config_accepts_known_enabled_agent() {
        let state = make_state(4);
        let svc = DaemonServiceImpl::new(state.clone());
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig {
                workers_count: 4,
                log_level: "info".into(),
                enabled_agents: vec!["review".into(), "qa".into()],
            }),
        });
        let res = svc.update_config(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_config_response::Result::Ok(c) => {
                assert_eq!(c.enabled_agents, vec!["review", "qa"]);
            }
            update_config_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    #[tokio::test]
    async fn update_config_rejects_invalid_log_level() {
        let svc = DaemonServiceImpl::new(make_state(4));
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig {
                workers_count: 4,
                log_level: "verbose".into(),
                enabled_agents: vec![],
            }),
        });
        let res = svc.update_config(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_config_response::Result::Error(msg) => {
                assert!(msg.contains("log_level"), "expected log_level error, got: {msg}");
                assert!(msg.contains("verbose"), "error should name the bad value, got: {msg}");
            }
            update_config_response::Result::Ok(_) => panic!("expected error for invalid log_level"),
        }
    }

    #[tokio::test]
    async fn update_config_accepts_valid_log_levels() {
        for level in &["error", "warn", "info", "debug", "trace"] {
            let svc = DaemonServiceImpl::new(make_state(4));
            let req = Request::new(UpdateConfigRequest {
                config: Some(DaemonConfig {
                    workers_count: 4,
                    log_level: (*level).into(),
                    enabled_agents: vec![],
                }),
            });
            let res = svc.update_config(req).await.unwrap().into_inner();
            match res.result.unwrap() {
                update_config_response::Result::Ok(c) => {
                    assert_eq!(c.log_level, *level);
                }
                update_config_response::Result::Error(e) => {
                    panic!("unexpected error for log_level '{level}': {e}");
                }
            }
        }
    }

    #[tokio::test]
    async fn update_config_deduplicates_enabled_agents() {
        let state = make_state(4);
        let svc = DaemonServiceImpl::new(state.clone());
        let req = Request::new(UpdateConfigRequest {
            config: Some(DaemonConfig {
                workers_count: 4,
                log_level: "info".into(),
                enabled_agents: vec!["review".into(), "qa".into(), "review".into()],
            }),
        });
        let res = svc.update_config(req).await.unwrap().into_inner();
        match res.result.unwrap() {
            update_config_response::Result::Ok(c) => {
                assert_eq!(c.enabled_agents, vec!["review", "qa"], "duplicates must be removed");
            }
            update_config_response::Result::Error(e) => panic!("unexpected error: {e}"),
        }
        // Persisted state must also be deduplicated
        let locked = state.lock().await;
        assert_eq!(locked.config.enabled_agents, vec!["review", "qa"]);
    }
}
