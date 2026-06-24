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

