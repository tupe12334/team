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
        let new_config = request
            .into_inner()
            .config
            .unwrap_or_else(|| DaemonConfig {
                workers_count: 4,
                log_level: "info".to_string(),
                enabled_agents: Vec::new(),
            });
        let mut state = self.state.lock().await;
        state.config = new_config.clone();
        if let Err(e) = state.save_config() {
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
        state.reload_config();
        Ok(Response::new(ReloadConfigResponse {
            result: Some(reload_config_response::Result::Ok(())),
        }))
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
    async fn reload_config_returns_ok() {
        let svc = DaemonServiceImpl::new(make_state(4));
        let res = svc.reload_config(Request::new(())).await.unwrap().into_inner();
        assert!(matches!(res.result.unwrap(), reload_config_response::Result::Ok(())));
    }
}
