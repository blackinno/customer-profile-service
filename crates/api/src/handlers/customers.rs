use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::error_handler::AppError;
use crate::middleware::user_uuid::UserUuid;
use crate::responses::ApiResponse;
use crate::routers::AppState;
use application::customers::dtos::{CreateCustomerRequest, SearchCustomerQuery, UpdateCustomerRequest};

/// POST /customers — create a new customer account.
pub async fn create_customer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.create(body).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(customer))))
}

/// GET /customers — search customers by one query field.
pub async fn search_customers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchCustomerQuery>,
) -> Result<impl IntoResponse, AppError> {
    let customers = state.use_cases.customers.search(query).await?;
    Ok(Json(ApiResponse::success(customers)))
}

/// GET /customers/me — fetch the currently authenticated customer's own profile.
pub async fn get_me(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.get_me(user_uuid).await?;
    Ok(Json(ApiResponse::success(customer)))
}

/// PUT /customers/me — update the authenticated customer's own profile.
pub async fn update_me(
    State(state): State<Arc<AppState>>,
    UserUuid(user_uuid): UserUuid,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.update_me(user_uuid, body).await?;
    Ok(Json(ApiResponse::success(customer)))
}

/// GET /customers/{id} — fetch a customer by internal UUID (internal/admin use).
pub async fn get_customer_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(customer)))
}

/// DELETE /customers/{id} — soft-delete a customer account.
pub async fn delete_customer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.use_cases.customers.delete(id).await?;
    Ok(Json(ApiResponse::success(customer)))
}
