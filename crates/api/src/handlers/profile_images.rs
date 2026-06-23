use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;

/// `POST /v1/customers/me/profile-images`
///
/// Accepts a `multipart/form-data` request with a single field named `file`.
/// Validates the content type and size, uploads to object storage, and returns
/// a signed CDN URL for the stored image.
pub async fn upload_profile_image(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut data: Vec<u8> = Vec::new();
    let mut content_type = String::from("image/jpeg");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
    {
        if field.name() == Some("file") {
            content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            data = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(e.to_string()))?
                .to_vec();
        }
    }

    let file_size = data.len();
    let response = state
        .use_cases
        .profile_images
        .upload(user_uuid, data, content_type, file_size)
        .await?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

/// `GET /v1/customers/me/profile-images`
///
/// Returns a signed CDN URL for the authenticated customer's current profile image.
/// Responds with 404 if no profile image has been uploaded.
pub async fn get_profile_image(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let response = state.use_cases.profile_images.get_image(user_uuid).await?;

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// `DELETE /v1/customers/me/profile-images`
///
/// Removes the authenticated customer's profile image from object storage and
/// clears the reference in the database. Responds with 204 No Content on success.
pub async fn delete_profile_image(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    state
        .use_cases
        .profile_images
        .delete_image(user_uuid)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
