use crate::middleware::error_handler::AppError;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::the1::dtos::The1AccountResponse;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

pub const THE1_BASE_PATH: &str = "/customers/the1";

#[derive(Deserialize, ToSchema)]
pub struct The1AccountQuery {
    pub user_uuid: Uuid,
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            THE1_BASE_PATH,
            Router::new().route("/account", get(get_the1_account)),
        )
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/v1/customers/the1/account",
    params(
        ("user_uuid" = Uuid, Query, description = "Customer UUID"),
    ),
    responses(
        (status = 200, description = "The1 account data", body = inline(ApiResponse<The1AccountResponse>)),
        (status = 404, description = "Account not found"),
    ),
    tag = "The1",
)]
pub async fn get_the1_account(
    State(state): State<Arc<AppState>>,
    Query(q): Query<The1AccountQuery>,
) -> Result<impl IntoResponse, AppError> {
    let account = state.use_cases.the1.get_the1_account(q.user_uuid).await?;
    Ok(Json(ApiResponse::success(account)))
}
