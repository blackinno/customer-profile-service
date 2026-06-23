use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use application::identities::dtos::CreateIdentityRequest;
use crate::{
    middleware::{error_handler::AppError, user_uuid::UserUuid},
    responses::ApiResponse,
    routers::AppState,
};

/// `GET /v1/customers/me/identities`
///
/// Return all active provider identities linked to the authenticated user.
pub async fn get_my_identities(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let identities = state.use_cases.identities.get_identities(user_uuid).await?;
    Ok(Json(ApiResponse::success(identities)))
}

/// `GET /v1/customers/:user_uuid/identities`
///
/// Internal/admin route — returns identities for any user without requiring
/// the `X-User-UUID` header (auth is handled upstream by the API gateway).
pub async fn get_identities_internal(
    State(state): State<Arc<AppState>>,
    Path(user_uuid): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let identities = state
        .use_cases
        .identities
        .get_identities_internal(user_uuid)
        .await?;
    Ok(Json(ApiResponse::success(identities)))
}

/// `POST /v1/customers/me/identities`
///
/// Link a new provider identity to the authenticated user.  Returns the
/// created (or restored) identity.
pub async fn create_identity(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Json(req): Json<CreateIdentityRequest>,
) -> Result<impl IntoResponse, AppError> {
    let identity = state
        .use_cases
        .identities
        .create_identity(user_uuid, req)
        .await?;
    Ok(Json(ApiResponse::success(identity)))
}

/// `DELETE /v1/customers/me/identities/:provider/:external_id`
///
/// Soft-delete the specified provider identity that belongs to the
/// authenticated user and record the action in the audit log.
pub async fn delete_identity(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Path((provider, external_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    state
        .use_cases
        .identities
        .delete_identity(user_uuid, provider, external_id)
        .await?;
    Ok(Json(ApiResponse::success(())))
}

/// `POST /v1/customers/me/identities/:provider_name/invoke`
///
/// Return the stored provider tokens for the given identity.  The live
/// The1 token-refresh call is wired in Task 24.
pub async fn invoke_token(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Path(provider_name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let tokens = state
        .use_cases
        .identities
        .invoke_token(user_uuid, provider_name)
        .await?;
    Ok(Json(ApiResponse::success(tokens)))
}
