use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Tier {
    pub id: Uuid,
    pub code: String,
    pub name: Option<String>,
    pub expired_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct The1User {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub member_id: String,
    pub account_id: String,
    pub profile_id: String,
    pub card_number: Option<String>,
    pub tiers: Vec<Tier>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertTier {
    pub code: String,
    pub name: Option<String>,
    pub expired_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct UpsertThe1User {
    pub member_id: String,
    pub account_id: String,
    pub profile_id: String,
    pub card_number: Option<String>,
    pub tiers: Vec<UpsertTier>,
}
