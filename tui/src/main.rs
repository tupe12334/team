use app::App;
use color_eyre::Result;

mod app;
mod client;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let port = std::env::var("DAEMON_PORT").unwrap_or_else(|_| "50051".to_string());
    let addr = format!("http://[::1]:{port}");

    let mut app = App::new(addr).await?;
    app.run().await
}
