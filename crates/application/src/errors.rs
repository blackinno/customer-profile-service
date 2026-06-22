use domain::errors::RepositoryError;
use domain::events::DispatchError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Validation error: {0}")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("External service error: {0}")]
    External(String),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Dispatch error: {0}")]
    Dispatch(#[from] DispatchError),
}
