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
