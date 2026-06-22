#[cfg(feature = "sns")]
use crate::{AwsSns, Message, configuration::initialize_aws_sns};
use crate::{
    QmlEventDispatcher,
    configuration::{Settings, create_connection_pool, create_qml_connection_pool, migration},
};
use application::{Repositories, UseCases};
#[cfg(feature = "sns")]
use aws_sdk_sns::Client;
use domain::events::EventDispatcher;
use qml_rs::Storage;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct InitialAppFactory {
    #[cfg(feature = "sns")]
    pub client: Client,
    pub db_pool: PgPool,
    pub storage: Arc<dyn Storage>,
    pub settings: Settings,
}

impl InitialAppFactory {
    pub async fn new(settings: Settings) -> anyhow::Result<Self> {
        let pool = create_connection_pool(&settings.database_url).await?;
        tracing::info!("Database connection pool created");

        migration(&pool).await?;
        tracing::info!("Database migrations applied");

        let storage = create_qml_connection_pool(&settings.qml_database_url).await?;
        tracing::info!("QML storage instance created");

        #[cfg(feature = "sns")]
        let client = {
            let c = initialize_aws_sns(settings.aws_region.clone()).await;
            tracing::info!("AWS SNS client initialized");
            c
        };

        Ok(Self {
            #[cfg(feature = "sns")]
            client,
            db_pool: pool,
            storage,
            settings,
        })
    }
}

#[derive(Clone)]
pub struct AppFactoryState {
    pub pool: PgPool,
    pub storage: Arc<dyn Storage>,
    pub repos: Repositories,
    pub use_cases: UseCases,
    #[cfg(feature = "sns")]
    pub message: Message,
}

impl AppFactoryState {
    pub fn new(_factory: InitialAppFactory) -> Self {
        // TODO (Task 8 - tech-lead wiring): instantiate all PgRepositories,
        // external clients, and use-case structs here once all engineers are done.
        // This stub allows the binary to compile during parallel development.
        todo!("wire all domains — done in Task 8 after all engineers complete")
    }
}
