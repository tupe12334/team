use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;

mod proto {
    tonic::include_proto!("team");
}

mod centy_poller;
mod gstack_agents;
mod issue_ref_json;
mod link_resolver;
mod services;
mod state;
mod worker_pool;

use proto::{
    agent_service_server::AgentServiceServer, daemon_service_server::DaemonServiceServer,
    queue_service_server::QueueServiceServer, worker_service_server::WorkerServiceServer,
};
use services::agent::AgentServiceImpl;
use services::daemon::DaemonServiceImpl;
use services::queue::QueueServiceImpl;
use services::worker::WorkerServiceImpl;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("DAEMON_PORT").expect("DAEMON_PORT must be set");
    let addr = format!("[::1]:{port}").parse()?;

    let pid_path = format!("/tmp/team-daemon-{port}.pid");

    // Kill any previous instance
    if let Ok(old_pid) = std::fs::read_to_string(&pid_path) {
        let old_pid = old_pid.trim();
        let _ = std::process::Command::new("kill").arg(old_pid).output();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    std::fs::write(&pid_path, std::process::id().to_string())?;

    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME must be set");
        format!("{home}/.config/team/config.toml")
    });
    let shared_state = Arc::new(Mutex::new(AppState::new(config_path)));

    worker_pool::start(shared_state.clone());
    centy_poller::start(shared_state.clone());

    println!("Daemon listening on {addr}");

    Server::builder()
        .add_service(AgentServiceServer::new(AgentServiceImpl::new(
            shared_state.clone(),
        )))
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
