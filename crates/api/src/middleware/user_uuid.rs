use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use crate::middleware::error_handler::AppError;

/// Extracts the authenticated user's UUID from the `user_uuid` request header.
///
/// The header is expected to be set by the upstream API gateway / auth middleware
/// after token validation. Handlers that require an authenticated caller destructure
/// this extractor: `UserUuid(user_uuid): UserUuid`.
pub struct UserUuid(pub Uuid);

impl<S> FromRequestParts<S> for UserUuid
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get("user_uuid")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("missing user_uuid header"))?;

        let id = Uuid::parse_str(raw)
            .map_err(|_| AppError::bad_request("invalid user_uuid header: not a valid UUID"))?;

        Ok(UserUuid(id))
    }
}
