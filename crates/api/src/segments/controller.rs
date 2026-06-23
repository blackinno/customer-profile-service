use crate::middleware::error_handler::AppError;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub const SEGMENTS_PATH: &str = "/customers/segments";

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            SEGMENTS_PATH,
            Router::new().route("/", get(get_segment)),
        )
        .with_state(state)
}

#[derive(Deserialize)]
pub struct SegmentQuery {
    pub card_number: String,
}

pub async fn get_segment(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SegmentQuery>,
) -> Result<impl IntoResponse, AppError> {
    let segment = state.use_cases.segments.get_segment(q.card_number).await?;
    Ok(Json(ApiResponse::success(segment)))
}
