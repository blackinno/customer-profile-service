use std::sync::Arc;

use application::config::AppConfig;
use application::customers::dtos::{CreateCustomerRequest, SearchCustomerQuery, UpdateCustomerRequest};
use application::customers::use_cases::CustomerUseCases;
use application::errors::ApplicationError;
use async_trait::async_trait;
use chrono::Utc;
use domain::{
    entities::customer::{CreateCustomer, Customer, SearchField, UpdateCustomer},
    errors::RepositoryError,
    repositories::customer_repository::CustomerRepository,
};
use uuid::Uuid;

fn make_customer(id: Uuid) -> Customer {
    Customer {
        id,
        email: Some("test@example.com".to_string()),
        phone: Some("+66812345678".to_string()),
        email_verified: false,
        phone_verified: false,
        locale: domain::entities::customer::Locale::En,
        has_consent: true,
        is_deleted: false,
        client_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        profile: None,
    }
}

fn make_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        country_code: "TH".to_string(),
        phone_number_format: "+66".to_string(),
        otp_expired_time: 5,
        otp_text: "OTP: {otp}".to_string(),
        jwt_secret_key: "secret".to_string(),
        profile_change_expired_time: 60,
        token_expired_time: 5,
        allow_image_types: vec!["image/jpeg".to_string()],
        max_image_size_mb: 5,
        image_prefix: "profiles".to_string(),
        image_expired_in_sec: 3600,
        sns_user_profile_changed: "arn:sns:profile-changed".to_string(),
        sns_email_sent_requested: "arn:sns:email-sent".to_string(),
        sns_user_identity_linked_changed: "arn:sns:identity".to_string(),
        sns_user_the1_get_profile_updated: "arn:sns:the1".to_string(),
        s3_profile_bucket: "bucket".to_string(),
        cloudfront_base_endpoint: "https://cdn.example.com".to_string(),
        cloudfront_key_id: "key-id".to_string(),
    })
}

struct MockCustomerRepo {
    find_by_id: Option<Customer>,
    find_by_email: Option<Customer>,
    find_by_phone: Option<Customer>,
    create_result: Customer,
    search_result: Vec<Customer>,
    update_result: Customer,
    soft_delete_result: Customer,
}

impl MockCustomerRepo {
    fn new(id: Uuid) -> Self {
        let c = make_customer(id);
        Self {
            find_by_id: Some(c.clone()),
            find_by_email: None,
            find_by_phone: None,
            create_result: c.clone(),
            search_result: vec![c.clone()],
            update_result: c.clone(),
            soft_delete_result: c,
        }
    }
}

#[async_trait]
impl CustomerRepository for MockCustomerRepo {
    async fn create(&self, _: CreateCustomer) -> Result<Customer, RepositoryError> {
        Ok(self.create_result.clone())
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<Customer>, RepositoryError> {
        Ok(self.find_by_id.clone())
    }
    async fn find_by_phone(&self, _: &str) -> Result<Option<Customer>, RepositoryError> {
        Ok(self.find_by_phone.clone())
    }
    async fn find_by_email(&self, _: &str) -> Result<Option<Customer>, RepositoryError> {
        Ok(self.find_by_email.clone())
    }
    async fn search(&self, _: SearchField) -> Result<Vec<Customer>, RepositoryError> {
        Ok(self.search_result.clone())
    }
    async fn update(&self, _: Uuid, _: UpdateCustomer) -> Result<Customer, RepositoryError> {
        Ok(self.update_result.clone())
    }
    async fn soft_delete(&self, _: Uuid) -> Result<Customer, RepositoryError> {
        Ok(self.soft_delete_result.clone())
    }
    async fn update_profile_image(&self, _: Uuid, _: Option<String>) -> Result<(), RepositoryError> {
        Ok(())
    }
}

fn use_cases(repo: MockCustomerRepo) -> CustomerUseCases {
    CustomerUseCases::new(Arc::new(repo), make_config())
}

// ── create ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_returns_customer_response() {
    let id = Uuid::new_v4();
    let uc = use_cases(MockCustomerRepo::new(id));
    let req = CreateCustomerRequest {
        email: Some("new@example.com".to_string()),
        phone: None,
        locale: Some("en".to_string()),
        has_consent: Some(true),
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };
    let res = uc.create(req).await.unwrap();
    assert_eq!(res.id, id.to_string());
}

#[tokio::test]
async fn create_rejects_duplicate_email() {
    let id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    mock.find_by_email = Some(make_customer(id)); // email already exists
    let uc = use_cases(mock);
    let req = CreateCustomerRequest {
        email: Some("existing@example.com".to_string()),
        phone: None,
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };
    let err = uc.create(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn create_rejects_duplicate_phone() {
    let id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    mock.find_by_phone = Some(make_customer(id));
    let uc = use_cases(mock);
    let req = CreateCustomerRequest {
        email: None,
        phone: Some("0812345678".to_string()),
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };
    let err = uc.create(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn create_normalises_phone_number() {
    let id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    // No phone conflict — phone is free
    mock.find_by_phone = None;
    let mut created = make_customer(id);
    created.phone = Some("+66812345678".to_string());
    mock.create_result = created;
    let uc = use_cases(mock);
    let req = CreateCustomerRequest {
        email: None,
        phone: Some("0812345678".to_string()),
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };
    let res = uc.create(req).await.unwrap();
    assert_eq!(res.phone.unwrap(), "+66812345678");
}

// ── get_by_id ────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_by_id_returns_customer() {
    let id = Uuid::new_v4();
    let uc = use_cases(MockCustomerRepo::new(id));
    let res = uc.get_by_id(id).await.unwrap();
    assert_eq!(res.id, id.to_string());
}

#[tokio::test]
async fn get_by_id_not_found() {
    let id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    mock.find_by_id = None;
    let uc = use_cases(mock);
    let err = uc.get_by_id(id).await.unwrap_err();
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

#[tokio::test]
async fn get_by_id_deleted_returns_not_found() {
    let id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    let mut deleted = make_customer(id);
    deleted.is_deleted = true;
    mock.find_by_id = Some(deleted);
    let uc = use_cases(mock);
    let err = uc.get_by_id(id).await.unwrap_err();
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

// ── search ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_by_phone_returns_results() {
    let id = Uuid::new_v4();
    let uc = use_cases(MockCustomerRepo::new(id));
    let query = SearchCustomerQuery {
        id: None,
        phone: Some("+66812345678".to_string()),
        the1_member_id: None,
        the1_card_number: None,
    };
    let results = uc.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn search_without_params_returns_bad_request() {
    let id = Uuid::new_v4();
    let uc = use_cases(MockCustomerRepo::new(id));
    let query = SearchCustomerQuery {
        id: None,
        phone: None,
        the1_member_id: None,
        the1_card_number: None,
    };
    let err = uc.search(query).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn search_with_invalid_uuid_returns_bad_request() {
    let id = Uuid::new_v4();
    let uc = use_cases(MockCustomerRepo::new(id));
    let query = SearchCustomerQuery {
        id: Some("not-a-uuid".to_string()),
        phone: None,
        the1_member_id: None,
        the1_card_number: None,
    };
    let err = uc.search(query).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

// ── delete ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_returns_customer_response() {
    let id = Uuid::new_v4();
    let uc = use_cases(MockCustomerRepo::new(id));
    let res = uc.delete(id).await.unwrap();
    assert_eq!(res.id, id.to_string());
}

// ── update_me ────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_me_rejects_email_owned_by_other_user() {
    let id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    // find_by_email returns a *different* customer → conflict
    mock.find_by_email = Some(make_customer(other_id));
    let uc = use_cases(mock);
    let req = UpdateCustomerRequest {
        email: Some("taken@example.com".to_string()),
        locale: None,
        has_consent: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };
    let err = uc.update_me(id, req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn update_me_allows_same_email_for_same_user() {
    let id = Uuid::new_v4();
    let mut mock = MockCustomerRepo::new(id);
    // find_by_email returns the *same* customer — re-submitting own email is fine
    mock.find_by_email = Some(make_customer(id));
    let uc = use_cases(mock);
    let req = UpdateCustomerRequest {
        email: Some("test@example.com".to_string()),
        locale: None,
        has_consent: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };
    assert!(uc.update_me(id, req).await.is_ok());
}
