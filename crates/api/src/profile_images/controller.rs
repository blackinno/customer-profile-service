use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use std::sync::Arc;

pub const PROFILE_IMAGES_PATH: &str = "/customers/me/profile-images";

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            PROFILE_IMAGES_PATH,
            Router::new().route(
                "/",
                post(upload_profile_image)
                    .get(get_profile_image)
                    .delete(delete_profile_image),
            ),
        )
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/v1/customers/me/profile-images",
    request_body(content = inline(String), content_type = "multipart/form-data", description = "Profile image file (field: 'file')"),
    params(
        ("user_uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 201, description = "Image uploaded", body = inline(crate::responses::ApiResponse<application::profile_images::dtos::ProfileImageResponse>)),
        (status = 400, description = "Bad request"),
        (status = 413, description = "File too large"),
    ),
    tag = "ProfileImages",
)]
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

#[utoipa::path(
    get,
    path = "/v1/customers/me/profile-images",
    params(
        ("user_uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 200, description = "OK", body = inline(crate::responses::ApiResponse<application::profile_images::dtos::ProfileImageResponse>)),
        (status = 404, description = "Not found"),
    ),
    tag = "ProfileImages",
)]
pub async fn get_profile_image(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let response = state.use_cases.profile_images.get_image(user_uuid).await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

#[utoipa::path(
    delete,
    path = "/v1/customers/me/profile-images",
    params(
        ("user_uuid" = String, Header, description = "Authenticated user UUID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "ProfileImages",
)]
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
