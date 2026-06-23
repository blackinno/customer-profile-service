use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("dispatch backend error: {0}")]
    Backend(String),
}

/// Port for emitting domain events to a background worker / message bus.
///
/// Implementations live in `infrastructure` (e.g. a Postgres-backed job queue,
/// SNS, in-memory for tests). The trait is intentionally framework-agnostic:
/// `args` is a vector of already-stringified parameters because most queue
/// backends ultimately persist arguments as strings.
#[async_trait]
pub trait EventDispatcher: Send + Sync {
    async fn dispatch(&self, event: &str, args: Vec<String>) -> Result<(), DispatchError>;

    async fn dispatch_delayed(
        &self,
        event: &str,
        args: Vec<String>,
        delay_minutes: i64,
        reason: &str,
    ) -> Result<(), DispatchError>;
}
