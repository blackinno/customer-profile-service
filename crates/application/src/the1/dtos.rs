use chrono::{DateTime, Utc};
use domain::entities::the1_user::{The1User, Tier};
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

impl From<Tier> for TierResponse {
    fn from(t: Tier) -> Self {
        TierResponse {
            id: t.id.to_string(),
            code: t.code,
            name: t.name,
            expired_date: t.expired_date,
        }
    }
}

impl From<The1User> for The1AccountResponse {
    fn from(user: The1User) -> Self {
        The1AccountResponse {
            id: user.id.to_string(),
            user_uuid: user.user_uuid,
            member_id: user.member_id,
            account_id: user.account_id,
            profile_id: user.profile_id,
            card_number: user.card_number,
            tiers: user.tiers.into_iter().map(TierResponse::from).collect(),
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
    }
}
