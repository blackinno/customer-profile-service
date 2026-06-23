//! Controller-level integration tests for identity routes.
//!
//! These tests build an in-memory Axum router and exercise the full
//! handler → use-case path using a mock `IdentityRepository`. All other
//! repositories are no-op stubs (identity routes never invoke them).

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{delete, get, post},
};
use chrono::Utc;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

use api::{handlers::identities, routers::AppState};
use application::{
    AppConfig, UseCases,
    customers::use_cases::CustomerUseCases,
    identities::use_cases::IdentityUseCases,
    profile_changes::use_cases::{ProfileChangeUseCases, SmsService, TokenService},
    profile_images::use_cases::{ImageStorage, ProfileImageUseCases, UrlSigner},
    segments::use_cases::{SegmentUseCases, The1Client, The1PartnerMemberData},
    the1::use_cases::The1UseCases,
};
use domain::{
    entities::{
        customer::{CreateCustomer, Customer, SearchField, UpdateCustomer},
        identity::{CreateIdentity, Identity},
        profile_change::{ChangeStatus, ChangeType, CreateProfileChange, ProfileChange},
        the1_user::{The1User, UpsertThe1User},
    },
    errors::RepositoryError,
    repositories::{
        customer_repository::CustomerRepository,
        identity_repository::IdentityRepository,
        profile_change_repository::ProfileChangeRepository,
        the1_user_repository::The1UserRepository,
    },
};

// ---------------------------------------------------------------------------
// Shared test helpers
// ---------------------------------------------------------------------------

fn test_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        country_code: "TH".to_string(),
        phone_number_format: "+66".to_string(),
        otp_expired_time: 300,
        otp_text: "OTP: {otp}".to_string(),
        jwt_secret_key: "test-secret".to_string(),
        profile_change_expired_time: 600,
        token_expired_time: 3600,
        allow_image_types: vec!["image/jpeg".to_string()],
        max_image_size_mb: 5,
        image_prefix: "profiles/".to_string(),
        image_expired_in_sec: 86400,
        sns_user_profile_changed: "test-topic".to_string(),
        sns_email_sent_requested: "test-topic-email".to_string(),
        sns_user_identity_linked_changed: "test-topic-identity".to_string(),
        sns_user_the1_get_profile_updated: "test-topic-the1".to_string(),
        s3_profile_bucket: "test-bucket".to_string(),
        cloudfront_base_endpoint: "https://cdn.test".to_string(),
        cloudfront_key_id: "key-id".to_string(),
    })
}

fn make_identity(user: Uuid, provider: &str, ext: &str) -> Identity {
    Identity {
        id: Uuid::new_v4(),
        user_uuid: user,
        provider_name: provider.to_string(),
        external_id: ext.to_string(),
        provider_id_token: None,
        provider_access_token: Some("acc".to_string()),
        provider_refresh_token: Some("ref".to_string()),
        is_deleted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// No-op service stubs for traits added after the test was written
// ---------------------------------------------------------------------------

struct NoOpSmsService;
#[async_trait]
impl SmsService for NoOpSmsService {
    async fn send(&self, _: &str, _: &str) -> Result<(), String> { Ok(()) }
}

struct NoOpTokenService;
impl TokenService for NoOpTokenService {
    fn generate(&self, id: Uuid, user: Uuid, _: u32) -> Result<String, String> {
        Ok(format!("{id}:{user}"))
    }
    fn validate(&self, _: &str) -> Result<(Uuid, Uuid), String> {
        Err("not used in identity tests".to_string())
    }
}

struct NoOpImageStorage;
#[async_trait]
impl ImageStorage for NoOpImageStorage {
    async fn upload(&self, key: &str, _: Vec<u8>, _: &str) -> Result<String, String> { Ok(key.to_string()) }
    async fn delete(&self, _: &str) -> Result<(), String> { Ok(()) }
}

struct NoOpUrlSigner;
impl UrlSigner for NoOpUrlSigner {
    fn sign_url(&self, key: &str) -> Result<String, String> { Ok(format!("https://cdn/{key}")) }
}

struct NoOpThe1Client;
#[async_trait]
impl The1Client for NoOpThe1Client {
    async fn get_partner_member(&self, _: &str) -> Result<The1PartnerMemberData, String> {
        Err("not used in identity tests".to_string())
    }
}

// ---------------------------------------------------------------------------
// No-op repository stubs (panic if called — identity routes never reach them)
// ---------------------------------------------------------------------------

struct NoOpCustomerRepo;
#[async_trait]
impl CustomerRepository for NoOpCustomerRepo {
    async fn create(&self, _: CreateCustomer) -> Result<Customer, RepositoryError> { unimplemented!() }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<Customer>, RepositoryError> { unimplemented!() }
    async fn find_by_phone(&self, _: &str) -> Result<Option<Customer>, RepositoryError> { unimplemented!() }
    async fn find_by_email(&self, _: &str) -> Result<Option<Customer>, RepositoryError> { unimplemented!() }
    async fn search(&self, _: SearchField) -> Result<Vec<Customer>, RepositoryError> { unimplemented!() }
    async fn update(&self, _: Uuid, _: UpdateCustomer) -> Result<Customer, RepositoryError> { unimplemented!() }
    async fn soft_delete(&self, _: Uuid) -> Result<Customer, RepositoryError> { unimplemented!() }
    async fn update_profile_image(&self, _: Uuid, _: Option<String>) -> Result<(), RepositoryError> { unimplemented!() }
}

struct NoOpProfileChangeRepo;
#[async_trait]
impl ProfileChangeRepository for NoOpProfileChangeRepo {
    async fn create(&self, _: CreateProfileChange) -> Result<ProfileChange, RepositoryError> { unimplemented!() }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<ProfileChange>, RepositoryError> { unimplemented!() }
    async fn find_active_by_user_and_type(&self, _: Uuid, _: ChangeType) -> Result<Option<ProfileChange>, RepositoryError> { unimplemented!() }
    async fn update_otp(&self, _: Uuid, _: String, _: String, _: chrono::DateTime<Utc>, _: chrono::DateTime<Utc>) -> Result<ProfileChange, RepositoryError> { unimplemented!() }
    async fn update_status_and_token(&self, _: Uuid, _: ChangeStatus, _: Option<String>, _: Option<chrono::DateTime<Utc>>) -> Result<ProfileChange, RepositoryError> { unimplemented!() }
}

struct NoOpThe1UserRepo;
#[async_trait]
impl The1UserRepository for NoOpThe1UserRepo {
    async fn find_by_user(&self, _: Uuid) -> Result<Option<The1User>, RepositoryError> { unimplemented!() }
    async fn find_by_card_number(&self, _: &str) -> Result<Option<The1User>, RepositoryError> { unimplemented!() }
    async fn find_by_member_id(&self, _: &str) -> Result<Option<The1User>, RepositoryError> { unimplemented!() }
    async fn upsert(&self, _: Uuid, _: UpsertThe1User) -> Result<The1User, RepositoryError> { unimplemented!() }
}

// ---------------------------------------------------------------------------
// Mock IdentityRepository
// ---------------------------------------------------------------------------

struct MockIdentityRepo {
    store: Mutex<Vec<Identity>>,
}

impl MockIdentityRepo {
    fn new(items: Vec<Identity>) -> Arc<Self> {
        Arc::new(Self { store: Mutex::new(items) })
    }
}

#[async_trait]
impl IdentityRepository for MockIdentityRepo {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Vec<Identity>, RepositoryError> {
        Ok(self.store.lock().unwrap().iter()
            .filter(|i| i.user_uuid == user_uuid && !i.is_deleted)
            .cloned().collect())
    }

    async fn find_active(&self, user_uuid: Uuid, provider: &str, external_id: &str) -> Result<Option<Identity>, RepositoryError> {
        Ok(self.store.lock().unwrap().iter()
            .find(|i| i.user_uuid == user_uuid && i.provider_name == provider && i.external_id == external_id && !i.is_deleted)
            .cloned())
    }

    async fn find_deleted(&self, provider: &str, external_id: &str) -> Result<Option<Identity>, RepositoryError> {
        Ok(self.store.lock().unwrap().iter()
            .find(|i| i.provider_name == provider && i.external_id == external_id && i.is_deleted)
            .cloned())
    }

    async fn create(&self, data: CreateIdentity) -> Result<Identity, RepositoryError> {
        let identity = Identity {
            id: Uuid::new_v4(),
            user_uuid: data.user_uuid,
            provider_name: data.provider_name,
            external_id: data.external_id,
            provider_id_token: data.provider_id_token,
            provider_access_token: data.provider_access_token,
            provider_refresh_token: data.provider_refresh_token,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.store.lock().unwrap().push(identity.clone());
        Ok(identity)
    }

    async fn restore(&self, id: Uuid, user_uuid: Uuid, tokens: CreateIdentity) -> Result<Identity, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let entry = store.iter_mut().find(|i| i.id == id)
            .ok_or_else(|| RepositoryError::NotFound("not found".into()))?;
        entry.user_uuid = user_uuid;
        entry.is_deleted = false;
        entry.provider_id_token = tokens.provider_id_token;
        entry.provider_access_token = tokens.provider_access_token;
        entry.provider_refresh_token = tokens.provider_refresh_token;
        Ok(entry.clone())
    }

    async fn soft_delete(&self, id: Uuid, user_uuid: Uuid) -> Result<Identity, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let entry = store.iter_mut().find(|i| i.id == id && i.user_uuid == user_uuid)
            .ok_or_else(|| RepositoryError::NotFound("not found".into()))?;
        entry.is_deleted = true;
        Ok(entry.clone())
    }

    async fn update_tokens(&self, id: Uuid, access_token: Option<String>, refresh_token: Option<String>) -> Result<Identity, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let entry = store.iter_mut().find(|i| i.id == id)
            .ok_or_else(|| RepositoryError::NotFound("not found".into()))?;
        entry.provider_access_token = access_token;
        entry.provider_refresh_token = refresh_token;
        Ok(entry.clone())
    }

    async fn log_transaction(&self, _: Uuid, _: &str, _: &str, _: &str) -> Result<(), RepositoryError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

fn build_router(identity_repo: Arc<MockIdentityRepo>) -> Router {
    let cfg = test_config();

    let identity_uc = IdentityUseCases::new(
        identity_repo,
        Arc::new(NoOpCustomerRepo),
    );

    let use_cases = UseCases {
        customers: Arc::new(CustomerUseCases::new(Arc::new(NoOpCustomerRepo), cfg.clone())),
        identities: Arc::new(identity_uc),
        profile_changes: Arc::new(ProfileChangeUseCases::new(
            Arc::new(NoOpProfileChangeRepo),
            Arc::new(NoOpCustomerRepo),
            cfg.clone(),
            Arc::new(NoOpSmsService),
            Arc::new(NoOpTokenService),
        )),
        profile_images: Arc::new(ProfileImageUseCases::new(
            Arc::new(NoOpCustomerRepo),
            cfg.clone(),
            Arc::new(NoOpImageStorage),
            Arc::new(NoOpUrlSigner),
        )),
        segments: Arc::new(SegmentUseCases::new(
            Arc::new(NoOpThe1UserRepo),
            Arc::new(NoOpThe1Client),
        )),
        the1: Arc::new(The1UseCases::new(Arc::new(NoOpThe1UserRepo))),
    };

    let state = Arc::new(AppState { use_cases });

    Router::new()
        .route(
            "/v1/customers/me/identities",
            get(identities::get_my_identities).post(identities::create_identity),
        )
        .route(
            "/v1/customers/me/identities/{provider}/{external_id}",
            delete(identities::delete_identity),
        )
        .route(
            "/v1/customers/me/identities/{provider_name}/invoke",
            post(identities::invoke_token),
        )
        .route(
            "/v1/customers/{user_uuid}/identities",
            get(identities::get_identities_internal),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_my_identities_returns_200_with_list() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(user, "google", "ext-1")]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/me/identities")
        .header("user_uuid", user.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"].as_array().unwrap().len(), 1);
    assert_eq!(json["data"][0]["provider_name"], "google");
}

#[tokio::test]
async fn get_my_identities_returns_empty_list_for_new_user() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/me/identities")
        .header("user_uuid", user.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn get_my_identities_returns_401_without_user_uuid_header() {
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/me/identities")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_identities_internal_returns_200_without_auth_header() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(user, "the1", "the1-ext")]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/v1/customers/{user}/identities"))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["data"].as_array().unwrap().len(), 1);
    assert_eq!(json["data"][0]["external_id"], "the1-ext");
}

#[tokio::test]
async fn create_identity_returns_200_on_new_link() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let body = serde_json::json!({
        "provider_name": "google",
        "external_id": "google-123",
        "provider_id_token": null,
        "provider_access_token": "acc",
        "provider_refresh_token": "ref"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers/me/identities")
        .header("user_uuid", user.to_string())
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["provider_name"], "google");
    assert_eq!(json["data"]["external_id"], "google-123");
}

#[tokio::test]
async fn create_identity_returns_400_when_already_linked() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(user, "google", "already-linked")]);
    let app = build_router(repo);

    let body = serde_json::json!({
        "provider_name": "google",
        "external_id": "already-linked",
        "provider_id_token": null,
        "provider_access_token": null,
        "provider_refresh_token": null
    });

    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers/me/identities")
        .header("user_uuid", user.to_string())
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_identity_returns_401_without_auth() {
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let body = serde_json::json!({
        "provider_name": "google",
        "external_id": "x",
        "provider_id_token": null,
        "provider_access_token": null,
        "provider_refresh_token": null
    });

    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers/me/identities")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_identity_returns_200_on_success() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(user, "google", "to-delete")]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("DELETE")
        .uri("/v1/customers/me/identities/google/to-delete")
        .header("user_uuid", user.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn delete_identity_returns_404_when_not_found() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("DELETE")
        .uri("/v1/customers/me/identities/google/ghost")
        .header("user_uuid", user.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_identity_returns_401_without_auth() {
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("DELETE")
        .uri("/v1/customers/me/identities/google/x")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invoke_token_returns_200_with_stored_tokens() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(user, "the1", "the1-user")]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers/me/identities/the1/invoke")
        .header("user_uuid", user.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["access_token"], "acc");
    assert_eq!(json["data"]["refresh_token"], "ref");
}

#[tokio::test]
async fn invoke_token_returns_404_when_provider_not_linked() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers/me/identities/the1/invoke")
        .header("user_uuid", user.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invoke_token_returns_401_without_auth() {
    let repo = MockIdentityRepo::new(vec![]);
    let app = build_router(repo);

    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers/me/identities/the1/invoke")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
