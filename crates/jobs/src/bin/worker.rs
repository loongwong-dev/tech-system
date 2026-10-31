//! worker 进程入口。
//!
//! 负责信号处理与配置读取；队列循环在 [`jobs::Worker`]。

use db::Db;
use jobs::{Handlers, Worker, WorkerConfig};
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let db = Db::from_env()?;
    let config = WorkerConfig::from_env()?;
    let handlers = Handlers::new();

    tracing::info!(
        worker_id = %config.worker_id,
        poll_seconds = config.poll_interval.as_secs(),
        lease_seconds = config.lease.as_secs(),
        "worker 启动"
    );
    if handlers.is_empty() {
        tracing::warn!("尚未注册业务 handler，未知任务会被记为失败");
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });

    Worker::new(db.admin().pool().clone(), config, handlers)
        .run(shutdown_rx)
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let terminate = async {
            match signal(SignalKind::terminate()) {
                Ok(mut stream) => {
                    stream.recv().await;
                }
                Err(error) => {
                    tracing::error!(%error, "无法监听 SIGTERM，仅响应 SIGINT");
                    std::future::pending::<()>().await;
                }
            }
        };

        tokio::select! {
            () = async {
                let _ = tokio::signal::ctrl_c().await;
            } => {}
            () = terminate => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
