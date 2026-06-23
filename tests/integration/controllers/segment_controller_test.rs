//! Controller-level integration tests for segment routes.
//!
//! These tests build an in-memory Axum router and exercise the full
//! handler → use-case path using mock `The1UserRepository` and `The1Client`
//! implementations. All unrelated repositories are no-op stubs.

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use api::{handlers::segments::get_segment, routers::AppState};
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
        the1_user::{The1User, Tier, UpsertTier, UpsertThe1User},
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
// No-op stubs — panic if unexpectedly called in segment tests
// ---------------------------------------------------------------------------

struct NoOpSmsService;
#[async_trait]
impl SmsService for NoOpSmsService {
    async fn send(&self, _phone: &str, _message: &str) -> Result<(), String> { Ok(()) }
}

struct NoOpTokenService;
impl TokenService for NoOpTokenService {
    fn generate(&self, id: Uuid, user: Uuid, _exp: u32) -> Result<String, String> {
        Ok(format!("{id}:{user}"))
    }
    fn validate(&self, token: &str) -> Result<(Uuid, Uuid), String> {
        let mut parts = token.splitn(2, ':');
        let id = parts.next().and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| "bad token".to_string())?;
        let user = parts.next().and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| "bad token".to_string())?;
        Ok((id, user))
    }
}

struct NoOpImageStorage;
#[async_trait]
impl ImageStorage for NoOpImageStorage {
    async fn upload(&self, key: &str, _data: Vec<u8>, _ct: &str) -> Result<String, String> {
        Ok(key.to_string())
    }
    async fn delete(&self, _key: &str) -> Result<(), String> { Ok(()) }
}

struct NoOpUrlSigner;
impl UrlSigner for NoOpUrlSigner {
    fn sign_url(&self, key: &str) -> Result<String, String> {
        Ok(format!("https://cdn.test/{key}"))
    }
}

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

struct NoOpIdentityRepo;
#[async_trait]
impl IdentityRepository for NoOpIdentityRepo {
    async fn find_by_user(&self, _: Uuid) -> Result<Vec<Identity>, RepositoryError> { unimplemented!() }
    async fn find_active(&self, _: Uuid, _: &str, _: &str) -> Result<Option<Identity>, RepositoryError> { unimplemented!() }
    async fn find_deleted(&self, _: &str, _: &str) -> Result<Option<Identity>, RepositoryError> { unimplemented!() }
    async fn create(&self, _: CreateIdentity) -> Result<Identity, RepositoryError> { unimplemented!() }
    async fn restore(&self, _: Uuid, _: Uuid, _: CreateIdentity) -> Result<Identity, RepositoryError> { unimplemented!() }
    async fn soft_delete(&self, _: Uuid, _: Uuid) -> Result<Identity, RepositoryError> { unimplemented!() }
    async fn update_tokens(&self, _: Uuid, _: Option<String>, _: Option<String>) -> Result<Identity, RepositoryError> { unimplemented!() }
    async fn log_transaction(&self, _: Uuid, _: &str, _: &str, _: &str) -> Result<(), RepositoryError> { unimplemented!() }
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

// ---------------------------------------------------------------------------
// Mock: The1UserRepository (upsert-only for segment tests)
// ---------------------------------------------------------------------------

struct MockThe1UserRepo {
    upsert_user: Option<The1User>, // None → Backend error
}

#[async_trait]
impl The1UserRepository for MockThe1UserRepo {
    async fn find_by_user(&self, _: Uuid) -> Result<Option<The1User>, RepositoryError> { unimplemented!() }
    async fn find_by_card_number(&self, _: &str) -> Result<Option<The1User>, RepositoryError> { unimplemented!() }
    async fn find_by_member_id(&self, _: &str) -> Result<Option<The1User>, RepositoryError> { unimplemented!() }

    async fn upsert(&self, user_uuid: Uuid, _: UpsertThe1User) -> Result<The1User, RepositoryError> {
        match &self.upsert_user {
            Some(t) => Ok(The1User {
                id: t.id,
                user_uuid,
                member_id: t.member_id.clone(),
                account_id: t.account_id.clone(),
                profile_id: t.profile_id.clone(),
                card_number: t.card_number.clone(),
                tiers: t.tiers.clone(),
                created_at: t.created_at,
                updated_at: t.updated_at,
            }),
            None => Err(RepositoryError::Backend("mock db error".to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// Mock: The1Client
// ---------------------------------------------------------------------------

struct MockThe1Client {
    partner_data: Option<The1PartnerMemberData>, // None → external error
}

#[async_trait]
impl The1Client for MockThe1Client {
    async fn get_partner_member(&self, card_number: &str) -> Result<The1PartnerMemberData, String> {
        match &self.partner_data {
            Some(d) => Ok(The1PartnerMemberData {
                user_uuid: d.user_uuid,
                member_id: d.member_id.clone(),
                account_id: d.account_id.clone(),
                profile_id: d.profile_id.clone(),
                card_number: Some(card_number.to_string()),
                tiers: d.tiers.clone(),
            }),
            None => Err("external service error".to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
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

fn make_the1_user(user_uuid: Uuid, tiers: Vec<Tier>) -> The1User {
    The1User {
        id: Uuid::new_v4(),
        user_uuid,
        member_id: "MEM001".to_string(),
        account_id: "ACC001".to_string(),
        profile_id: "PRO001".to_string(),
        card_number: Some("1234567890123456".to_string()),
        tiers,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_partner_data(user_uuid: Uuid, tiers: Vec<UpsertTier>) -> The1PartnerMemberData {
    The1PartnerMemberData {
        user_uuid,
        member_id: "MEM001".to_string(),
        account_id: "ACC001".to_string(),
        profile_id: "PRO001".to_string(),
        card_number: Some("1234567890123456".to_string()),
        tiers,
    }
}

fn build_router(
    the1_users: Arc<dyn The1UserRepository>,
    the1_client: Arc<dyn The1Client>,
) -> Router {
    let cfg = test_config();
    let noop_customers: Arc<dyn CustomerRepository> = Arc::new(NoOpCustomerRepo);

    let use_cases = UseCases {
        customers: Arc::new(CustomerUseCases::new(noop_customers.clone(), cfg.clone())),
        identities: Arc::new(IdentityUseCases::new(
            Arc::new(NoOpIdentityRepo),
            noop_customers.clone(),
        )),
        profile_changes: Arc::new(ProfileChangeUseCases::new(
            Arc::new(NoOpProfileChangeRepo),
            noop_customers.clone(),
            cfg.clone(),
            Arc::new(NoOpSmsService),
            Arc::new(NoOpTokenService),
        )),
        profile_images: Arc::new(ProfileImageUseCases::new(
            noop_customers,
            cfg,
            Arc::new(NoOpImageStorage),
            Arc::new(NoOpUrlSigner),
        )),
        segments: Arc::new(SegmentUseCases::new(the1_users.clone(), the1_client)),
        the1: Arc::new(The1UseCases::new(the1_users)),
    };

    let state = Arc::new(AppState { use_cases });

    Router::new()
        .route("/v1/customers/segments", get(get_segment))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_segment_returns_200_with_segment_slug() {
    let user_uuid = Uuid::new_v4();
    let tiers = vec![Tier {
        id: Uuid::new_v4(),
        code: "GOLD".to_string(),
        name: Some("Gold".to_string()),
        expired_date: None,
    }];
    let the1_users = Arc::new(MockThe1UserRepo {
        upsert_user: Some(make_the1_user(user_uuid, tiers)),
    });
    let the1_client = Arc::new(MockThe1Client {
        partner_data: Some(make_partner_data(user_uuid, vec![
            UpsertTier { code: "GOLD".to_string(), name: None, expired_date: None },
        ])),
    });

    let app = build_router(the1_users, the1_client);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/segments?card_number=1234567890123456")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["segment_slug"], "GOLD");
    assert_eq!(json["data"]["user_uuid"], user_uuid.to_string());
}

#[tokio::test]
async fn get_segment_returns_404_when_no_tiers() {
    let user_uuid = Uuid::new_v4();
    let the1_users = Arc::new(MockThe1UserRepo {
        upsert_user: Some(make_the1_user(user_uuid, vec![])), // no tiers
    });
    let the1_client = Arc::new(MockThe1Client {
        partner_data: Some(make_partner_data(user_uuid, vec![])),
    });

    let app = build_router(the1_users, the1_client);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/segments?card_number=1234567890123456")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_segment_returns_502_on_external_service_error() {
    let user_uuid = Uuid::new_v4();
    let the1_users = Arc::new(MockThe1UserRepo {
        upsert_user: Some(make_the1_user(user_uuid, vec![])),
    });
    let the1_client = Arc::new(MockThe1Client { partner_data: None }); // simulate failure

    let app = build_router(the1_users, the1_client);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/segments?card_number=bad_card")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn get_segment_returns_422_when_card_number_missing() {
    let user_uuid = Uuid::new_v4();
    let the1_users = Arc::new(MockThe1UserRepo {
        upsert_user: Some(make_the1_user(user_uuid, vec![])),
    });
    let the1_client = Arc::new(MockThe1Client { partner_data: None });

    let app = build_router(the1_users, the1_client);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/customers/segments") // no card_number query param
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Axum 0.8 returns 400 when required query parameters are missing
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
