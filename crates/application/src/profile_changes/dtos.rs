use domain::entities::profile_change::{ChangeStatus, ChangeType, ProfileChange};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProfileChangeRequest {
    pub change_type: String,
    pub new_value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileChangeRequest {
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyProfileChangeRequest {
    pub otp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProfileChangeResponse {
    pub id: String,
    pub user_uuid: String,
    pub change_type: String,
    pub identifier: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub status: String,
    pub token: Option<String>,
    pub token_expired_at: String,
    pub ref_code: Option<String>,
    pub next_otp_request_at: String,
    pub otp_expired_at: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProfileChange> for ProfileChangeResponse {
    fn from(pc: ProfileChange) -> Self {
        ProfileChangeResponse {
            id: pc.id.to_string(),
            user_uuid: pc.user_uuid.to_string(),
            change_type: change_type_to_str(&pc.change_type).to_string(),
            identifier: pc.identifier,
            old_value: pc.old_value,
            new_value: pc.new_value,
            status: change_status_to_str(&pc.status).to_string(),
            token: pc.token,
            token_expired_at: pc.token_expired_at.to_rfc3339(),
            ref_code: pc.ref_code,
            next_otp_request_at: pc.next_otp_request_at.to_rfc3339(),
            otp_expired_at: pc.otp_expired_at.to_rfc3339(),
            created_at: pc.created_at.to_rfc3339(),
            updated_at: pc.updated_at.to_rfc3339(),
        }
    }
}

fn change_type_to_str(ct: &ChangeType) -> &'static str {
    match ct {
        ChangeType::Telephone => "telephone",
        ChangeType::Email => "email",
    }
}

fn change_status_to_str(cs: &ChangeStatus) -> &'static str {
    match cs {
        ChangeStatus::PendingVerifyOtp => "pending_verify_otp",
        ChangeStatus::VerifyChangeCompleted => "verify_change_completed",
        ChangeStatus::PendingChangeTopConfirmation => "pending_change_top_confirmation",
        ChangeStatus::Completed => "completed",
    }
}
