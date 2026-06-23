use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::profile_changes::dtos::{
    CreateProfileChangeRequest, UpdateProfileChangeRequest, VerifyProfileChangeRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, put},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

pub const PROFILE_CHANGES_PATH: &str = "/customers/me/profile-changes";

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            PROFILE_CHANGES_PATH,
            Router::new()
                .route("/", post(create_profile_change))
                .route("/{profile_id}", put(update_profile_change))
                .route("/{profile_id}/verify", post(verify_profile_change))
                .route("/{profile_id}/confirm", post(confirm_profile_change)),
        )
        .with_state(state)
}

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
