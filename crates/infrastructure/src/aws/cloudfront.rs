use application::profile_images::use_cases::UrlSigner;
use base64::{Engine, engine::general_purpose::STANDARD};
use rsa::{Pkcs1v15Sign, RsaPrivateKey, pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey};
use sha1::{Digest, Sha1};

pub struct CloudFrontSigner {
    private_key: RsaPrivateKey,
    key_id: String,
    base_url: String,
    expires_in_secs: u32,
}

impl CloudFrontSigner {
    pub fn new(
        pem: &str,
        key_id: String,
        base_url: String,
        expires_in_secs: u32,
    ) -> anyhow::Result<Self> {
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

        let digest = Sha1::new().chain_update(policy.as_bytes()).finalize();
        let sig = self
            .private_key
            .sign(Pkcs1v15Sign::new::<Sha1>(), &digest)
            .map_err(|e| e.to_string())?;

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
