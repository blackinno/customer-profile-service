use std::sync::Arc;

use async_trait::async_trait;
use domain::entities::the1_user::{UpsertThe1User, UpsertTier};
use domain::repositories::the1_user_repository::The1UserRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApplicationError;
use crate::events::{NoopPublisher, Publisher, The1ProfileUpdatedPayload};
use crate::segments::dtos::SegmentResponse;

/// Data returned from The1 partner-member endpoint, already converted to
/// domain types. The trait implementation (in infrastructure) is responsible
/// for HTTP transport and JSON-to-domain mapping.
pub struct The1PartnerMemberData {
    pub user_uuid: Uuid,
    pub member_id: String,
    pub account_id: String,
    pub profile_id: String,
    pub card_number: Option<String>,
    pub tiers: Vec<UpsertTier>,
}

/// Application-level gateway to The1 external service.
/// Concrete implementation lives in the infrastructure crate so the
/// application layer stays free of reqwest / HTTP details.
#[async_trait]
pub trait The1Client: Send + Sync {
    async fn get_partner_member(&self, card_number: &str) -> Result<The1PartnerMemberData, String>;
}

pub struct SegmentUseCases {
    the1_users: Arc<dyn The1UserRepository>,
    the1_client: Arc<dyn The1Client>,
    publisher: Arc<dyn Publisher>,
    sns_topic: String,
}

impl SegmentUseCases {
    pub fn new(the1_users: Arc<dyn The1UserRepository>, the1_client: Arc<dyn The1Client>) -> Self {
        Self {
            the1_users,
            the1_client,
            publisher: Arc::new(NoopPublisher),
            sns_topic: String::new(),
        }
    }

    pub fn with_publisher(mut self, publisher: Arc<dyn Publisher>, config: &AppConfig) -> Self {
        self.publisher = publisher;
        self.sns_topic = config.sns_user_the1_get_profile_updated.clone();
        self
    }

    /// Look up a The1 member by card number, upsert the local record, and
    /// return the member's primary segment (first tier) as a `SegmentResponse`.
    ///
    /// Returns `NotFound` when the member has no tiers (no segment assigned).
    pub async fn get_segment(
        &self,
        card_number: String,
    ) -> Result<SegmentResponse, ApplicationError> {
        let data = self
            .the1_client
            .get_partner_member(&card_number)
            .await
            .map_err(ApplicationError::External)?;

        let user_uuid = data.user_uuid;
        let upsert_data = UpsertThe1User {
            member_id: data.member_id,
            account_id: data.account_id,
            profile_id: data.profile_id,
            card_number: data.card_number,
            tiers: data.tiers,
        };

        let the1_user = self.the1_users.upsert(user_uuid, upsert_data).await?;

        let payload = serde_json::to_string(&The1ProfileUpdatedPayload {
            user_uuid: user_uuid.to_string(),
            member_id: the1_user.member_id.clone(),
            card_number: the1_user.card_number.clone(),
        })
        .unwrap_or_default();
        if let Err(e) = self.publisher.publish(&self.sns_topic, &payload).await {
            tracing::warn!("sns publish failed (the1 profile updated): {e}");
        }

        let the1_user_uuid = the1_user.user_uuid;
        let first_tier = the1_user
            .tiers
            .into_iter()
            .next()
            .ok_or_else(|| ApplicationError::NotFound("no segment found".to_string()))?;

        Ok(SegmentResponse {
            segment_slug: first_tier.code,
            expired_time: first_tier.expired_date,
            user_uuid: the1_user_uuid,
        })
    }
}
