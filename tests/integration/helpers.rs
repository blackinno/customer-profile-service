use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use tower::ServiceExt;
use uuid::Uuid;

use api::{customers::controller::*, routers::AppState};
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
        customer::{
            CreateCustomer, Customer, CustomerProfile, Locale, SearchField, UpdateCustomer,
        },
        identity::{CreateIdentity, Identity},
        profile_change::{ChangeStatus, ChangeType, CreateProfileChange, ProfileChange},
        the1_user::{The1User, UpsertThe1User},
    },
    errors::RepositoryError,
    repositories::{
        customer_repository::CustomerRepository, identity_repository::IdentityRepository,
        profile_change_repository::ProfileChangeRepository,
        the1_user_repository::The1UserRepository,
    },
};

// ============================================================
// In-memory customer repository used by both helpers and tests
// ============================================================

pub struct InMemoryCustomerRepository {
    pub store: Arc<Mutex<HashMap<Uuid, Customer>>>,
}

impl InMemoryCustomerRepository {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_customer(c: Customer) -> Self {
        let mut map = HashMap::new();
        map.insert(c.id, c);
        Self {
            store: Arc::new(Mutex::new(map)),
        }
    }
}

#[async_trait]
impl CustomerRepository for InMemoryCustomerRepository {
    async fn create(&self, data: CreateCustomer) -> Result<Customer, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let customer = Customer {
            id,
            email: data.email,
            phone: data.phone,
            email_verified: false,
            phone_verified: false,
            locale: data.locale.unwrap_or(Locale::Th),
            has_consent: data.has_consent.unwrap_or(false),
            is_deleted: false,
            client_id: data.client_id,
            created_at: now,
            updated_at: now,
            profile: Some(CustomerProfile {
                id: Uuid::new_v4(),
                user_uuid: id,
                first_name: data.first_name,
                last_name: data.last_name,
                birthdate: data.birthdate,
                gender: data.gender,
                profile_image: None,
                nationality: data.nationality,
                created_at: now,
                updated_at: now,
            }),
        };
        self.store.lock().unwrap().insert(id, customer.clone());
        Ok(customer)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, RepositoryError> {
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_phone(&self, phone: &str) -> Result<Option<Customer>, RepositoryError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|c| c.phone.as_deref() == Some(phone))
            .cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Customer>, RepositoryError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|c| c.email.as_deref() == Some(email))
            .cloned())
    }

    async fn search(&self, field: SearchField) -> Result<Vec<Customer>, RepositoryError> {
        let store = self.store.lock().unwrap();
        let results: Vec<Customer> = match field {
            SearchField::Id(id) => store.values().filter(|c| c.id == id).cloned().collect(),
            SearchField::Phone(phone) => store
                .values()
                .filter(|c| c.phone.as_deref() == Some(phone.as_str()))
                .cloned()
                .collect(),
            SearchField::The1MemberId(_) | SearchField::The1CardNumber(_) => vec![],
        };
        Ok(results)
    }

    async fn update(&self, id: Uuid, data: UpdateCustomer) -> Result<Customer, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let customer = store
            .get_mut(&id)
            .ok_or_else(|| RepositoryError::NotFound(format!("customer {} not found", id)))?;

        if let Some(v) = data.email {
            customer.email = Some(v);
        }
        if let Some(v) = data.phone {
            customer.phone = Some(v);
        }
        if let Some(v) = data.locale {
            customer.locale = v;
        }
        if let Some(v) = data.has_consent {
            customer.has_consent = v;
        }
        if let Some(profile) = customer.profile.as_mut() {
            if let Some(v) = data.first_name {
                profile.first_name = Some(v);
            }
            if let Some(v) = data.last_name {
                profile.last_name = Some(v);
            }
            if let Some(v) = data.birthdate {
                profile.birthdate = Some(v);
            }
            if let Some(v) = data.gender {
                profile.gender = Some(v);
            }
            if let Some(v) = data.nationality {
                profile.nationality = Some(v);
            }
        }
        customer.updated_at = Utc::now();
        Ok(customer.clone())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<Customer, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let customer = store
            .get_mut(&id)
            .ok_or_else(|| RepositoryError::NotFound(format!("customer {} not found", id)))?;
        let new_phone = customer
            .phone
            .as_ref()
            .map(|p| format!("{}-deleted-{}", p, id));
        let new_email = customer
            .email
            .as_ref()
            .map(|e| format!("{}-deleted-{}", e, id));
        customer.is_deleted = true;
        customer.phone = new_phone;
        customer.email = new_email;
        customer.updated_at = Utc::now();
        Ok(customer.clone())
    }

    async fn update_profile_image(
        &self,
        user_uuid: Uuid,
        image_key: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut store = self.store.lock().unwrap();
        if let Some(c) = store.get_mut(&user_uuid)
            && let Some(p) = c.profile.as_mut()
        {
            p.profile_image = image_key;
        }
        Ok(())
    }
}

// ============================================================
// Stub repositories for the other use-case domains
// (methods are unreachable — only customer endpoints are tested)
// ============================================================

struct StubIdentityRepository;

#[async_trait]
impl IdentityRepository for StubIdentityRepository {
    async fn find_by_user(&self, _: Uuid) -> Result<Vec<Identity>, RepositoryError> {
        unreachable!("not called in customer integration tests")
    }
    async fn find_active(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        unreachable!()
    }
    async fn find_deleted(&self, _: &str, _: &str) -> Result<Option<Identity>, RepositoryError> {
        unreachable!()
    }
    async fn create(&self, _: CreateIdentity) -> Result<Identity, RepositoryError> {
        unreachable!()
    }
    async fn restore(
        &self,
        _: Uuid,
        _: Uuid,
        _: CreateIdentity,
    ) -> Result<Identity, RepositoryError> {
        unreachable!()
    }
    async fn soft_delete(&self, _: Uuid, _: Uuid) -> Result<Identity, RepositoryError> {
        unreachable!()
    }
    async fn update_tokens(
        &self,
        _: Uuid,
        _: Option<String>,
        _: Option<String>,
    ) -> Result<Identity, RepositoryError> {
        unreachable!()
    }
    async fn log_transaction(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), RepositoryError> {
        unreachable!()
    }
}

struct StubProfileChangeRepository;

#[async_trait]
impl ProfileChangeRepository for StubProfileChangeRepository {
    async fn create(&self, _: CreateProfileChange) -> Result<ProfileChange, RepositoryError> {
        unreachable!("not called in customer integration tests")
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<ProfileChange>, RepositoryError> {
        unreachable!()
    }
    async fn find_active_by_user_and_type(
        &self,
        _: Uuid,
        _: ChangeType,
    ) -> Result<Option<ProfileChange>, RepositoryError> {
        unreachable!()
    }
    async fn update_otp(
        &self,
        _: Uuid,
        _: String,
        _: String,
        _: chrono::DateTime<Utc>,
        _: chrono::DateTime<Utc>,
    ) -> Result<ProfileChange, RepositoryError> {
        unreachable!()
    }
    async fn update_status_and_token(
        &self,
        _: Uuid,
        _: ChangeStatus,
        _: Option<String>,
        _: Option<chrono::DateTime<Utc>>,
    ) -> Result<ProfileChange, RepositoryError> {
        unreachable!()
    }
}

struct StubThe1UserRepository;

#[async_trait]
impl The1UserRepository for StubThe1UserRepository {
    async fn find_by_user(&self, _: Uuid) -> Result<Option<The1User>, RepositoryError> {
        unreachable!("not called in customer integration tests")
    }
    async fn find_by_card_number(&self, _: &str) -> Result<Option<The1User>, RepositoryError> {
        unreachable!()
    }
    async fn find_by_member_id(&self, _: &str) -> Result<Option<The1User>, RepositoryError> {
        unreachable!()
    }
    async fn upsert(&self, _: Uuid, _: UpsertThe1User) -> Result<The1User, RepositoryError> {
        unreachable!()
    }
}

// ============================================================
// Stub external-service implementations (panic-free no-ops)
// ============================================================

struct StubSmsService;

#[async_trait]
impl SmsService for StubSmsService {
    async fn send(&self, _phone: &str, _message: &str) -> Result<(), String> {
        Ok(())
    }
}

struct StubTokenService;

impl TokenService for StubTokenService {
    fn generate(&self, id: Uuid, user: Uuid, _exp: u32) -> Result<String, String> {
        Ok(format!("{id}:{user}"))
    }
    fn validate(&self, token: &str) -> Result<(Uuid, Uuid), String> {
        let mut parts = token.splitn(2, ':');
        let id = parts
            .next()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| "bad token".to_string())?;
        let user = parts
            .next()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| "bad token".to_string())?;
        Ok((id, user))
    }
}

struct StubImageStorage;

#[async_trait]
impl ImageStorage for StubImageStorage {
    async fn upload(&self, key: &str, _data: Vec<u8>, _ct: &str) -> Result<String, String> {
        Ok(key.to_string())
    }
    async fn delete(&self, _key: &str) -> Result<(), String> {
        Ok(())
    }
}

struct StubUrlSigner;

impl UrlSigner for StubUrlSigner {
    fn sign_url(&self, key: &str) -> Result<String, String> {
        Ok(format!("https://cdn.test.com/{key}"))
    }
}

struct StubThe1Client;

#[async_trait]
impl The1Client for StubThe1Client {
    async fn get_partner_member(
        &self,
        _card_number: &str,
    ) -> Result<The1PartnerMemberData, String> {
        Err("stub — not used in customer integration tests".to_string())
    }
}

// ============================================================
// Public test helpers
// ============================================================

pub fn test_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        country_code: "66".to_string(),
        phone_number_format: "+66".to_string(),
        otp_expired_time: 300,
        otp_text: "OTP: {otp}".to_string(),
        jwt_secret_key: "test_secret_key".to_string(),
        profile_change_expired_time: 3600,
        token_expired_time: 86400,
        allow_image_types: vec!["image/jpeg".to_string(), "image/png".to_string()],
        max_image_size_mb: 5,
        image_prefix: "profiles/".to_string(),
        image_expired_in_sec: 3600,
        sns_user_profile_changed: "test-topic".to_string(),
        sns_email_sent_requested: "test-topic".to_string(),
        sns_user_identity_linked_changed: "test-topic".to_string(),
        sns_user_the1_get_profile_updated: "test-topic".to_string(),
        s3_profile_bucket: "test-bucket".to_string(),
        cloudfront_base_endpoint: "https://cdn.test.com".to_string(),
        cloudfront_key_id: "test-key".to_string(),
    })
}

/// Build a minimal Axum router wired with the customer routes and an in-memory
/// customer repository. Other use-case repositories are stubs that panic if called.
pub fn create_test_app(customers: Arc<InMemoryCustomerRepository>) -> Router {
    let config = test_config();
    let customers_dyn: Arc<dyn CustomerRepository> = customers;

    let use_cases = UseCases {
        customers: Arc::new(CustomerUseCases::new(customers_dyn.clone(), config.clone())),
        identities: Arc::new(IdentityUseCases::new(Arc::new(StubIdentityRepository))),
        profile_changes: Arc::new(ProfileChangeUseCases::new(
            Arc::new(StubProfileChangeRepository),
            customers_dyn.clone(),
            config.clone(),
            Arc::new(StubSmsService),
            Arc::new(StubTokenService),
        )),
        profile_images: Arc::new(ProfileImageUseCases::new(
            customers_dyn.clone(),
            config.clone(),
            Arc::new(StubImageStorage),
            Arc::new(StubUrlSigner),
        )),
        segments: Arc::new(SegmentUseCases::new(
            Arc::new(StubThe1UserRepository),
            Arc::new(StubThe1Client),
        )),
        the1: Arc::new(The1UseCases::new(Arc::new(StubThe1UserRepository))),
    };

    let state = Arc::new(AppState { use_cases });

    // Literal `/customers/me` is registered before the parameterised
    // `/customers/{id}` so that Axum's router resolves it as a literal.
    Router::new()
        .route("/customers", post(create_customer).get(search_customers))
        .route("/customers/me", get(get_me).put(update_me))
        .route(
            "/customers/{id}",
            get(get_customer_by_id).delete(delete_customer),
        )
        .with_state(state)
}

/// Drive a single HTTP request through the router and return the status code
/// together with the parsed JSON response body.
pub async fn send_request(app: Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    (status, json)
}
