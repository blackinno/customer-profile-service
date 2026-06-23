use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct SegmentResponse {
    pub segment_slug: String,
    pub expired_time: Option<DateTime<Utc>>,
    pub user_uuid: Uuid,
}
