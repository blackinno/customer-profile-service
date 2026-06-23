use async_trait::async_trait;
use serde::Serialize;

/// Application-layer port for publishing domain events to a message bus.
/// Concrete implementation lives in the infrastructure crate (`SnsPublisher`).
/// Tests and non-SNS builds use `NoopPublisher`.
#[async_trait]
pub trait Publisher: Send + Sync {
    async fn publish(&self, topic_arn: &str, payload: &str) -> Result<(), String>;
}

pub struct NoopPublisher;

#[async_trait]
impl Publisher for NoopPublisher {
    async fn publish(&self, _topic_arn: &str, _payload: &str) -> Result<(), String> {
        Ok(())
    }
}

// ---- Domain event payload types ----

#[derive(Serialize)]
pub struct ProfileChangedPayload {
    pub user_uuid: String,
}

#[derive(Serialize)]
pub struct EmailSentRequestedPayload {
    pub user_uuid: String,
    pub email: String,
    pub otp: String,
    pub ref_code: String,
    pub otp_expired_at: String,
}

#[derive(Serialize)]
pub struct IdentityLinkedChangedPayload {
    pub user_uuid: String,
    pub provider_name: String,
    /// "linked" or "unlinked"
    pub action: String,
}

#[derive(Serialize)]
pub struct The1ProfileUpdatedPayload {
    pub user_uuid: String,
    pub member_id: String,
    pub card_number: Option<String>,
}
