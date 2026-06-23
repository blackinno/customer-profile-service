use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::profile_changes::dtos::{
    CreateProfileChangeRequest, UpdateProfileChangeRequest, VerifyProfileChangeRequest,
};

/// POST /v1/customers/me/profile-changes — begin a phone or email change flow.
pub async fn create_profile_change(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Json(body): Json<CreateProfileChangeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .use_cases
        .profile_changes
        .create_profile_change(user_uuid, body)
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(record))))
}

/// PUT /v1/customers/me/profile-changes/{profile_id} — resend OTP.
pub async fn update_profile_change(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Path(profile_id): Path<Uuid>,
    Json(body): Json<UpdateProfileChangeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .use_cases
        .profile_changes
        .update_profile_change(user_uuid, profile_id, body)
        .await?;
    Ok(Json(ApiResponse::success(record)))
}

/// POST /v1/customers/me/profile-changes/{profile_id}/verify — verify OTP.
pub async fn verify_profile_change(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Path(profile_id): Path<Uuid>,
    Json(body): Json<VerifyProfileChangeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .use_cases
        .profile_changes
        .verify_profile_change(user_uuid, profile_id, body)
        .await?;
    Ok(Json(ApiResponse::success(record)))
}

/// POST /v1/customers/me/profile-changes/{profile_id}/confirm — finalise the change.
pub async fn confirm_profile_change(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Path(profile_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .use_cases
        .profile_changes
        .confirm_profile_change(user_uuid, profile_id)
        .await?;
    Ok(Json(ApiResponse::success(record)))
}
