use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::middleware::error_handler::AppError;
use crate::responses::ApiResponse;
use crate::routers::AppState;

#[derive(Deserialize)]
pub struct SegmentQuery {
    pub card_number: String,
}

/// `GET /v1/customers/segments?card_number=<str>`
///
/// Looks up a The1 member by card number, upserts the local record, and
/// returns the member's primary segment.
pub async fn get_segment(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SegmentQuery>,
) -> Result<impl IntoResponse, AppError> {
    let segment = state.use_cases.segments.get_segment(q.card_number).await?;
    Ok(Json(ApiResponse::success(segment)))
}
