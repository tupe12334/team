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
            workers_count: state.workers.len() as i32,
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
            });
        let mut state = self.state.lock().await;
        state.config = new_config.clone();
        Ok(Response::new(UpdateConfigResponse {
            result: Some(update_config_response::Result::Ok(new_config)),
        }))
    }

    async fn reload_config(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ReloadConfigResponse>, Status> {
        Ok(Response::new(ReloadConfigResponse {
            result: Some(reload_config_response::Result::Ok(())),
        }))
    }
}
