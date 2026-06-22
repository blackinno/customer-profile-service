use aws_sdk_sns::Client;

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
