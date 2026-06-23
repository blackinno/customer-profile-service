use application::events::Publisher;
use async_trait::async_trait;
use std::sync::Arc;

use super::sns::AwsSns;

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
