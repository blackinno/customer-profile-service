use application::profile_images::use_cases::UrlSigner;
use base64::{Engine, engine::general_purpose::STANDARD};
use rsa::{Pkcs1v15Sign, RsaPrivateKey, pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey};
use sha1::{Digest, Sha1};

/// CloudFront canned-policy signed-URL generator.
///
/// Parses a PKCS#1 PEM private key on construction and uses RSA-SHA1 (PKCS#1 v1.5)
/// to sign CloudFront canned-policy statements, following the AWS documentation:
/// <https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/private-content-creating-signed-url-canned-policy.html>
pub struct CloudFrontSigner {
    private_key: RsaPrivateKey,
    key_id: String,
    base_url: String,
    expires_in_secs: u32,
}

impl CloudFrontSigner {
    /// Create a new signer.
    ///
    /// # Arguments
    /// * `pem` — PKCS#1 RSA private key in PEM format (CloudFront key pair).
    /// * `key_id` — CloudFront key-pair ID (shown in the AWS console).
    /// * `base_url` — CloudFront distribution base URL, e.g. `https://d1234.cloudfront.net`.
    /// * `expires_in_secs` — How many seconds from now each signed URL should be valid.
    pub fn new(
        pem: &str,
        key_id: String,
        base_url: String,
        expires_in_secs: u32,
    ) -> anyhow::Result<Self> {
        // Accept both PKCS#8 ("BEGIN PRIVATE KEY") and PKCS#1 ("BEGIN RSA PRIVATE KEY").
        let private_key = RsaPrivateKey::from_pkcs8_pem(pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))?;
        Ok(Self {
            private_key,
            key_id,
            base_url,
            expires_in_secs,
        })
    }
}

impl UrlSigner for CloudFrontSigner {
    fn sign_url(&self, object_key: &str) -> Result<String, String> {
        let expires = chrono::Utc::now().timestamp() as u64 + self.expires_in_secs as u64;

        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            object_key.trim_start_matches('/')
        );

        // CloudFront canned policy JSON (must be compact / no extra whitespace)
        let policy = serde_json::json!({
            "Statement": [{
                "Resource": url,
                "Condition": {
                    "DateLessThan": {
                        "AWS:EpochTime": expires
                    }
                }
            }]
        })
        .to_string();

        // Hash the policy with SHA-1, then sign with RSA PKCS#1 v1.5.
        let digest = Sha1::new().chain_update(policy.as_bytes()).finalize();
        let sig = self
            .private_key
            .sign(Pkcs1v15Sign::new::<Sha1>(), &digest)
            .map_err(|e| e.to_string())?;

        // Base64-encode and make URL-safe per CloudFront spec.
        let sig_b64 = STANDARD.encode(&sig);
        let sig_url_safe = sig_b64
            .replace('+', "-")
            .replace('=', "_")
            .replace('/', "~");

        Ok(format!(
            "{}?Expires={}&Signature={}&Key-Pair-Id={}",
            url, expires, sig_url_safe, self.key_id
        ))
    }
}
