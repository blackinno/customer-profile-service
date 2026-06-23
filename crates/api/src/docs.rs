use crate::responses::ApiResponse;
use crate::routers::{LivenessResponse, ReadinessResponse, ReadinessServices};
use utoipa::OpenApi;

pub const TAG_HEALTH: &str = "Health";

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routers::liveness,
        crate::routers::readiness,
    ),
    components(
        schemas(
            LivenessResponse,
            ReadinessResponse,
            ReadinessServices,
            ApiResponse<serde_json::Value>,
        )
    ),
    tags(
        (name = TAG_HEALTH, description = "Health check endpoints"),
    ),
    info(
        title = "Customer Profile Service API",
        version = "1.0.0",
        description = "Customer profile service built with Rust and Axum",
    ),
    servers(
        (url = "http://localhost:8000", description = "Local server"),
    )
)]
pub struct ApiDoc;
