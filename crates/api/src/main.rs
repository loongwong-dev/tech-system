//! API 进程入口。

use api::{AppState, build_router};
use db::Db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let db = Db::from_env()?;
    let app = build_router(AppState::new(db));
    let address = std::env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(%address, "api listening");

    axum::serve(listener, app).await?;
    Ok(())
}
