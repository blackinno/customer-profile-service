use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use domain::entities::customer::UpdateCustomer;
use domain::entities::profile_change::{ChangeStatus, ChangeType, CreateProfileChange};
use domain::repositories::customer_repository::CustomerRepository;
use domain::repositories::profile_change_repository::ProfileChangeRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApplicationError;
use crate::events::{EmailSentRequestedPayload, NoopPublisher, ProfileChangedPayload, Publisher};
use crate::profile_changes::dtos::{
    CreateProfileChangeRequest, ProfileChangeResponse, UpdateProfileChangeRequest,
    VerifyProfileChangeRequest,
};

/// Abstraction over the SMS delivery backend.
/// Concrete implementation lives in the infrastructure crate (`SmsClient`).
#[async_trait]
pub trait SmsService: Send + Sync {
    async fn send(&self, phone: &str, message: &str) -> Result<(), String>;
}

/// Abstraction over JWT token generation and validation for profile-change flows.
/// Concrete implementation lives in the infrastructure crate (`JwtTokenService`).
pub trait TokenService: Send + Sync {
    fn generate(
        &self,
        profile_change_id: Uuid,
        user_uuid: Uuid,
        expires_in_minutes: u32,
    ) -> Result<String, String>;

    /// Returns `(profile_change_id, user_uuid)` on success.
    fn validate(&self, token: &str) -> Result<(Uuid, Uuid), String>;
}

pub struct ProfileChangeUseCases {
    profile_changes: Arc<dyn ProfileChangeRepository>,
    customers: Arc<dyn CustomerRepository>,
    config: Arc<AppConfig>,
    sms: Arc<dyn SmsService>,
    token_service: Arc<dyn TokenService>,
    publisher: Arc<dyn Publisher>,
}

impl ProfileChangeUseCases {
    pub fn new(
        profile_changes: Arc<dyn ProfileChangeRepository>,
        customers: Arc<dyn CustomerRepository>,
        config: Arc<AppConfig>,
        sms: Arc<dyn SmsService>,
        token_service: Arc<dyn TokenService>,
    ) -> Self {
        Self {
            profile_changes,
            customers,
            config,
            sms,
            token_service,
            publisher: Arc::new(NoopPublisher),
        }
    }

    pub fn with_publisher(mut self, publisher: Arc<dyn Publisher>) -> Self {
        self.publisher = publisher;
        self
    }

    /// Begin a phone-number or email-change flow. Sends an OTP and returns the pending record.
    pub async fn create_profile_change(
        &self,
        user_uuid: Uuid,
        req: CreateProfileChangeRequest,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        let change_type = parse_change_type(&req.change_type)?;

        // Reject concurrent active requests for the same type.
        if self
            .profile_changes
            .find_active_by_user_and_type(user_uuid, change_type.clone())
            .await?
            .is_some()
        {
            return Err(ApplicationError::BadRequest(
                "an active profile change of this type already exists".to_string(),
            ));
        }

        // Fetch current value to store as old_value.
        let customer =
            self.customers.find_by_id(user_uuid).await?.ok_or_else(|| {
                ApplicationError::NotFound(format!("customer {user_uuid} not found"))
            })?;

        let old_value = match change_type {
            ChangeType::Telephone => customer.phone.clone(),
            ChangeType::Email => customer.email.clone(),
        };

        let new_value = match change_type {
            ChangeType::Telephone => {
                normalize_phone(&req.new_value, &self.config.phone_number_format)
            }
            ChangeType::Email => req.new_value.clone(),
        };

        let otp = crate::profile_changes::use_cases::generate_otp_code();
        let ref_code = crate::profile_changes::use_cases::generate_ref_code();
        let now = Utc::now();
        let otp_expired_at = now + Duration::minutes(self.config.otp_expired_time as i64);
        let next_otp_request_at = now + Duration::minutes(1);
        let token_expired_at =
            now + Duration::minutes(self.config.profile_change_expired_time as i64);

        let record = self
            .profile_changes
            .create(CreateProfileChange {
                user_uuid,
                change_type: change_type.clone(),
                identifier: None,
                old_value,
                new_value: Some(new_value.clone()),
                status: ChangeStatus::PendingVerifyOtp,
                otp: Some(otp.clone()),
                ref_code: Some(ref_code.clone()),
                token_expired_at,
                next_otp_request_at,
                otp_expired_at,
            })
            .await?;

        if change_type == ChangeType::Telephone {
            let message = self.config.otp_text.replace("{otp}", &otp);
            self.sms
                .send(&new_value, &message)
                .await
                .map_err(ApplicationError::External)?;
        } else if change_type == ChangeType::Email {
            let payload = serde_json::to_string(&EmailSentRequestedPayload {
                user_uuid: user_uuid.to_string(),
                email: new_value.clone(),
                otp: otp.clone(),
                ref_code: ref_code.clone(),
                otp_expired_at: otp_expired_at.to_rfc3339(),
            })
            .unwrap_or_default();
            if let Err(e) = self
                .publisher
                .publish(&self.config.sns_email_sent_requested, &payload)
                .await
            {
                tracing::warn!("sns publish failed (email sent requested): {e}");
            }
        }

        Ok(record.into())
    }

    /// Resend OTP (caller supplies the existing token to prove ownership of the request).
    pub async fn update_profile_change(
        &self,
        user_uuid: Uuid,
        profile_id: Uuid,
        req: UpdateProfileChangeRequest,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        let record = self
            .profile_changes
            .find_by_id(profile_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("profile change {profile_id} not found"))
            })?;

        if record.user_uuid != user_uuid {
            return Err(ApplicationError::NotFound(format!(
                "profile change {profile_id} not found"
            )));
        }

        // Validate the existing token to ensure the caller owns this request.
        let (claimed_id, claimed_user) = self
            .token_service
            .validate(&req.token)
            .map_err(|e| ApplicationError::BadRequest(format!("invalid token: {e}")))?;

        if claimed_id != profile_id || claimed_user != user_uuid {
            return Err(ApplicationError::BadRequest("token mismatch".to_string()));
        }

        let now = Utc::now();

        // Respect the rate-limit window.
        if now < record.next_otp_request_at {
            return Err(ApplicationError::BadRequest(
                "OTP request too soon — please wait before requesting a new OTP".to_string(),
            ));
        }

        let otp = crate::profile_changes::use_cases::generate_otp_code();
        let ref_code = crate::profile_changes::use_cases::generate_ref_code();
        let otp_expired_at = now + Duration::minutes(self.config.otp_expired_time as i64);
        let next_otp_request_at = now + Duration::minutes(1);

        let updated = self
            .profile_changes
            .update_otp(
                profile_id,
                otp.clone(),
                ref_code,
                otp_expired_at,
                next_otp_request_at,
            )
            .await?;

        // Resend SMS for phone changes.
        if record.change_type == ChangeType::Telephone
            && let Some(ref new_phone) = record.new_value
        {
            let message = self.config.otp_text.replace("{otp}", &otp);
            self.sms
                .send(new_phone, &message)
                .await
                .map_err(ApplicationError::External)?;
        }

        Ok(updated.into())
    }

    /// Verify the OTP supplied by the user and return a short-lived confirmation token.
    pub async fn verify_profile_change(
        &self,
        user_uuid: Uuid,
        profile_id: Uuid,
        req: VerifyProfileChangeRequest,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        let record = self
            .profile_changes
            .find_by_id(profile_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("profile change {profile_id} not found"))
            })?;

        if record.user_uuid != user_uuid {
            return Err(ApplicationError::NotFound(format!(
                "profile change {profile_id} not found"
            )));
        }

        // OTP must match.
        if record.otp.as_deref() != Some(req.otp.as_str()) {
            return Err(ApplicationError::BadRequest("invalid OTP".to_string()));
        }

        // OTP must not be expired.
        if Utc::now() > record.otp_expired_at {
            return Err(ApplicationError::BadRequest("OTP has expired".to_string()));
        }

        // Issue a short-lived confirmation token.
        let token = self
            .token_service
            .generate(profile_id, user_uuid, self.config.token_expired_time)
            .map_err(|e| ApplicationError::External(format!("token generation failed: {e}")))?;

        let token_expires = Utc::now() + Duration::minutes(self.config.token_expired_time as i64);

        let updated = self
            .profile_changes
            .update_status_and_token(
                profile_id,
                ChangeStatus::VerifyChangeCompleted,
                Some(token),
                Some(token_expires),
            )
            .await?;

        Ok(updated.into())
    }

    /// Finalise the change using the confirmation token issued by `verify_profile_change`.
    pub async fn confirm_profile_change(
        &self,
        user_uuid: Uuid,
        profile_id: Uuid,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        let record = self
            .profile_changes
            .find_by_id(profile_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("profile change {profile_id} not found"))
            })?;

        if record.user_uuid != user_uuid {
            return Err(ApplicationError::NotFound(format!(
                "profile change {profile_id} not found"
            )));
        }

        if record.status != ChangeStatus::VerifyChangeCompleted {
            return Err(ApplicationError::BadRequest(
                "OTP must be verified before confirming the change".to_string(),
            ));
        }

        if Utc::now() > record.token_expired_at {
            return Err(ApplicationError::BadRequest(
                "confirmation token has expired".to_string(),
            ));
        }

        let new_value = record
            .new_value
            .clone()
            .ok_or_else(|| ApplicationError::BadRequest("missing new value".to_string()))?;

        // Apply the change to the customer record.
        let update = match record.change_type {
            ChangeType::Telephone => UpdateCustomer {
                phone: Some(new_value),
                email: None,
                locale: None,
                has_consent: None,
                first_name: None,
                last_name: None,
                birthdate: None,
                gender: None,
                nationality: None,
            },
            ChangeType::Email => UpdateCustomer {
                email: Some(new_value),
                phone: None,
                locale: None,
                has_consent: None,
                first_name: None,
                last_name: None,
                birthdate: None,
                gender: None,
                nationality: None,
            },
        };
        self.customers.update(user_uuid, update).await?;

        let payload = serde_json::to_string(&ProfileChangedPayload {
            user_uuid: user_uuid.to_string(),
        })
        .unwrap_or_default();
        if let Err(e) = self
            .publisher
            .publish(&self.config.sns_user_profile_changed, &payload)
            .await
        {
            tracing::warn!("sns publish failed (profile changed via confirm): {e}");
        }

        let completed = self
            .profile_changes
            .update_status_and_token(profile_id, ChangeStatus::Completed, None, None)
            .await?;

        Ok(completed.into())
    }
}

// ---- private helpers ----

fn parse_change_type(s: &str) -> Result<ChangeType, ApplicationError> {
    match s.to_lowercase().as_str() {
        "telephone" => Ok(ChangeType::Telephone),
        "email" => Ok(ChangeType::Email),
        _ => Err(ApplicationError::BadRequest(format!(
            "invalid change_type '{s}': expected 'telephone' or 'email'"
        ))),
    }
}

fn normalize_phone(phone: &str, format: &str) -> String {
    if let Some(stripped) = phone.strip_prefix('0') {
        format!("{}{}", format, stripped)
    } else {
        phone.to_string()
    }
}

pub(crate) fn generate_otp_code() -> String {
    use rand::Rng;
    format!("{:06}", rand::thread_rng().gen_range(0..=999999u32))
}

pub(crate) fn generate_ref_code() -> String {
    use rand::Rng;
    (0..6)
        .map(|_| char::from(rand::thread_rng().gen_range(b'A'..=b'Z')))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
}

