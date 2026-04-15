use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;

mod proto {
    tonic::include_proto!("team");
}

mod centy_poller;
mod centy_resolver;
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

/// Build the gRPC bind address from env vars.
/// Extracted as a pure function so both host branches are testable without
/// touching process environment (which is unsafe in Rust 1.85+).
fn resolve_bind_addr(env_host: Option<String>, port: &str) -> String {
    let host = env_host.unwrap_or_else(|| "[::1]".to_string());
    format!("{host}:{port}")
}

/// Resolve the config file path: explicit CONFIG_PATH, or the default under HOME.
/// Extracted as a pure function for the same reason as resolve_bind_addr.
fn resolve_config_path(env_config_path: Option<String>, home: &str) -> String {
    env_config_path.unwrap_or_else(|| format!("{home}/.config/team/config.toml"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("DAEMON_PORT").expect("DAEMON_PORT must be set");
    let addr = resolve_bind_addr(std::env::var("DAEMON_HOST").ok(), &port).parse()?;

    let pid_path = format!("/tmp/team-daemon-{port}.pid");

    // Kill any previous instance
    if let Ok(old_pid) = std::fs::read_to_string(&pid_path) {
        let old_pid = old_pid.trim();
        let _ = std::process::Command::new("kill").arg(old_pid).output();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    std::fs::write(&pid_path, std::process::id().to_string())?;

    let home = std::env::var("HOME").expect("HOME must be set");
    let config_path = resolve_config_path(std::env::var("CONFIG_PATH").ok(), &home);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// DAEMON_HOST set → used as-is; default "[::1]" is not substituted.
    #[test]
    fn resolve_bind_addr_uses_explicit_host() {
        let addr = resolve_bind_addr(Some("0.0.0.0".into()), "50051");
        assert_eq!(addr, "0.0.0.0:50051");
    }

    /// DAEMON_HOST absent → host defaults to "[::1]".
    #[test]
    fn resolve_bind_addr_defaults_host_to_loopback() {
        let addr = resolve_bind_addr(None, "50051");
        assert_eq!(addr, "[::1]:50051");
    }

    /// CONFIG_PATH set → returned as-is; HOME-derived default is not used.
    #[test]
    fn resolve_config_path_uses_explicit_path() {
        let path = resolve_config_path(Some("/etc/team/config.toml".into()), "/home/user");
        assert_eq!(path, "/etc/team/config.toml");
    }

    /// CONFIG_PATH absent → derived from HOME as "{HOME}/.config/team/config.toml".
    #[test]
    fn resolve_config_path_derives_from_home() {
        let path = resolve_config_path(None, "/home/user");
        assert_eq!(path, "/home/user/.config/team/config.toml");
    }
}
