use domain::entities::identity::Identity;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIdentityRequest {
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct InvokeTokenResponse {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl From<Identity> for IdentityResponse {
    fn from(identity: Identity) -> Self {
        IdentityResponse {
            id: identity.id.to_string(),
            user_uuid: identity.user_uuid.to_string(),
            provider_name: identity.provider_name,
            external_id: identity.external_id,
            provider_id_token: identity.provider_id_token,
            provider_access_token: identity.provider_access_token,
            provider_refresh_token: identity.provider_refresh_token,
            is_deleted: identity.is_deleted,
            created_at: identity.created_at.to_rfc3339(),
            updated_at: identity.updated_at.to_rfc3339(),
        }
    }
}
