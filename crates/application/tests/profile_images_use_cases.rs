use std::sync::Arc;

use application::config::AppConfig;
use application::errors::ApplicationError;
use application::profile_images::use_cases::{ImageStorage, ProfileImageUseCases, UrlSigner};
use async_trait::async_trait;
use chrono::Utc;
use domain::{
    entities::customer::{CreateCustomer, Customer, CustomerProfile, SearchField, UpdateCustomer},
    errors::RepositoryError,
    repositories::customer_repository::CustomerRepository,
};
use uuid::Uuid;

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
