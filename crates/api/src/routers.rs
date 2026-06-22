use crate::docs::{ApiDoc, TAG_HEALTH};
use crate::middleware::{apply_observability_layers, metrics_router};
use application::UseCases;
use axum::http::StatusCode;
use axum::response::Json;
use axum::{Router, extract::State, routing::get};
use infrastructure::AppFactoryState;
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// Maximum request body size (1 MiB). Override per-route if larger payloads
/// are expected (e.g. file upload endpoints).
const REQUEST_BODY_LIMIT: usize = 1024 * 1024;

/// Default request timeout. Slow-loris and slow-write attacks otherwise hold
/// worker tasks open indefinitely.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Routers;

impl Routers {
    pub fn init_routers(app_state: AppFactoryState, metrics_handle: PrometheusHandle) -> Router {
        let liveness_router = Router::new().route("/livez", get(liveness));

        let readiness_router = Router::new()
            .route("/readyz", get(readiness))
            // Keep /healthz for backwards compatibility — points at readiness.
            .route("/healthz", get(readiness))
            .with_state(app_state.pool.clone());

        let swagger_router =
            SwaggerUi::new("/swagger").url("/swagger/openapi.json", ApiDoc::openapi());

        let state = Arc::new(AppState::new(app_state));

        // CORS scaffold — by default no CORS layer is mounted. To allow a
        // browser frontend, uncomment and configure with explicit origins:
        //
        //   use tower_http::cors::{Any, CorsLayer};
        //   let cors = CorsLayer::new()
        //       .allow_origin(["https://app.example.com".parse().unwrap()])
        //       .allow_methods(Any)
        //       .allow_headers(Any);
        //   .layer(cors)

        let app = Router::new()
            .merge(swagger_router)
            .merge(liveness_router)
            .merge(readiness_router)
            .merge(metrics_router(metrics_handle))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                REQUEST_TIMEOUT,
            ))
            .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT));

        apply_observability_layers(app)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub use_cases: UseCases,
}

impl AppState {
    pub fn new(state: AppFactoryState) -> AppState {
        Self {
            use_cases: state.use_cases,
        }
    }
}

#[derive(serde::Serialize, ToSchema)]
pub struct LivenessResponse {
    pub status: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct ReadinessResponse {
    pub status: String,
    pub timestamp: String,
    pub services: ReadinessServices,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
pub struct ReadinessServices {
    pub api: String,
    pub database: String,
}

#[utoipa::path(
    get,
    path = "/livez",
    responses(
        (status = 200, description = "Process is alive", body = LivenessResponse),
    ),
    tag = TAG_HEALTH,
)]
pub async fn liveness() -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "ok".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/readyz",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready", body = ReadinessResponse),
    ),
    tag = TAG_HEALTH,
)]
pub async fn readiness(State(db_pool): State<PgPool>) -> (StatusCode, Json<ReadinessResponse>) {
    let timestamp = chrono::Utc::now().to_rfc3339();

    match sqlx::query("SELECT 1").fetch_one(&db_pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ReadinessResponse {
                status: "ready".to_string(),
                timestamp,
                services: ReadinessServices {
                    api: "ok".to_string(),
                    database: "ok".to_string(),
                },
                error: None,
            }),
        ),
        Err(e) => {
            tracing::error!("Database readiness check failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadinessResponse {
                    status: "not_ready".to_string(),
                    timestamp,
                    services: ReadinessServices {
                        api: "ok".to_string(),
                        database: "error".to_string(),
                    },
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}
