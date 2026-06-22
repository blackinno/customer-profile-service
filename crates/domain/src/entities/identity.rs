use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Identity {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateIdentity {
    pub user_uuid: Uuid,
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateIdentityTokens {
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}
