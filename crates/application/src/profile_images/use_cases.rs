use std::sync::Arc;

use async_trait::async_trait;
use domain::repositories::customer_repository::CustomerRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApplicationError;
use crate::profile_images::dtos::ProfileImageResponse;

/// Abstraction over an object-storage backend (e.g. S3).
///
/// Defined here — in the application layer — so use cases never import from
/// the infrastructure crate. The concrete implementation (`S3Storage`) lives in
/// `infrastructure::storage::s3` and is injected at startup.
#[async_trait]
pub trait ImageStorage: Send + Sync {
    /// Upload `data` with `content_type` at the given `key`.
    /// Returns the stored key (may differ if the backend adds its own prefix).
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<String, String>;

    /// Delete the object stored at `key`.
    async fn delete(&self, key: &str) -> Result<(), String>;
}

/// Abstraction over a CDN URL-signing service (e.g. CloudFront).
///
/// Defined here for the same reason as `ImageStorage`: the application layer
/// must stay free of infrastructure imports.
pub trait UrlSigner: Send + Sync {
    /// Return a time-limited signed URL for the object at `object_key`.
    fn sign_url(&self, object_key: &str) -> Result<String, String>;
}

pub struct ProfileImageUseCases {
    customers: Arc<dyn CustomerRepository>,
    config: Arc<AppConfig>,
    storage: Arc<dyn ImageStorage>,
    signer: Arc<dyn UrlSigner>,
}

impl ProfileImageUseCases {
    pub fn new(
        customers: Arc<dyn CustomerRepository>,
        config: Arc<AppConfig>,
        storage: Arc<dyn ImageStorage>,
        signer: Arc<dyn UrlSigner>,
    ) -> Self {
        Self {
            customers,
            config,
            storage,
            signer,
        }
    }

    /// Validate, upload, and persist a customer's profile image.
    ///
    /// Validates content type and size, stores the object under a stable key
    /// (`<image_prefix>/<user_uuid>`), updates the customer record, and returns
    /// a short-lived signed CDN URL.
    pub async fn upload(
        &self,
        user_uuid: Uuid,
        data: Vec<u8>,
        content_type: String,
        file_size: usize,
    ) -> Result<ProfileImageResponse, ApplicationError> {
        // 1. Validate content type against the allowlist
        if !self.config.allow_image_types.contains(&content_type) {
            return Err(ApplicationError::BadRequest(
                "unsupported image type".to_string(),
            ));
        }

        // 2. Validate file size (config specifies limit in megabytes)
        let max_bytes = self.config.max_image_size_mb as usize * 1024 * 1024;
        if file_size > max_bytes {
            return Err(ApplicationError::BadRequest("image too large".to_string()));
        }

        // 3. Derive a stable, collision-free storage key
        let key = format!("{}/{}", self.config.image_prefix, user_uuid);

        // 4. Persist the object in object storage
        self.storage
            .upload(&key, data, &content_type)
            .await
            .map_err(ApplicationError::External)?;

        // 5. Record the image key on the customer profile
        self.customers
            .update_profile_image(user_uuid, Some(key.clone()))
            .await?;

        // 6. Generate a time-limited signed CDN URL for the caller
        let signed_url = self
            .signer
            .sign_url(&key)
            .map_err(ApplicationError::External)?;

        Ok(ProfileImageResponse { url: signed_url })
    }

    /// Return a signed CDN URL for the customer's current profile image.
    pub async fn get_image(
        &self,
        user_uuid: Uuid,
    ) -> Result<ProfileImageResponse, ApplicationError> {
        let customer =
            self.customers.find_by_id(user_uuid).await?.ok_or_else(|| {
                ApplicationError::NotFound(format!("customer {user_uuid} not found"))
            })?;

        let profile = customer
            .profile
            .ok_or_else(|| ApplicationError::NotFound("profile image not found".to_string()))?;

        let key = profile
            .profile_image
            .ok_or_else(|| ApplicationError::NotFound("profile image not found".to_string()))?;

        let signed_url = self
            .signer
            .sign_url(&key)
            .map_err(ApplicationError::External)?;

        Ok(ProfileImageResponse { url: signed_url })
    }

    /// Delete the customer's profile image from object storage and clear the DB reference.
    pub async fn delete_image(&self, user_uuid: Uuid) -> Result<(), ApplicationError> {
        let customer =
            self.customers.find_by_id(user_uuid).await?.ok_or_else(|| {
                ApplicationError::NotFound(format!("customer {user_uuid} not found"))
            })?;

        let profile = customer
            .profile
            .ok_or_else(|| ApplicationError::NotFound("profile image not found".to_string()))?;

        let key = profile
            .profile_image
            .ok_or_else(|| ApplicationError::NotFound("profile image not found".to_string()))?;

        self.storage
            .delete(&key)
            .await
            .map_err(ApplicationError::External)?;

        self.customers.update_profile_image(user_uuid, None).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use domain::{
        entities::customer::{CreateCustomer, Customer, CustomerProfile, SearchField, UpdateCustomer},
        errors::RepositoryError,
        repositories::customer_repository::CustomerRepository,
    };

    fn make_config() -> Arc<AppConfig> {
        Arc::new(AppConfig {
            country_code: "TH".to_string(),
            phone_number_format: "+66".to_string(),
            otp_expired_time: 5,
            otp_text: "".to_string(),
            jwt_secret_key: "secret".to_string(),
            profile_change_expired_time: 60,
            token_expired_time: 5,
            allow_image_types: vec!["image/jpeg".to_string(), "image/png".to_string()],
            max_image_size_mb: 2,
            image_prefix: "profiles".to_string(),
            image_expired_in_sec: 3600,
            sns_user_profile_changed: "".to_string(),
            sns_email_sent_requested: "".to_string(),
            sns_user_identity_linked_changed: "".to_string(),
            sns_user_the1_get_profile_updated: "".to_string(),
            s3_profile_bucket: "bucket".to_string(),
            cloudfront_base_endpoint: "https://cdn.example.com".to_string(),
            cloudfront_key_id: "key-id".to_string(),
        })
    }

    fn make_customer_with_image(id: Uuid, image_key: Option<String>) -> Customer {
        Customer {
            id,
            email: None,
            phone: None,
            email_verified: false,
            phone_verified: false,
            locale: domain::entities::customer::Locale::En,
            has_consent: true,
            is_deleted: false,
            client_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            profile: Some(CustomerProfile {
                id: Uuid::new_v4(),
                user_uuid: id,
                first_name: None,
                last_name: None,
                birthdate: None,
                gender: None,
                profile_image: image_key,
                nationality: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }),
        }
    }

    struct MockCustomerRepo { customer: Option<Customer> }

    #[async_trait]
    impl CustomerRepository for MockCustomerRepo {
        async fn create(&self, _: CreateCustomer) -> Result<Customer, RepositoryError> { Ok(self.customer.clone().unwrap()) }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<Customer>, RepositoryError> { Ok(self.customer.clone()) }
        async fn find_by_phone(&self, _: &str) -> Result<Option<Customer>, RepositoryError> { Ok(None) }
        async fn find_by_email(&self, _: &str) -> Result<Option<Customer>, RepositoryError> { Ok(None) }
        async fn search(&self, _: SearchField) -> Result<Vec<Customer>, RepositoryError> { Ok(vec![]) }
        async fn update(&self, _: Uuid, _: UpdateCustomer) -> Result<Customer, RepositoryError> { Ok(self.customer.clone().unwrap()) }
        async fn soft_delete(&self, _: Uuid) -> Result<Customer, RepositoryError> { Ok(self.customer.clone().unwrap()) }
        async fn update_profile_image(&self, _: Uuid, _: Option<String>) -> Result<(), RepositoryError> { Ok(()) }
    }

    struct MockStorage { fail: bool }
    #[async_trait]
    impl ImageStorage for MockStorage {
        async fn upload(&self, key: &str, _: Vec<u8>, _: &str) -> Result<String, String> {
            if self.fail { Err("s3 error".to_string()) } else { Ok(key.to_string()) }
        }
        async fn delete(&self, _: &str) -> Result<(), String> {
            if self.fail { Err("s3 error".to_string()) } else { Ok(()) }
        }
    }

    struct MockSigner;
    impl UrlSigner for MockSigner {
        fn sign_url(&self, key: &str) -> Result<String, String> {
            Ok(format!("https://cdn.example.com/{}?signature=abc", key))
        }
    }

    fn use_cases(customer: Option<Customer>, storage_fail: bool) -> ProfileImageUseCases {
        ProfileImageUseCases::new(
            Arc::new(MockCustomerRepo { customer }),
            make_config(),
            Arc::new(MockStorage { fail: storage_fail }),
            Arc::new(MockSigner),
        )
    }

    // ── upload ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upload_returns_signed_url() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, None)), false);
        let res = uc.upload(user_uuid, vec![0u8; 100], "image/jpeg".to_string(), 100).await.unwrap();
        assert!(res.url.contains("cdn.example.com"));
    }

    #[tokio::test]
    async fn upload_rejects_unsupported_content_type() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, None)), false);
        let err = uc.upload(user_uuid, vec![], "application/pdf".to_string(), 0).await.unwrap_err();
        assert!(matches!(err, ApplicationError::BadRequest(_)));
    }

    #[tokio::test]
    async fn upload_rejects_oversized_file() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, None)), false);
        let size = 3 * 1024 * 1024; // 3 MB, limit is 2 MB
        let err = uc.upload(user_uuid, vec![0u8; size], "image/jpeg".to_string(), size).await.unwrap_err();
        assert!(matches!(err, ApplicationError::BadRequest(_)));
    }

    // ── get_image ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_image_returns_signed_url() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, Some("profiles/uuid".to_string()))), false);
        let res = uc.get_image(user_uuid).await.unwrap();
        assert!(res.url.contains("profiles/uuid"));
    }

    #[tokio::test]
    async fn get_image_customer_not_found() {
        let uc = use_cases(None, false);
        let err = uc.get_image(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, ApplicationError::NotFound(_)));
    }

    #[tokio::test]
    async fn get_image_no_image_key_returns_not_found() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, None)), false);
        let err = uc.get_image(user_uuid).await.unwrap_err();
        assert!(matches!(err, ApplicationError::NotFound(_)));
    }

    // ── delete_image ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_image_succeeds() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, Some("profiles/uuid".to_string()))), false);
        assert!(uc.delete_image(user_uuid).await.is_ok());
    }

    #[tokio::test]
    async fn delete_image_no_image_returns_not_found() {
        let user_uuid = Uuid::new_v4();
        let uc = use_cases(Some(make_customer_with_image(user_uuid, None)), false);
        let err = uc.delete_image(user_uuid).await.unwrap_err();
        assert!(matches!(err, ApplicationError::NotFound(_)));
    }
}
