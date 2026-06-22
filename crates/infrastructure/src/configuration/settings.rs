use std::env::var;
use std::fmt;

use serde::Deserialize;

/// Application configuration loaded from environment variables.
///
/// Note: `Debug` masks sensitive fields so credentials don't leak through tracing calls.
#[derive(Clone, Deserialize)]
pub struct Settings {
    // Template fields
    pub database_url: String,
    pub qml_database_url: String,
    pub qml_worker_count: usize,
    pub qml_batch_size: usize,
    pub qml_retry_max_attempts: u32,
    pub qml_retry_base_seconds: u32,
    pub qml_retry_multiplier: f64,
    pub qml_retry_max_seconds: u32,
    pub aws_region: String,
    pub server_host: String,
    pub server_port: u16,
    // AWS S3 / CloudFront
    pub s3_profile_bucket: String,
    pub cloudfront_base_endpoint: String,
    pub cloudfront_private_key: String,
    pub cloudfront_key_id: String,
    pub image_expired_in_sec: u32,
    // SNS topic ARNs
    pub sns_user_profile_changed: String,
    pub sns_email_sent_requested: String,
    pub sns_user_identity_linked_changed: String,
    pub sns_user_the1_get_profile_updated: String,
    // The1
    pub the1_proxy_service_url: String,
    // SMS
    pub sms_proxy_service_url: String,
    pub phone_number_format: String,
    pub otp_text: String,
    pub otp_expired_time: u32,
    // Auth / misc
    pub jwt_secret_key: String,
    pub country_code: String,
    pub profile_change_expired_time: u32,
    pub token_expired_time: u32,
    pub allow_image_types: Vec<String>,
    pub max_image_size_mb: u32,
    pub image_prefix: String,
}

impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("database_url", &"[REDACTED]")
            .field("qml_database_url", &"[REDACTED]")
            .field("jwt_secret_key", &"[REDACTED]")
            .field("cloudfront_private_key", &"[REDACTED]")
            .field("server_host", &self.server_host)
            .field("server_port", &self.server_port)
            .finish()
    }
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = required_var("DATABASE_URL")?;
        let qml_database_url = var("QML_DATABASE_URL").unwrap_or_else(|_| database_url.clone());
        let qml_worker_count = parse_var("QML_WORKER_COUNT", 2)?;
        let qml_batch_size = parse_var("QML_BATCH_SIZE", 5)?;
        let qml_retry_max_attempts = parse_var("QML_RETRY_MAX_ATTEMPTS", 1u32)?;
        let qml_retry_base_seconds = parse_var("QML_RETRY_BASE_SECONDS", 1u32)?;
        let qml_retry_multiplier = parse_var("QML_RETRY_MULTIPLIER", 2.0f64)?;
        if !qml_retry_multiplier.is_finite() || qml_retry_multiplier <= 0.0 {
            anyhow::bail!(
                "invalid value for `QML_RETRY_MULTIPLIER`: must be finite and > 0, got {}",
                qml_retry_multiplier
            );
        }
        let qml_retry_max_seconds = parse_var("QML_RETRY_MAX_SECONDS", 60u32)?;
        let aws_region = var("AWS_REGION").unwrap_or_else(|_| "ap-southeast-1".to_string());
        let server_host = var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let server_port = parse_var("SERVER_PORT", 8000u16)?;

        let allow_image_types_raw =
            var("ALLOW_IMAGE_TYPES").unwrap_or_else(|_| "image/jpeg,image/png".to_string());
        let allow_image_types = allow_image_types_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Ok(Settings {
            database_url,
            qml_database_url,
            qml_worker_count,
            qml_batch_size,
            qml_retry_max_attempts,
            qml_retry_base_seconds,
            qml_retry_multiplier,
            qml_retry_max_seconds,
            aws_region,
            server_host,
            server_port,
            s3_profile_bucket: required_var("S3_PROFILE_BUCKET")?,
            cloudfront_base_endpoint: required_var("CLOUDFRONT_BASE_ENDPOINT")?,
            cloudfront_private_key: required_var("CLOUDFRONT_PRIVATE_KEY")?,
            cloudfront_key_id: required_var("CLOUDFRONT_KEY_ID")?,
            image_expired_in_sec: parse_var("IMAGE_EXPIRED_IN_SEC", 3600u32)?,
            sns_user_profile_changed: var("SNS_USER_PROFILE_CHANGED")
                .unwrap_or_default(),
            sns_email_sent_requested: var("SNS_EMAIL_SENT_REQUESTED")
                .unwrap_or_default(),
            sns_user_identity_linked_changed: var("SNS_USER_IDENTITY_LINKED_CHANGED")
                .unwrap_or_default(),
            sns_user_the1_get_profile_updated: var("SNS_USER_THE1_GET_PROFILE_UPDATED")
                .unwrap_or_default(),
            the1_proxy_service_url: required_var("THE1_PROXY_SERVICE_URL")?,
            sms_proxy_service_url: required_var("SMS_PROXY_SERVICE_URL")?,
            phone_number_format: var("PHONE_NUMBER_FORMAT")
                .unwrap_or_else(|_| "+66".to_string()),
            otp_text: var("OTP_TEXT")
                .unwrap_or_else(|_| "Your OTP is {otp}".to_string()),
            otp_expired_time: parse_var("OTP_EXPIRED_TIME", 5u32)?,
            jwt_secret_key: required_var("JWT_SECRET_KEY")?,
            country_code: var("COUNTRY_CODE").unwrap_or_else(|_| "66".to_string()),
            profile_change_expired_time: parse_var("PROFILE_CHANGE_EXPIRED_TIME", 30u32)?,
            token_expired_time: parse_var("TOKEN_EXPIRED_TIME", 15u32)?,
            allow_image_types,
            max_image_size_mb: parse_var("MAX_IMAGE_SIZE_MB", 5u32)?,
            image_prefix: var("IMAGE_PREFIX").unwrap_or_else(|_| "profile-images".to_string()),
        })
    }
}

fn required_var(name: &str) -> anyhow::Result<String> {
    var(name).map_err(|_| anyhow::anyhow!("environment variable `{}` is required", name))
}

fn parse_var<T>(name: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr + fmt::Display,
    T::Err: fmt::Display,
{
    match var(name) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("invalid value for `{}`: {}", name, e)),
        Err(_) => Ok(default),
    }
}
