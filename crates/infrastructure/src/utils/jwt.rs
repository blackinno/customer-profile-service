use application::profile_changes::use_cases::TokenService;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct ProfileChangeClaims {
    sub: String, // profile_change_id
    user_uuid: String,
    exp: usize, // Unix timestamp
}

/// JWT-based implementation of `TokenService` for profile-change confirmation tokens.
pub struct JwtTokenService {
    secret: String,
}

impl JwtTokenService {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl TokenService for JwtTokenService {
    fn generate(
        &self,
        profile_change_id: Uuid,
        user_uuid: Uuid,
        expires_in_minutes: u32,
    ) -> Result<String, String> {
        let exp = (Utc::now() + Duration::minutes(expires_in_minutes as i64)).timestamp() as usize;
        let claims = ProfileChangeClaims {
            sub: profile_change_id.to_string(),
            user_uuid: user_uuid.to_string(),
            exp,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| e.to_string())
    }

    fn validate(&self, token: &str) -> Result<(Uuid, Uuid), String> {
        let data = decode::<ProfileChangeClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| e.to_string())?;

        let profile_change_id = Uuid::parse_str(&data.claims.sub).map_err(|e| e.to_string())?;
        let user_uuid = Uuid::parse_str(&data.claims.user_uuid).map_err(|e| e.to_string())?;
        Ok((profile_change_id, user_uuid))
    }
}
