use api::middleware::install_metrics_recorder;
use api::routers::Routers;
use infrastructure::configuration::{Settings, initialize_qml_factory};
use infrastructure::{AppFactoryState, InitialAppFactory};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    // Install the global Prometheus recorder before any metrics! macros fire.
    let metrics_handle = install_metrics_recorder()?;

    let settings = Settings::from_env()?;

    let factory = InitialAppFactory::new(settings.clone()).await?;
    let app_stage = AppFactoryState::build(factory).await?;

    sqlx::migrate!("./migrations").run(&app_stage.pool).await?;
    tracing::info!("Database migrations applied");
    tracing::info!("Application state initialized");

    // Start QML workers. The server holds its own internal task handles —
    // call `stop().await` (not `abort()`) to drain workers cleanly.
    let qml_server = initialize_qml_factory(settings.clone(), app_stage.clone()).await?;

    let app = Routers::init_routers(app_stage, metrics_handle);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", settings.server_host, settings.server_port))
            .await?;
    tracing::info!(
        host = %settings.server_host,
        port = settings.server_port,
        "Server listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("HTTP server stopped; draining background workers");
    if let Err(e) = qml_server.stop().await {
        tracing::error!("QML graceful stop failed: {}", e);
    }

    Ok(())
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Switch to JSON logs in non-dev environments by setting LOG_FORMAT=json.
    let json_logs = std::env::var("LOG_FORMAT").as_deref() == Ok("json");

    let registry = tracing_subscriber::registry().with(env_filter);

    if json_logs {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C, shutting down"),
        _ = terminate => tracing::info!("Received SIGTERM, shutting down"),
    }
}
