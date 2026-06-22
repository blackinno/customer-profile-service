/// Application-layer configuration. Infrastructure's Settings populates this
/// at startup via `From<&Settings>`. Use cases depend only on this type, not
/// on the infrastructure crate.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub country_code: String,
    pub phone_number_format: String,
    pub otp_expired_time: u32,
    pub otp_text: String,
    pub jwt_secret_key: String,
    pub profile_change_expired_time: u32,
    pub token_expired_time: u32,
    pub allow_image_types: Vec<String>,
    pub max_image_size_mb: u32,
    pub image_prefix: String,
    pub image_expired_in_sec: u32,
    pub sns_user_profile_changed: String,
    pub sns_email_sent_requested: String,
    pub sns_user_identity_linked_changed: String,
    pub sns_user_the1_get_profile_updated: String,
    pub s3_profile_bucket: String,
    pub cloudfront_base_endpoint: String,
    pub cloudfront_key_id: String,
}
