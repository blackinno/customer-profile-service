use async_trait::async_trait;
use chrono::Duration;
use domain::events::{DispatchError, EventDispatcher};
use qml_rs::{Job, JobState, Storage};
use std::sync::Arc;

/// QML-backed implementation of `EventDispatcher`.
///
/// Args from the domain port (`Vec<String>`) are serialized into the qml-rs
/// JSON payload as an array of strings. Workers read them back via
/// `job.payload.as_array()`. Keep individual elements string-encoded
/// (UUIDs, emails, JSON blobs, etc.) at the call site.
#[derive(Clone)]
pub struct QmlEventDispatcher {
    storage: Arc<dyn Storage>,
}

impl QmlEventDispatcher {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl EventDispatcher for QmlEventDispatcher {
    async fn dispatch(&self, event: &str, args: Vec<String>) -> Result<(), DispatchError> {
        let job = Job::new(event, serde_json::json!(args));
        self.storage
            .enqueue(&job)
            .await
            .map_err(|e| DispatchError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn dispatch_delayed(
        &self,
        event: &str,
        args: Vec<String>,
        delay_minutes: i64,
        reason: &str,
    ) -> Result<(), DispatchError> {
        let mut job = Job::new(event, serde_json::json!(args));
        let scheduled_time = chrono::Utc::now() + Duration::minutes(delay_minutes);
        job.state = JobState::scheduled(scheduled_time, reason.to_string());
        self.storage
            .enqueue(&job)
            .await
            .map_err(|e| DispatchError::Backend(e.to_string()))?;
        Ok(())
    }
}
