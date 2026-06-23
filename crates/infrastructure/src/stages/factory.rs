#[cfg(feature = "sns")]
use crate::{AwsSns, Message, configuration::initialize_aws_sns};
use crate::{
    configuration::{Settings, create_connection_pool, create_qml_connection_pool, migration},
    external::{sms_client::SmsClient, the1_client::The1HttpClient},
    persistence::{
        pg_customer_repository::PgCustomerRepository, pg_identity_repository::PgIdentityRepository,
        pg_profile_change_repository::PgProfileChangeRepository,
        pg_the1_user_repository::PgThe1UserRepository,
    },
    storage::{cloudfront_signer::CloudFrontSigner, s3::S3Storage},
    utils::jwt::JwtTokenService,
};
use application::{
    AppConfig, Repositories, UseCases, customers::use_cases::CustomerUseCases,
    identities::use_cases::IdentityUseCases, profile_changes::use_cases::ProfileChangeUseCases,
    profile_images::use_cases::ProfileImageUseCases, segments::use_cases::SegmentUseCases,
    the1::use_cases::The1UseCases,
};
use aws_config::{BehaviorVersion, Region};
#[cfg(feature = "sns")]
use aws_sdk_sns::Client;
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
        // Synchronous wiring is fine — all heavy async work (DB pool, migrations,
        // QML storage, AWS clients) was already done in `InitialAppFactory::new`.
        // `AppFactoryState::new` is only called once, at startup.
        panic!("AppFactoryState::new requires async context — call AppFactoryState::build instead")
    }

    /// Wire all repositories, external clients, and use-case structs.
    /// Called once at startup after `InitialAppFactory::new` completes.
    pub async fn build(factory: InitialAppFactory) -> anyhow::Result<Self> {
        let settings = &factory.settings;
        let pool = factory.db_pool.clone();

        // ---- AppConfig ----
        let config = Arc::new(AppConfig::from(settings));

        // ---- Repositories ----
        let customers: Arc<dyn domain::repositories::customer_repository::CustomerRepository> =
            Arc::new(PgCustomerRepository::new(pool.clone()));
        let identities: Arc<dyn domain::repositories::identity_repository::IdentityRepository> =
            Arc::new(PgIdentityRepository::new(pool.clone()));
        let profile_changes: Arc<
            dyn domain::repositories::profile_change_repository::ProfileChangeRepository,
        > = Arc::new(PgProfileChangeRepository::new(pool.clone()));
        let the1_users: Arc<dyn domain::repositories::the1_user_repository::The1UserRepository> =
            Arc::new(PgThe1UserRepository::new(pool.clone()));

        let repos = Repositories {
            customers: customers.clone(),
            identities: identities.clone(),
            profile_changes: profile_changes.clone(),
            the1_users: the1_users.clone(),
        };

        // ---- External clients ----
        let the1_client: Arc<dyn application::segments::use_cases::The1Client> =
            Arc::new(The1HttpClient::new(settings.the1_proxy_service_url.clone()));

        let sms: Arc<dyn application::profile_changes::use_cases::SmsService> =
            Arc::new(SmsClient::new(settings.sms_proxy_service_url.clone()));

        let token_service: Arc<dyn application::profile_changes::use_cases::TokenService> =
            Arc::new(JwtTokenService::new(settings.jwt_secret_key.clone()));

        // ---- AWS S3 client ----
        let aws_cfg = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(settings.aws_region.clone()))
            .load()
            .await;
        let s3_client = aws_sdk_s3::Client::new(&aws_cfg);

        let storage_backend: Arc<dyn application::profile_images::use_cases::ImageStorage> =
            Arc::new(S3Storage::new(
                s3_client,
                settings.s3_profile_bucket.clone(),
                settings.image_prefix.clone(),
            ));

        let signer: Arc<dyn application::profile_images::use_cases::UrlSigner> =
            Arc::new(CloudFrontSigner::new(
                &settings.cloudfront_private_key,
                settings.cloudfront_key_id.clone(),
                settings.cloudfront_base_endpoint.clone(),
                settings.image_expired_in_sec,
            )?);

        // ---- Use cases ----
        let customer_uc = Arc::new(CustomerUseCases::new(customers.clone(), config.clone()));
        let identity_uc = Arc::new(IdentityUseCases::new(identities));
        let profile_change_uc = Arc::new(ProfileChangeUseCases::new(
            profile_changes,
            customers.clone(),
            config.clone(),
            sms,
            token_service,
        ));
        let profile_image_uc = Arc::new(ProfileImageUseCases::new(
            customers,
            config,
            storage_backend,
            signer,
        ));
        let segment_uc = Arc::new(SegmentUseCases::new(the1_users.clone(), the1_client));
        let the1_uc = Arc::new(The1UseCases::new(the1_users));

        let use_cases = UseCases {
            customers: customer_uc,
            identities: identity_uc,
            profile_changes: profile_change_uc,
            profile_images: profile_image_uc,
            segments: segment_uc,
            the1: the1_uc,
        };

        #[cfg(feature = "sns")]
        let message = {
            Message {
                sns: std::sync::Arc::new(AwsSns::new(factory.client)),
            }
        };

        Ok(Self {
            pool,
            storage: factory.storage,
            repos,
            use_cases,
            #[cfg(feature = "sns")]
            message,
        })
    }
}
