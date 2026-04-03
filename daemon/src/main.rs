use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;

mod proto {
    tonic::include_proto!("team");
}

mod services;
mod state;

use proto::{
    daemon_service_server::DaemonServiceServer, queue_service_server::QueueServiceServer,
    worker_service_server::WorkerServiceServer,
};
use services::daemon::DaemonServiceImpl;
use services::queue::QueueServiceImpl;
use services::worker::WorkerServiceImpl;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("DAEMON_PORT").expect("DAEMON_PORT must be set");
    let addr = format!("[::1]:{port}").parse()?;

    let shared_state = Arc::new(Mutex::new(AppState::new(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/etc/team/config.toml".to_string()),
    )));

    println!("Daemon listening on {addr}");

    Server::builder()
        .add_service(DaemonServiceServer::new(DaemonServiceImpl::new(
            shared_state.clone(),
        )))
        .add_service(QueueServiceServer::new(QueueServiceImpl::new(
            shared_state.clone(),
        )))
        .add_service(WorkerServiceServer::new(WorkerServiceImpl::new(
            shared_state.clone(),
        )))
        .serve(addr)
        .await?;

    Ok(())
}
