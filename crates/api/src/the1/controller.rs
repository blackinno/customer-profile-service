use crate::middleware::error_handler::AppError;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

pub const THE1_BASE_PATH: &str = "/customers/the1";

#[derive(Deserialize)]
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

pub async fn get_the1_account(
    State(state): State<Arc<AppState>>,
    Query(q): Query<The1AccountQuery>,
) -> Result<impl IntoResponse, AppError> {
    let account = state.use_cases.the1.get_the1_account(q.user_uuid).await?;
    Ok(Json(ApiResponse::success(account)))
}
