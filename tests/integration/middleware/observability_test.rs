use api::middleware::apply_observability_layers;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use tower::ServiceExt;
use uuid::Uuid;

fn test_app() -> Router {
    let inner = Router::new().route("/ping", get(|| async { "pong" }));
    apply_observability_layers(inner)
}

async fn send(app: Router, req: Request<Body>) -> (StatusCode, String) {
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let request_id = res
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    (status, request_id)
}

#[tokio::test]
async fn auto_generates_uuid_when_header_missing() {
    let req = Request::builder().uri("/ping").body(Body::empty()).unwrap();

    let (status, request_id) = send(test_app(), req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        Uuid::parse_str(&request_id).is_ok(),
        "expected a UUID in x-request-id, got `{}`",
        request_id
    );
}

#[tokio::test]
async fn propagates_inbound_request_id() {
    let req = Request::builder()
        .uri("/ping")
        .header("x-request-id", "client-supplied-abc")
        .body(Body::empty())
        .unwrap();

    let (status, request_id) = send(test_app(), req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(request_id, "client-supplied-abc");
}

#[tokio::test]
async fn empty_inbound_request_id_is_replaced() {
    let req = Request::builder()
        .uri("/ping")
        .header("x-request-id", "")
        .body(Body::empty())
        .unwrap();

    let (status, request_id) = send(test_app(), req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !request_id.is_empty(),
        "empty inbound request id should be replaced with a generated UUID"
    );
    assert!(
        Uuid::parse_str(&request_id).is_ok(),
        "expected a UUID after replacing empty header, got `{}`",
        request_id
    );
}
