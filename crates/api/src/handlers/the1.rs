use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::middleware::error_handler::AppError;
use crate::responses::ApiResponse;
use crate::routers::AppState;

#[derive(Deserialize)]
pub struct The1AccountQuery {
    pub user_uuid: Uuid,
}

/// `GET /v1/customers/the1/account?user_uuid=<uuid>`
///
/// Internal endpoint (no user-auth header required). Returns the The1 account
/// linked to the given platform user UUID.
pub async fn get_the1_account(
    State(state): State<Arc<AppState>>,
    Query(q): Query<The1AccountQuery>,
) -> Result<impl IntoResponse, AppError> {
    let account = state.use_cases.the1.get_the1_account(q.user_uuid).await?;
    Ok(Json(ApiResponse::success(account)))
}
