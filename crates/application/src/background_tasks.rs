use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Payload for a generic SNS publish job.
/// `topic_arn` is the full ARN; `message` is any JSON-serialisable value.
#[derive(Debug, Serialize, Deserialize)]
pub struct SnsPublishCommand {
    pub topic_arn: String,
    pub message: Value,
}

/// Payload for a queued SMS delivery job.
#[derive(Debug, Serialize, Deserialize)]
pub struct SmsCommand {
    pub phone: String,
    pub message: String,
}

/// Payload for a queued email delivery job.
/// The worker publishes this to the `sns_email_sent_requested` topic,
/// which a downstream consumer turns into an actual email.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailCommand {
    pub user_uuid: String,
    pub email: String,
    pub otp: String,
    pub ref_code: String,
    pub otp_expired_at: String,
}
