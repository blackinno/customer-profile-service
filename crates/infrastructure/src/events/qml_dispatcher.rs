use async_trait::async_trait;
use chrono::Duration;
use domain::events::{DispatchError, EventDispatcher};
use qml_rs::{Job, JobState, Storage};
use serde_json::Value;
use std::sync::Arc;

/// Encode `args` into a `serde_json::Value` for storage as `job.payload`.
///
/// When there is exactly one arg and it parses as a JSON object or array, it is
/// stored unwrapped so that `TypedWorker` implementations can deserialize the
/// payload directly into their `Args` struct. Everything else is stored as a
/// JSON array of strings (legacy fallback).
fn encode_payload(args: Vec<String>) -> Value {
    if let [single] = args.as_slice() {
        if let Ok(v @ (Value::Object(_) | Value::Array(_))) = serde_json::from_str(single) {
            return v;
        }
    }
    serde_json::json!(args)
}

/// QML-backed implementation of `EventDispatcher`.
///
/// **Payload encoding** — single-arg dispatches where the arg is a valid JSON
/// object are stored as the parsed value directly so that `TypedWorker`
/// implementations can deserialize the payload into their `Args` struct without
/// an extra layer of base64/string unwrapping. All other cases (multiple args,
/// or a single non-JSON string) fall back to a JSON array of strings.
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
        let job = Job::new(event, encode_payload(args));
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
        let mut job = Job::new(event, encode_payload(args));
        let scheduled_time = chrono::Utc::now() + Duration::minutes(delay_minutes);
        job.state = JobState::scheduled(scheduled_time, reason.to_string());
        self.storage
            .enqueue(&job)
            .await
            .map_err(|e| DispatchError::Backend(e.to_string()))?;
        Ok(())
    }
}
