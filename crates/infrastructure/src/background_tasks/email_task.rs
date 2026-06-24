use application::background_tasks::EmailCommand;
#[cfg(feature = "sns")]
use application::events::EmailSentRequestedPayload;
use async_trait::async_trait;
use domain::EMAIL_SEND_TASK;
use qml_rs::{QmlError, TypedWorker, WorkerContext, WorkerResult};

use crate::AppFactoryState;

pub struct EmailTask {
    #[allow(dead_code)]
    stage: AppFactoryState,
}

impl EmailTask {
    pub fn new(stage: AppFactoryState) -> Self {
        Self { stage }
    }
}

#[async_trait]
impl TypedWorker for EmailTask {
    type Args = EmailCommand;

    fn method_name(&self) -> &str {
        EMAIL_SEND_TASK
    }

    async fn execute(
        &self,
        cmd: EmailCommand,
        context: &WorkerContext,
    ) -> Result<WorkerResult, QmlError> {
        tracing::info!("📧 Email task executing, to: {}", cmd.email);

        #[cfg(feature = "sns")]
        {
            let payload = EmailSentRequestedPayload {
                user_uuid: cmd.user_uuid.clone(),
                email: cmd.email.clone(),
                otp: cmd.otp,
                ref_code: cmd.ref_code,
                otp_expired_at: cmd.otp_expired_at,
            };
            let message_str =
                serde_json::to_string(&payload).map_err(|e| QmlError::WorkerError {
                    message: format!("failed to serialize EmailSentRequestedPayload: {e}"),
                })?;
            self.stage
                .message
                .sns
                .publish_message(&self.stage.settings.sns_email_sent_requested, &message_str)
                .await
                .map_err(|e| QmlError::WorkerError {
                    message: format!("SNS publish for email task failed: {e}"),
                })?;
        }

        #[cfg(not(feature = "sns"))]
        tracing::info!(
            "SNS disabled — skipping email publish for user {}",
            cmd.user_uuid
        );

        Ok(WorkerResult::success(
            Some(format!("Email task published for {}", cmd.email)),
            context.duration().num_milliseconds() as u64,
        ))
    }
}
