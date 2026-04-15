use app::App;
use color_eyre::Result;

mod app;
mod client;
mod ui;

/// Build the daemon gRPC address from the three env vars.
/// Extracted as a pure function so all four branches are testable without
/// touching process environment (which is unsafe in Rust 1.85+).
fn daemon_addr_from(
    env_addr: Option<String>,
    env_port: Option<String>,
    env_host: Option<String>,
) -> String {
    if let Some(a) = env_addr {
        format!("http://{a}")
    } else {
        let port = env_port.unwrap_or_else(|| "50051".to_string());
        let host = env_host.unwrap_or_else(|| "[::1]".to_string());
        format!("http://{host}:{port}")
    }
}

fn daemon_addr() -> String {
    daemon_addr_from(
        std::env::var("DAEMON_ADDR").ok(),
        std::env::var("DAEMON_PORT").ok(),
        std::env::var("DAEMON_HOST").ok(),
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Load .env from the project root (one level up from tui/)
    let _ = dotenvy::from_path("../.env");

    let mut app = App::new(daemon_addr())?;
    app.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DAEMON_ADDR set → returned as-is with http:// prefix; port/host ignored.
    #[test]
    fn daemon_addr_uses_explicit_addr() {
        let addr = daemon_addr_from(
            Some("192.168.1.50:9090".into()),
            Some("99999".into()),
            Some("ignored-host".into()),
        );
        assert_eq!(addr, "http://192.168.1.50:9090");
    }

    /// All three absent → defaults: http://[::1]:50051.
    #[test]
    fn daemon_addr_defaults_to_loopback_50051() {
        let addr = daemon_addr_from(None, None, None);
        assert_eq!(addr, "http://[::1]:50051");
    }

    /// DAEMON_PORT set, DAEMON_HOST absent → host defaults to [::1].
    #[test]
    fn daemon_addr_uses_port_with_default_host() {
        let addr = daemon_addr_from(None, Some("12345".into()), None);
        assert_eq!(addr, "http://[::1]:12345");
    }

    /// DAEMON_HOST set, DAEMON_PORT absent → port defaults to 50051.
    #[test]
    fn daemon_addr_uses_host_with_default_port() {
        let addr = daemon_addr_from(None, None, Some("10.0.0.1".into()));
        assert_eq!(addr, "http://10.0.0.1:50051");
    }
}
