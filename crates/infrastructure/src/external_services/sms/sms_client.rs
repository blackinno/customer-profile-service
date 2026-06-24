use application::profile_changes::use_cases::SmsService;
use async_trait::async_trait;

pub struct SmsClient {
    http: reqwest::Client,
    proxy_url: String,
}

impl SmsClient {
    pub fn new(proxy_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            proxy_url,
        }
    }
}

#[async_trait]
impl SmsService for SmsClient {
    async fn send(&self, phone: &str, message: &str) -> Result<(), String> {
        self.http
            .post(&self.proxy_url)
            .json(&serde_json::json!({ "phone": phone, "message": message }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
