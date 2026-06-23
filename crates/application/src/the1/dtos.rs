use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct TierResponse {
    pub id: String,
    pub code: String,
    pub name: Option<String>,
    pub expired_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct The1AccountResponse {
    pub id: String,
    pub user_uuid: Uuid,
    pub member_id: String,
    pub account_id: String,
    pub profile_id: String,
    pub card_number: Option<String>,
    pub tiers: Vec<TierResponse>,
    pub created_at: String,
    pub updated_at: String,
}
