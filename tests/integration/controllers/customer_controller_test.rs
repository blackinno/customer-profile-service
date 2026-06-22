use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use domain::entities::customer::{Customer, CustomerProfile, Locale};

use crate::integration::helpers::{create_test_app, send_request, InMemoryCustomerRepository};

// ---- helpers ----

fn sample_customer(email: Option<&str>, phone: Option<&str>) -> Customer {
    let now = Utc::now();
    let id = Uuid::new_v4();
    Customer {
        id,
        email: email.map(str::to_string),
        phone: phone.map(str::to_string),
        email_verified: false,
        phone_verified: false,
        locale: Locale::Th,
        has_consent: false,
        is_deleted: false,
        client_id: None,
        created_at: now,
        updated_at: now,
        profile: Some(CustomerProfile {
            id: Uuid::new_v4(),
            user_uuid: id,
            first_name: Some("Jane".to_string()),
            last_name: Some("Doe".to_string()),
            birthdate: None,
            gender: None,
            profile_image: None,
            nationality: None,
            created_at: now,
            updated_at: now,
        }),
    }
}

fn json_body(value: serde_json::Value) -> Body {
    Body::from(serde_json::to_vec(&value).unwrap())
}

// ============================================================
// POST /customers
// ============================================================

#[tokio::test]
async fn post_customers_happy_path() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/customers")
        .header(header::CONTENT_TYPE, "application/json")
        .body(json_body(json!({
            "email": "new@example.com",
            "phone": "0812345678",
            "locale": "en",
            "has_consent": true,
            "first_name": "John",
            "last_name": "Doe"
        })))
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["email"], "new@example.com");
    // Phone should be normalised
    assert_eq!(body["data"]["phone"], "+66812345678");
    assert_eq!(body["data"]["locale"], "en");
}

#[tokio::test]
async fn post_customers_no_phone_no_email() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/customers")
        .header(header::CONTENT_TYPE, "application/json")
        .body(json_body(json!({})))
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn post_customers_duplicate_email_returns_400() {
    let existing = sample_customer(Some("taken@example.com"), None);
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(existing));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/customers")
        .header(header::CONTENT_TYPE, "application/json")
        .body(json_body(json!({ "email": "taken@example.com" })))
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["detail"]
        .as_str()
        .map(|s| s.contains("email"))
        .unwrap_or(false));
}

#[tokio::test]
async fn post_customers_duplicate_phone_returns_400() {
    let existing = sample_customer(None, Some("+66811111111"));
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(existing));
    let app = create_test_app(repo);

    // "0811111111" normalises to "+66811111111" which is already taken
    let req = Request::builder()
        .method(Method::POST)
        .uri("/customers")
        .header(header::CONTENT_TYPE, "application/json")
        .body(json_body(json!({ "phone": "0811111111" })))
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================
// GET /customers/{id}
// ============================================================

#[tokio::test]
async fn get_customer_by_id_happy_path() {
    let customer = sample_customer(Some("found@example.com"), None);
    let id = customer.id;
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(customer));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/customers/{}", id))
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], id.to_string());
}

#[tokio::test]
async fn get_customer_by_id_not_found_returns_404() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/customers/{}", Uuid::new_v4()))
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_customer_by_id_invalid_uuid_returns_error() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers/not-a-valid-uuid")
        .body(Body::empty())
        .unwrap();

    // Axum rejects the invalid UUID before reaching the handler
    let (status, _body) = send_request(app, req).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "unexpected status: {}",
        status
    );
}

// ============================================================
// GET /customers/me
// ============================================================

#[tokio::test]
async fn get_me_happy_path() {
    let customer = sample_customer(Some("me@example.com"), None);
    let user_uuid = customer.id;
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(customer));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers/me")
        .header("user_uuid", user_uuid.to_string())
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], user_uuid.to_string());
}

#[tokio::test]
async fn get_me_missing_header_returns_401() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers/me")
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_me_invalid_uuid_header_returns_400() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers/me")
        .header("user_uuid", "not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_me_not_found_returns_404() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers/me")
        .header("user_uuid", Uuid::new_v4().to_string())
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================
// PUT /customers/me
// ============================================================

#[tokio::test]
async fn put_me_happy_path() {
    let customer = sample_customer(Some("update@example.com"), None);
    let user_uuid = customer.id;
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(customer));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/customers/me")
        .header(header::CONTENT_TYPE, "application/json")
        .header("user_uuid", user_uuid.to_string())
        .body(json_body(json!({
            "first_name": "Updated",
            "locale": "en"
        })))
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["first_name"], "Updated");
    assert_eq!(body["data"]["locale"], "en");
}

#[tokio::test]
async fn put_me_missing_header_returns_401() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/customers/me")
        .header(header::CONTENT_TYPE, "application/json")
        .body(json_body(json!({ "first_name": "Oops" })))
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================
// DELETE /customers/{id}
// ============================================================

#[tokio::test]
async fn delete_customer_happy_path() {
    let customer = sample_customer(Some("todelete@example.com"), Some("+66822222222"));
    let id = customer.id;
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(customer));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/customers/{}", id))
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["is_deleted"], true);
}

#[tokio::test]
async fn delete_customer_not_found_returns_404() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/customers/{}", Uuid::new_v4()))
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================
// GET /customers (search)
// ============================================================

#[tokio::test]
async fn search_by_phone_returns_customer() {
    let customer = sample_customer(None, Some("+66833333333"));
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(customer));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers?phone=%2B66833333333") // URL-encode the +
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    let data = body["data"].as_array().expect("data should be array");
    assert_eq!(data.len(), 1);
}

#[tokio::test]
async fn search_no_params_returns_400() {
    let repo = Arc::new(InMemoryCustomerRepository::new());
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/customers")
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_by_id_returns_customer() {
    let customer = sample_customer(Some("searchid@example.com"), None);
    let id = customer.id;
    let repo = Arc::new(InMemoryCustomerRepository::with_customer(customer));
    let app = create_test_app(repo);

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/customers?id={}", id))
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    let data = body["data"].as_array().expect("data should be array");
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["id"], id.to_string());
}
