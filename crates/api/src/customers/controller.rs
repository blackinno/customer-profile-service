use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::customers::dtos::{
    CreateCustomerRequest, SearchCustomerQuery, UpdateCustomerRequest,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use uuid::Uuid;

pub const CUSTOMERS_PATH: &str = "/customers";

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            CUSTOMERS_PATH,
            Router::new()
                .route("/", post(create_customer).get(search_customers))
                .route("/me", get(get_me).put(update_me))
                .route("/{id}", get(get_customer_by_id).delete(delete_customer)),
        )
        .with_state(state)
}

pub async fn create_customer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.create(body).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(customer))))
}

pub async fn search_customers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchCustomerQuery>,
) -> Result<impl IntoResponse, AppError> {
    let customers = state.use_cases.customers.search(query).await?;
    Ok(Json(ApiResponse::success(customers)))
}

pub async fn get_me(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.get_me(user_uuid).await?;
    Ok(Json(ApiResponse::success(customer)))
}

pub async fn update_me(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.update_me(user_uuid, body).await?;
    Ok(Json(ApiResponse::success(customer)))
}

pub async fn get_customer_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(customer)))
}

pub async fn delete_customer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.delete(id).await?;
    Ok(Json(ApiResponse::success(customer)))
}
