use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateIdentityRequest {
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IdentityResponse {
    pub id: String,
    pub user_uuid: String,
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
    pub is_deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct InvokeTokenResponse {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}
