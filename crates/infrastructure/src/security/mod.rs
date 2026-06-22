//! Authentication / authorization seam.
//!
//! This module is intentionally a skeleton. The template does not bundle a
//! specific auth strategy — pick what fits (JWT, opaque session, mTLS, etc.)
//! and wire it here.
//!
//! ## Suggested wiring
//!
//! 1. Add a request extractor for the authenticated principal:
//!    ```ignore
//!    use axum::{extract::FromRequestParts, http::request::Parts};
//!
//!    pub struct AuthUser {
//!        pub id: uuid::Uuid,
//!        pub email: String,
//!    }
//!
//!    #[axum::async_trait]
//!    impl<S> FromRequestParts<S> for AuthUser
//!    where
//!        S: Send + Sync,
//!    {
//!        type Rejection = AuthError;
//!        async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
//!            let token = parts
//!                .headers
//!                .get(axum::http::header::AUTHORIZATION)
//!                .and_then(|v| v.to_str().ok())
//!                .and_then(|v| v.strip_prefix("Bearer "))
//!                .ok_or(AuthError::Missing)?;
//!            // verify token, look up user, populate AuthUser
//!            todo!()
//!        }
//!    }
//!    ```
//!
//! 2. Apply via `axum::middleware::from_extractor` or by adding `AuthUser` as
//!    a handler parameter on protected routes.
//!
//! 3. For role-based access, add a `RequireRole(Role)` extractor that reads
//!    `AuthUser` and rejects with 403 when the role does not match.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("missing authentication credentials")]
    Missing,

    #[error("invalid authentication credentials")]
    Invalid,

    #[error("insufficient permissions")]
    Forbidden,
}
