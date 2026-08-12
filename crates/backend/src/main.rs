use akasha_backend::{Config, build_app};
use anyhow::{Context, Result};
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, fmt};

/// 启动 HTTP 服务器并等待操作系统关闭信号
#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load().context("failed to load configuration")?;

    let filter = EnvFilter::try_new(&config.log_level).context("invalid LOG_LEVEL")?;
    fmt().with_env_filter(filter).init();

    let bind_addr = config.bind_addr;

    // 先完成数据库和路由初始化，避免未就绪时接收 worker 请求
    let app = build_app(config)
        .await
        .context("failed to build application")?;

    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server failed")
}

/// 等待 Ctrl+C 或 SIGTERM 后允许服务器优雅关闭
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = signal::ctrl_c().await {
            tracing::error!(?error, "failed to listen for ctrl-c");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(?error, "failed to listen for SIGTERM");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl-c received"),
        _ = terminate => tracing::info!("SIGTERM received"),
    }
}
