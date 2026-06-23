use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::identities::dtos::CreateIdentityRequest;
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

pub async fn get_my_identities(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let identities = state.use_cases.identities.get_identities(user_uuid).await?;
    Ok(Json(ApiResponse::success(identities)))
}

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
