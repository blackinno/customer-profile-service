use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::identities::dtos::{CreateIdentityRequest, IdentityResponse, InvokeTokenResponse};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

pub const IDENTITIES_BASE_PATH: &str = "/customers";

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            IDENTITIES_BASE_PATH,
            Router::new()
                .route(
                    "/me/identities",
                    get(get_my_identities).post(create_identity),
                )
                .route(
                    "/me/identities/{provider}/{external_id}",
                    delete(delete_identity),
                )
                .route(
                    "/me/identities/{provider_name}/invoke",
                    post(invoke_token),
                )
                .route("/{user_uuid}/identities", get(get_identities_internal)),
        )
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/v1/customers/me/identities",
    responses(
        (status = 200, description = "List of identities", body = inline(crate::responses::ApiResponse<Vec<IdentityResponse>>)),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Identities",
)]
pub async fn get_my_identities(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let identities = state.use_cases.identities.get_identities(user_uuid).await?;
    Ok(Json(ApiResponse::success(identities)))
}

#[utoipa::path(
    get,
    path = "/v1/customers/{user_uuid}/identities",
    params(
        ("user_uuid" = Uuid, Path, description = "Customer UUID"),
    ),
    responses(
        (status = 200, description = "List of identities for customer", body = inline(crate::responses::ApiResponse<Vec<IdentityResponse>>)),
        (status = 404, description = "Customer not found"),
    ),
    tag = "Identities",
)]
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

#[utoipa::path(
    post,
    path = "/v1/customers/me/identities",
    request_body = CreateIdentityRequest,
    responses(
        (status = 200, description = "Created identity", body = inline(crate::responses::ApiResponse<IdentityResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Identities",
)]
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

#[utoipa::path(
    delete,
    path = "/v1/customers/me/identities/{provider}/{external_id}",
    params(
        ("provider" = String, Path, description = "Identity provider name"),
        ("external_id" = String, Path, description = "External ID at the provider"),
    ),
    responses(
        (status = 200, description = "Identity deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Identity not found"),
    ),
    tag = "Identities",
)]
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

#[utoipa::path(
    post,
    path = "/v1/customers/me/identities/{provider_name}/invoke",
    params(
        ("provider_name" = String, Path, description = "Identity provider name"),
    ),
    responses(
        (status = 200, description = "Invoked tokens", body = inline(crate::responses::ApiResponse<InvokeTokenResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Identity not found"),
    ),
    tag = "Identities",
)]
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
