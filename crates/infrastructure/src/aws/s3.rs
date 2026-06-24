use application::profile_images::use_cases::ImageStorage;
use async_trait::async_trait;
use aws_sdk_s3::Client;

pub struct S3Storage {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Storage {
    pub fn new(client: Client, bucket: String, prefix: String) -> Self {
        Self {
            client,
            bucket,
            prefix,
        }
    }
}

#[async_trait]
impl ImageStorage for S3Storage {
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<String, String> {
        let full_key = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        };

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(data.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(full_key)
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
