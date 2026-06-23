//! HTTP observability — request IDs, structured tracing, Prometheus metrics.
//!
//! Wiring lives here so `routers.rs` stays focused on routing. `main.rs` calls
//! [`install_metrics_recorder`] once at startup; the rest is plumbed through
//! [`apply_observability_layers`].

use axum::{
    Router,
    body::Body,
    extract::{MatchedPath, Request, State},
    http::{HeaderName, Response},
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::{Duration, Instant};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::Span;

/// Header name used for inbound request IDs and outbound propagation.
const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Histogram buckets in seconds — covers sub-millisecond to ~10s requests.
const HTTP_DURATION_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Initialize the global Prometheus recorder and return a render handle.
///
/// Call exactly once at startup, before any `metrics::counter!` /
/// `metrics::histogram!` invocations. The returned [`PrometheusHandle`] is
/// what the `/metrics` endpoint renders.
pub fn install_metrics_recorder() -> Result<PrometheusHandle, anyhow::Error> {
    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full("http_requests_duration_seconds".into()),
            HTTP_DURATION_BUCKETS,
        )?
        .install_recorder()?;
    Ok(handle)
}

/// Apply request-id, structured tracing, and per-request metrics layers.
///
/// Order matters — outermost layer runs first on the request and last on the
/// response. We want the request id set before the trace span is created, and
/// the metrics middleware to see the final status / latency.
///
/// All HTTP traffic that should appear in `http_requests_total` /
/// `http_requests_duration_seconds` must be merged into the router before
/// passing it through this function. Routes mounted *outside* the wrapper get
/// no request id, no structured span, and no metrics.
pub fn apply_observability_layers(router: Router) -> Router {
    router
        .layer(middleware::from_fn(record_http_metrics))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<Body>| {
                    let request_id = req
                        .headers()
                        .get(&REQUEST_ID_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    let route = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str)
                        .unwrap_or("");
                    tracing::info_span!(
                        "http_request",
                        method = %req.method(),
                        uri = %req.uri(),
                        route = %route,
                        request_id = %request_id,
                        status = tracing::field::Empty,
                        latency_ms = tracing::field::Empty,
                    )
                })
                .on_response(|res: &Response<Body>, latency: Duration, span: &Span| {
                    span.record("status", res.status().as_u16());
                    span.record("latency_ms", latency.as_millis() as u64);
                    tracing::info!(
                        status = res.status().as_u16(),
                        latency_ms = latency.as_millis() as u64,
                        "request completed"
                    );
                }),
        )
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER.clone()))
        .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid))
        // Strip empty x-request-id headers so SetRequestIdLayer (which only
        // generates when the header is *absent*) sees a clean request and
        // injects a fresh UUID. Without this, a client sending
        // `x-request-id:` (empty) would produce empty span fields.
        .layer(middleware::from_fn(strip_empty_request_id))
}

async fn strip_empty_request_id(mut req: Request, next: Next) -> Response<Body> {
    let is_empty = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .map(|v| v.as_bytes().is_empty())
        .unwrap_or(false);
    if is_empty {
        req.headers_mut().remove(&REQUEST_ID_HEADER);
    }
    next.run(req).await
}

/// Mount `GET /metrics` on `router` with the supplied Prometheus handle.
pub fn metrics_router(handle: PrometheusHandle) -> Router {
    Router::new()
        .route("/metrics", get(render_metrics))
        .with_state(handle)
}

async fn render_metrics(State(handle): State<PrometheusHandle>) -> impl IntoResponse {
    handle.render()
}

/// Per-request middleware: records `http_requests_total` and
/// `http_requests_duration_seconds` with `method` / `route` / `status` labels.
async fn record_http_metrics(req: Request, next: Next) -> Response<Body> {
    let method = req.method().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "<unmatched>".to_string());
    let started = Instant::now();

    let response = next.run(req).await;
    let elapsed = started.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::counter!(
        "http_requests_total",
        "method" => method.clone(),
        "route" => route.clone(),
        "status" => status.clone(),
    )
    .increment(1);
    metrics::histogram!(
        "http_requests_duration_seconds",
        "method" => method,
        "route" => route,
        "status" => status,
    )
    .record(elapsed);

    response
}
