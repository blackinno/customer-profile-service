pub mod pg_customer_repository;
pub mod pg_identity_repository;
pub mod pg_profile_change_repository;
pub mod pg_the1_user_repository;

use domain::errors::RepositoryError;
use tracing::debug;

/// Translate `sqlx::Error` into the domain repository taxonomy.
pub(crate) fn map_sqlx_error(err: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(ref db_err) = err {
        let raw = db_err.message();
        match db_err.code().as_deref() {
            Some("23505") => {
                debug!(sqlstate = "23505", raw = %raw, "unique violation");
                return RepositoryError::Conflict(
                    "resource with this unique key already exists".to_string(),
                );
            }
            Some("23503") => {
                debug!(sqlstate = "23503", raw = %raw, "foreign key violation");
                return RepositoryError::Conflict(
                    "foreign-key constraint violated (referenced row is missing or is still referenced)"
                        .to_string(),
                );
            }
            Some("23514") => {
                debug!(sqlstate = "23514", raw = %raw, "check violation");
                return RepositoryError::Conflict(
                    "value violates a database check constraint".to_string(),
                );
            }
            _ => {}
        }
    }
    RepositoryError::Backend(err.to_string())
}
