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
