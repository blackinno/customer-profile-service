use application::events::Publisher;
use async_trait::async_trait;
use aws_sdk_sns::Client;
use std::sync::Arc;

pub struct AwsSns {
    pub client: Client,
}

impl AwsSns {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn publish_message(
        &self,
        topic_arn: &str,
        message: &str,
    ) -> Result<String, aws_sdk_sns::Error> {
        let result = self
            .client
            .publish()
            .topic_arn(topic_arn)
            .message(message)
            .send()
            .await?;

        Ok(result.message_id().unwrap_or("unknown").to_string())
    }
}

pub struct SnsPublisher {
    sns: Arc<AwsSns>,
}

impl SnsPublisher {
    pub fn new(sns: Arc<AwsSns>) -> Self {
        Self { sns }
    }
}

#[async_trait]
impl Publisher for SnsPublisher {
    async fn publish(&self, topic_arn: &str, payload: &str) -> Result<(), String> {
        self.sns
            .publish_message(topic_arn, payload)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[derive(Clone)]
pub struct Message {
    pub sns: Arc<AwsSns>,
}
