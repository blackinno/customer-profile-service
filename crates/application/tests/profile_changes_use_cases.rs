use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    entities::{
        customer::{CreateCustomer, Customer, SearchField, UpdateCustomer},
        profile_change::{ChangeStatus, ChangeType, CreateProfileChange, ProfileChange},
    },
    errors::RepositoryError,
    repositories::{
        customer_repository::CustomerRepository,
        profile_change_repository::ProfileChangeRepository,
    },
};
use uuid::Uuid;

use application::config::AppConfig;
use application::errors::ApplicationError;
use application::profile_changes::dtos::{CreateProfileChangeRequest, VerifyProfileChangeRequest};
use application::profile_changes::use_cases::{ProfileChangeUseCases, SmsService, TokenService};

fn make_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        country_code: "TH".to_string(),
        phone_number_format: "+66".to_string(),
        otp_expired_time: 5,
        otp_text: "OTP: {otp}".to_string(),
        jwt_secret_key: "secret".to_string(),
        profile_change_expired_time: 60,
        token_expired_time: 5,
        allow_image_types: vec![],
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

fn make_customer(id: Uuid) -> Customer {
    Customer {
        id,
        email: Some("user@example.com".to_string()),
        phone: Some("+66811111111".to_string()),
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

fn make_profile_change(id: Uuid, user_uuid: Uuid, status: ChangeStatus) -> ProfileChange {
    let now = Utc::now();
    ProfileChange {
        id,
        user_uuid,
        change_type: ChangeType::Email,
        identifier: None,
        old_value: Some("old@example.com".to_string()),
        new_value: Some("new@example.com".to_string()),
        status,
        token: None,
        token_expired_at: now + chrono::Duration::minutes(60),
        otp: Some("123456".to_string()),
        ref_code: Some("ABCDEF".to_string()),
        next_otp_request_at: now - chrono::Duration::seconds(10), // already past
        otp_expired_at: now + chrono::Duration::minutes(5),
        created_at: now,
        updated_at: now,
    }
}

// ── Customer mock ─────────────────────────────────────────────────────────

struct MockCustomerRepo {
    customer: Option<Customer>,
}

#[async_trait]
impl CustomerRepository for MockCustomerRepo {
    async fn create(&self, _: CreateCustomer) -> Result<Customer, RepositoryError> {
        Ok(self.customer.clone().unwrap())
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<Customer>, RepositoryError> {
        Ok(self.customer.clone())
    }
    async fn find_by_phone(&self, _: &str) -> Result<Option<Customer>, RepositoryError> {
        Ok(None)
    }
    async fn find_by_email(&self, _: &str) -> Result<Option<Customer>, RepositoryError> {
        Ok(None)
    }
    async fn search(&self, _: SearchField) -> Result<Vec<Customer>, RepositoryError> {
        Ok(vec![])
    }
    async fn update(&self, _: Uuid, _: UpdateCustomer) -> Result<Customer, RepositoryError> {
        Ok(self.customer.clone().unwrap())
    }
    async fn soft_delete(&self, _: Uuid) -> Result<Customer, RepositoryError> {
        Ok(self.customer.clone().unwrap())
    }
    async fn update_profile_image(&self, _: Uuid, _: Option<String>) -> Result<(), RepositoryError> {
        Ok(())
    }
}

// ── ProfileChange mock ────────────────────────────────────────────────────

struct MockProfileChangeRepo {
    find_by_id: Option<ProfileChange>,
    find_active: Option<ProfileChange>,
    create_result: ProfileChange,
    update_result: ProfileChange,
}

#[async_trait]
impl ProfileChangeRepository for MockProfileChangeRepo {
    async fn create(&self, _: CreateProfileChange) -> Result<ProfileChange, RepositoryError> {
        Ok(self.create_result.clone())
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<ProfileChange>, RepositoryError> {
        Ok(self.find_by_id.clone())
    }
    async fn find_active_by_user_and_type(&self, _: Uuid, _: ChangeType) -> Result<Option<ProfileChange>, RepositoryError> {
        Ok(self.find_active.clone())
    }
    async fn update_otp(&self, _: Uuid, _: String, _: String, _: chrono::DateTime<Utc>, _: chrono::DateTime<Utc>) -> Result<ProfileChange, RepositoryError> {
        Ok(self.update_result.clone())
    }
    async fn update_status_and_token(&self, _: Uuid, _: ChangeStatus, _: Option<String>, _: Option<chrono::DateTime<Utc>>) -> Result<ProfileChange, RepositoryError> {
        Ok(self.update_result.clone())
    }
}

// ── SmsService mock ───────────────────────────────────────────────────────

struct OkSms;
#[async_trait]
impl SmsService for OkSms {
    async fn send(&self, _: &str, _: &str) -> Result<(), String> { Ok(()) }
}

// ── TokenService mock ─────────────────────────────────────────────────────

struct OkToken { valid: bool }
impl TokenService for OkToken {
    fn generate(&self, id: Uuid, user: Uuid, _: u32) -> Result<String, String> {
        Ok(format!("{}.{}", id, user))
    }
    fn validate(&self, token: &str) -> Result<(Uuid, Uuid), String> {
        if !self.valid {
            return Err("invalid token".to_string());
        }
        let parts: Vec<&str> = token.split('.').collect();
        Ok((Uuid::parse_str(parts[0]).unwrap(), Uuid::parse_str(parts[1]).unwrap()))
    }
}

fn make_use_cases(
    pc_repo: MockProfileChangeRepo,
    customer: Option<Customer>,
) -> ProfileChangeUseCases {
    ProfileChangeUseCases::new(
        Arc::new(pc_repo),
        Arc::new(MockCustomerRepo { customer }),
        make_config(),
        Arc::new(OkSms),
        Arc::new(OkToken { valid: true }),
    )
}

// ── create_profile_change ─────────────────────────────────────────────────

#[tokio::test]
async fn create_email_change_returns_pending_record() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let pc = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp);
    let repo = MockProfileChangeRepo {
        find_by_id: None,
        find_active: None,
        create_result: pc,
        update_result: make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp),
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let req = CreateProfileChangeRequest {
        change_type: "email".to_string(),
        new_value: "new@example.com".to_string(),
    };
    let res = uc.create_profile_change(user_uuid, req).await.unwrap();
    assert_eq!(res.status, "pending_verify_otp");
}

#[tokio::test]
async fn create_telephone_change_returns_pending_record() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let mut pc = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp);
    pc.change_type = ChangeType::Telephone;
    let repo = MockProfileChangeRepo {
        find_by_id: None,
        find_active: None,
        create_result: pc.clone(),
        update_result: pc,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let req = CreateProfileChangeRequest {
        change_type: "telephone".to_string(),
        new_value: "0899999999".to_string(),
    };
    let res = uc.create_profile_change(user_uuid, req).await.unwrap();
    assert_eq!(res.status, "pending_verify_otp");
}

#[tokio::test]
async fn create_rejects_duplicate_active_change() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let active = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp);
    let repo = MockProfileChangeRepo {
        find_by_id: None,
        find_active: Some(active.clone()),
        create_result: active.clone(),
        update_result: active,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let req = CreateProfileChangeRequest {
        change_type: "email".to_string(),
        new_value: "another@example.com".to_string(),
    };
    let err = uc.create_profile_change(user_uuid, req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

// ── verify_profile_change ─────────────────────────────────────────────────

#[tokio::test]
async fn verify_correct_otp_succeeds() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let pc = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp);
    let mut updated = pc.clone();
    updated.status = ChangeStatus::VerifyChangeCompleted;
    let repo = MockProfileChangeRepo {
        find_by_id: Some(pc),
        find_active: None,
        create_result: updated.clone(),
        update_result: updated,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let req = VerifyProfileChangeRequest { otp: "123456".to_string() };
    let res = uc.verify_profile_change(user_uuid, pc_id, req).await.unwrap();
    assert_eq!(res.status, "verify_change_completed");
}

#[tokio::test]
async fn verify_wrong_otp_returns_bad_request() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let pc = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp);
    let repo = MockProfileChangeRepo {
        find_by_id: Some(pc.clone()),
        find_active: None,
        create_result: pc.clone(),
        update_result: pc,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let req = VerifyProfileChangeRequest { otp: "000000".to_string() }; // wrong OTP
    let err = uc.verify_profile_change(user_uuid, pc_id, req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn verify_expired_otp_returns_bad_request() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let mut pc = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp);
    pc.otp_expired_at = Utc::now() - chrono::Duration::minutes(10); // already expired
    let repo = MockProfileChangeRepo {
        find_by_id: Some(pc.clone()),
        find_active: None,
        create_result: pc.clone(),
        update_result: pc,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let req = VerifyProfileChangeRequest { otp: "123456".to_string() };
    let err = uc.verify_profile_change(user_uuid, pc_id, req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

// ── confirm_profile_change ────────────────────────────────────────────────

#[tokio::test]
async fn confirm_wrong_status_returns_bad_request() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let pc = make_profile_change(pc_id, user_uuid, ChangeStatus::PendingVerifyOtp); // not VerifyChangeCompleted
    let repo = MockProfileChangeRepo {
        find_by_id: Some(pc.clone()),
        find_active: None,
        create_result: pc.clone(),
        update_result: pc,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let err = uc.confirm_profile_change(user_uuid, pc_id).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn confirm_expired_token_returns_bad_request() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let mut pc = make_profile_change(pc_id, user_uuid, ChangeStatus::VerifyChangeCompleted);
    pc.token_expired_at = Utc::now() - chrono::Duration::minutes(10); // token expired
    let repo = MockProfileChangeRepo {
        find_by_id: Some(pc.clone()),
        find_active: None,
        create_result: pc.clone(),
        update_result: pc,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let err = uc.confirm_profile_change(user_uuid, pc_id).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn confirm_succeeds_with_valid_state() {
    let user_uuid = Uuid::new_v4();
    let pc_id = Uuid::new_v4();
    let pc = make_profile_change(pc_id, user_uuid, ChangeStatus::VerifyChangeCompleted);
    let mut completed = pc.clone();
    completed.status = ChangeStatus::Completed;
    let repo = MockProfileChangeRepo {
        find_by_id: Some(pc),
        find_active: None,
        create_result: completed.clone(),
        update_result: completed,
    };
    let uc = make_use_cases(repo, Some(make_customer(user_uuid)));
    let res = uc.confirm_profile_change(user_uuid, pc_id).await.unwrap();
    assert_eq!(res.status, "completed");
}
