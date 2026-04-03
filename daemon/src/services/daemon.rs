use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::proto::daemon_service_server::DaemonService;
use crate::proto::{DaemonInfo, ReloadConfigResponse};
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
    ) -> Result<Response<DaemonInfo>, Status> {
        let state = self.state.lock().await;
        let info = DaemonInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.start_time.elapsed().as_secs() as i64,
            config_path: state.config_path.clone(),
            workers_count: state.workers.len() as i32,
        };
        Ok(Response::new(info))
    }

    async fn shutdown(
        &self,
        _request: Request<()>,
    ) -> Result<Response<()>, Status> {
        tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            std::process::exit(0);
        });
        Ok(Response::new(()))
    }

    async fn reload_config(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ReloadConfigResponse>, Status> {
        // Placeholder: real reload logic goes here
        Ok(Response::new(ReloadConfigResponse {
            success: true,
            error: String::new(),
        }))
    }
}
