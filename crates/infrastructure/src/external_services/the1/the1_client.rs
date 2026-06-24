use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::entities::the1_user::UpsertTier;
use serde::Deserialize;
use uuid::Uuid;

use application::segments::use_cases::{The1Client, The1PartnerMemberData};

#[derive(Deserialize)]
pub struct The1ProfileResponse {
    pub member_id: Option<String>,
    pub account_id: Option<String>,
    pub profile_id: Option<String>,
    pub card_number: Option<String>,
    pub tiers: Option<Vec<The1TierData>>,
}

#[derive(Deserialize)]
pub struct The1TierData {
    pub code: String,
    pub name: Option<String>,
    pub expired_date: Option<String>,
}

#[derive(Deserialize)]
pub struct InvokeTokenResponse {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
pub struct The1PartnerMemberResponse {
    pub user_uuid: Option<String>,
    pub member_id: Option<String>,
    pub account_id: Option<String>,
    pub profile_id: Option<String>,
    pub card_number: Option<String>,
    pub tiers: Option<Vec<The1TierData>>,
}

pub struct The1HttpClient {
    http: reqwest::Client,
    base_url: String,
}

impl The1HttpClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn get_profile(&self, access_token: &str) -> Result<The1ProfileResponse, String> {
        self.http
            .get(format!("{}/customers/me", self.base_url))
            .bearer_auth(access_token)
            .header("x-api-channel", "central-x")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn invoke_token(&self, refresh_token: &str) -> Result<InvokeTokenResponse, String> {
        self.http
            .post(format!("{}/auth/invoke", self.base_url))
            .bearer_auth(refresh_token)
            .header("x-api-channel", "central-x")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    async fn fetch_partner_member(
        &self,
        card_number: &str,
    ) -> Result<The1PartnerMemberResponse, String> {
        self.http
            .get(format!("{}/partner-members/{}", self.base_url, card_number))
            .header("x-api-channel", "central-x")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    fn map_tier(tier: The1TierData) -> UpsertTier {
        let expired_date = tier
            .expired_date
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        UpsertTier {
            code: tier.code,
            name: tier.name,
            expired_date,
        }
    }
}

#[async_trait]
impl The1Client for The1HttpClient {
    async fn get_partner_member(&self, card_number: &str) -> Result<The1PartnerMemberData, String> {
        let resp = self.fetch_partner_member(card_number).await?;

        let user_uuid_str = resp
            .user_uuid
            .ok_or_else(|| "The1 response missing user_uuid".to_string())?;
        let user_uuid = Uuid::parse_str(&user_uuid_str)
            .map_err(|e| format!("invalid user_uuid from The1: {e}"))?;

        let member_id = resp
            .member_id
            .ok_or_else(|| "The1 response missing member_id".to_string())?;
        let account_id = resp
            .account_id
            .ok_or_else(|| "The1 response missing account_id".to_string())?;
        let profile_id = resp
            .profile_id
            .ok_or_else(|| "The1 response missing profile_id".to_string())?;

        let tiers = resp
            .tiers
            .unwrap_or_default()
            .into_iter()
            .map(Self::map_tier)
            .collect();

        Ok(The1PartnerMemberData {
            user_uuid,
            member_id,
            account_id,
            profile_id,
            card_number: resp.card_number,
            tiers,
        })
    }
}
