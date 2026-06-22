use crate::{AppFactoryState, configuration::Settings};
use chrono::Duration;
use qml_rs::{
    BackgroundJobServer, PostgresConfig, RetryPolicy, RetryStrategy, ServerConfig, Storage,
    StorageInstance, WorkerRegistry,
};
use std::sync::Arc;

pub async fn create_qml_connection_pool(
    database_url: &str,
) -> Result<Arc<dyn Storage>, qml_rs::StorageError> {
    let config = PostgresConfig::with_defaults().with_database_url(database_url);
    StorageInstance::postgres(config).await
}

/// Build, register workers, and start the QML background-job server.
pub async fn initialize_qml_factory(
    settings: Settings,
    app_state: AppFactoryState,
) -> anyhow::Result<Arc<BackgroundJobServer>> {
    let server_config = ServerConfig::new("default")
        .worker_count(settings.qml_worker_count)
        .polling_interval(Duration::milliseconds(100))
        .fetch_batch_size(settings.qml_batch_size)
        .enable_scheduler(true);

    let retry_policy = RetryPolicy::new(RetryStrategy::exponential_backoff(
        Duration::seconds(settings.qml_retry_base_seconds.into()),
        settings.qml_retry_multiplier,
        Duration::seconds(settings.qml_retry_max_seconds.into()),
        settings.qml_retry_max_attempts,
    ));

    let worker_registry = initialize_qml_worker_registry(app_state.clone());

    let qml_server = Arc::new(BackgroundJobServer::with_retry_policy(
        server_config,
        app_state.storage.clone(),
        Arc::new(worker_registry),
        retry_policy,
    ));

    tracing::info!("Starting QML background job server");
    qml_server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("failed to start QML background job server: {}", e))?;

    Ok(qml_server)
}

pub fn initialize_qml_worker_registry(_app_state: AppFactoryState) -> WorkerRegistry {
    // Workers are registered here once engineers implement their background tasks.
    WorkerRegistry::new()
}
