use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::profile_changes::dtos::{
    CreateProfileChangeRequest, ProfileChangeResponse, UpdateProfileChangeRequest,
    VerifyProfileChangeRequest,
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

#[utoipa::path(
    post,
    path = "/v1/customers/me/profile-changes",
    request_body = CreateProfileChangeRequest,
    params(
        ("x-user-uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 201, description = "Created", body = inline(crate::responses::ApiResponse<ProfileChangeResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "ProfileChanges",
)]
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

#[utoipa::path(
    put,
    path = "/v1/customers/me/profile-changes/{profile_id}",
    request_body = UpdateProfileChangeRequest,
    params(
        ("profile_id" = Uuid, Path, description = "Profile change request ID"),
        ("x-user-uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 200, description = "OK", body = inline(crate::responses::ApiResponse<ProfileChangeResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    tag = "ProfileChanges",
)]
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

#[utoipa::path(
    post,
    path = "/v1/customers/me/profile-changes/{profile_id}/verify",
    request_body = VerifyProfileChangeRequest,
    params(
        ("profile_id" = Uuid, Path, description = "Profile change request ID"),
        ("x-user-uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 200, description = "OK", body = inline(crate::responses::ApiResponse<ProfileChangeResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    tag = "ProfileChanges",
)]
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

#[utoipa::path(
    post,
    path = "/v1/customers/me/profile-changes/{profile_id}/confirm",
    params(
        ("profile_id" = Uuid, Path, description = "Profile change request ID"),
        ("x-user-uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 200, description = "OK", body = inline(crate::responses::ApiResponse<ProfileChangeResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    tag = "ProfileChanges",
)]
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
