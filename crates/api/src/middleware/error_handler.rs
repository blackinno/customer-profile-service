use application::errors::ApplicationError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::errors::RepositoryError;
use serde::Serialize;
use std::fmt;
use tracing::error;
use validator::ValidationErrors;

#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_url: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
}

/// HTTP error type for handler return values.
///
/// Handlers return `Result<impl IntoResponse, AppError>`. The `?` operator
/// on `ApplicationError` auto-converts via `From<ApplicationError> for AppError`.
#[derive(Debug)]
pub struct AppError {
    pub status_code: StatusCode,
    pub type_url: String,
    pub title: String,
    pub detail: String,
}

impl AppError {
    pub fn new(
        status_code: StatusCode,
        type_url: impl Into<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            status_code,
            type_url: type_url.into(),
            title: title.into(),
            detail: detail.into(),
        }
    }

    pub fn validation_error(validation_errors: ValidationErrors) -> Self {
        Self {
            status_code: StatusCode::UNPROCESSABLE_ENTITY,
            type_url: "https://datatracker.ietf.org/doc/html/rfc4918#section-11.2".to_string(),
            title: "Validation Error".to_string(),
            detail: format!("{}", validation_errors),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::NOT_FOUND,
            type_url: "https://datatracker.ietf.org/doc/html/rfc7231#section-6.5.4".to_string(),
            title: "Resource Not Found".to_string(),
            detail: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::CONFLICT,
            type_url: "https://datatracker.ietf.org/doc/html/rfc7231#section-6.5.8".to_string(),
            title: "Conflict".to_string(),
            detail: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            type_url: "https://datatracker.ietf.org/doc/html/rfc7235#section-3.1".to_string(),
            title: "Unauthorized".to_string(),
            detail: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
            type_url: "https://datatracker.ietf.org/doc/html/rfc7231#section-6.5.1".to_string(),
            title: "Bad Request".to_string(),
            detail: message.into(),
        }
    }

    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::BAD_GATEWAY,
            type_url: "https://datatracker.ietf.org/doc/html/rfc7231#section-6.6.3".to_string(),
            title: "Bad Gateway".to_string(),
            detail: message.into(),
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            type_url: "https://datatracker.ietf.org/doc/html/rfc7231#section-6.6.1".to_string(),
            title: "Internal Server Error".to_string(),
            detail: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.status_code, self.detail)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let problem = ProblemDetails {
            type_url: self.type_url,
            title: self.title,
            status: self.status_code.as_u16(),
            detail: self.detail,
        };
        (self.status_code, Json(problem)).into_response()
    }
}

/// Allows `?` in handlers to convert `ApplicationError` into `AppError`.
impl From<ApplicationError> for AppError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::ValidationError(e) => AppError::validation_error(e),
            ApplicationError::NotFound(message) => AppError::not_found(message),
            ApplicationError::BadRequest(message) => AppError::bad_request(message),
            ApplicationError::BusinessRuleViolation(message) => AppError::bad_request(message),
            ApplicationError::External(message) => {
                error!("External service error: {}", message);
                AppError::bad_gateway(message)
            }
            ApplicationError::Repository(e) => match e {
                RepositoryError::NotFound(m) => AppError::not_found(m),
                RepositoryError::Conflict(m) => AppError::conflict(m),
                RepositoryError::Backend(m) => {
                    error!("Repository backend error: {}", m);
                    AppError::internal_error("Internal server error")
                }
            },
            ApplicationError::Dispatch(e) => {
                error!("Dispatch error: {}", e);
                AppError::internal_error("Internal server error")
            }
        }
    }
}
