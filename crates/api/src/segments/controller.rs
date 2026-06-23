use crate::middleware::error_handler::AppError;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::segments::dtos::SegmentResponse;
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

pub const SEGMENTS_PATH: &str = "/customers/segments";

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            SEGMENTS_PATH,
            Router::new().route("/", get(get_segment)),
        )
        .with_state(state)
}

#[derive(Deserialize, ToSchema)]
pub struct SegmentQuery {
    pub card_number: String,
}

#[utoipa::path(
    get,
    path = "/v1/customers/segments",
    params(
        ("card_number" = String, Query, description = "The1 card number"),
    ),
    responses(
        (status = 200, description = "Segment found", body = inline(crate::responses::ApiResponse<SegmentResponse>)),
        (status = 404, description = "No tier/segment found"),
        (status = 502, description = "External The1 service error"),
    ),
    tag = "Segments",
)]
pub async fn get_segment(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SegmentQuery>,
) -> Result<impl IntoResponse, AppError> {
    let segment = state.use_cases.segments.get_segment(q.card_number).await?;
    Ok(Json(ApiResponse::success(segment)))
}
