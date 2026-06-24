use application::background_tasks::SnsPublishCommand;
use async_trait::async_trait;
use domain::SNS_PUBLISH_TASK;
use qml_rs::{QmlError, TypedWorker, WorkerContext, WorkerResult};
use serde_json::to_string;

use crate::AppFactoryState;

pub struct SnsPublishTask {
    #[allow(dead_code)]
    stage: AppFactoryState,
}

impl SnsPublishTask {
    pub fn new(stage: AppFactoryState) -> Self {
        Self { stage }
    }
}

#[async_trait]
impl TypedWorker for SnsPublishTask {
    type Args = SnsPublishCommand;

    fn method_name(&self) -> &str {
        SNS_PUBLISH_TASK
    }

    async fn execute(
        &self,
        cmd: SnsPublishCommand,
        context: &WorkerContext,
    ) -> Result<WorkerResult, QmlError> {
        tracing::info!("📣 SNS publish task executing, topic: {}", cmd.topic_arn);

        let message_str = to_string(&cmd.message).map_err(|e| QmlError::WorkerError {
            message: format!("failed to serialize SNS message: {e}"),
        })?;

        #[cfg(feature = "sns")]
        self.stage
            .message
            .sns
            .publish_message(&cmd.topic_arn, &message_str)
            .await
            .map_err(|e| QmlError::WorkerError {
                message: format!("SNS publish failed: {e}"),
            })?;

        #[cfg(not(feature = "sns"))]
        tracing::info!(
            "SNS disabled — skipping publish to {} with message: {}",
            cmd.topic_arn,
            message_str
        );

        Ok(WorkerResult::success(
            Some(format!("SNS published to {}", cmd.topic_arn)),
            context.duration().num_milliseconds() as u64,
        ))
    }
}
