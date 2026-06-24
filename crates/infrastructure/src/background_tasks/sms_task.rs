use application::{background_tasks::SmsCommand, profile_changes::use_cases::SmsService};
use async_trait::async_trait;
use domain::SMS_SEND_TASK;
use qml_rs::{QmlError, TypedWorker, WorkerContext, WorkerResult};
use std::sync::Arc;

pub struct SmsTask {
    sms: Arc<dyn SmsService>,
}

impl SmsTask {
    pub fn new(sms: Arc<dyn SmsService>) -> Self {
        Self { sms }
    }
}

#[async_trait]
impl TypedWorker for SmsTask {
    type Args = SmsCommand;

    fn method_name(&self) -> &str {
        SMS_SEND_TASK
    }

    async fn execute(
        &self,
        cmd: SmsCommand,
        context: &WorkerContext,
    ) -> Result<WorkerResult, QmlError> {
        tracing::info!("📱 SMS task executing, phone: {}", cmd.phone);

        self.sms
            .send(&cmd.phone, &cmd.message)
            .await
            .map_err(|e| QmlError::WorkerError {
                message: format!("SMS send failed: {e}"),
            })?;

        Ok(WorkerResult::success(
            Some(format!("SMS sent to {}", cmd.phone)),
            context.duration().num_milliseconds() as u64,
        ))
    }
}
