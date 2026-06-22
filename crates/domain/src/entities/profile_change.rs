use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Telephone,
    Email,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeStatus {
    PendingVerifyOtp,
    VerifyChangeCompleted,
    PendingChangeTopConfirmation,
    Completed,
}

#[derive(Debug, Clone)]
pub struct ProfileChange {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub change_type: ChangeType,
    pub identifier: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub status: ChangeStatus,
    pub token: Option<String>,
    pub token_expired_at: DateTime<Utc>,
    pub otp: Option<String>,
    pub ref_code: Option<String>,
    pub next_otp_request_at: DateTime<Utc>,
    pub otp_expired_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateProfileChange {
    pub user_uuid: Uuid,
    pub change_type: ChangeType,
    pub identifier: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub status: ChangeStatus,
    pub otp: Option<String>,
    pub ref_code: Option<String>,
    pub token_expired_at: DateTime<Utc>,
    pub next_otp_request_at: DateTime<Utc>,
    pub otp_expired_at: DateTime<Utc>,
}
