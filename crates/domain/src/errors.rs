use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),
}

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Repository backend error: {0}")]
    Backend(String),
}
