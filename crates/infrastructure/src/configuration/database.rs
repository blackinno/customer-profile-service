use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_connection_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(30))
        .connect(database_url)
        .await
}

/// Run all pending SQLx migrations against `pool`.
///
/// Migrations are embedded at compile time via `sqlx::migrate!`, so the binary
/// is self-contained — no `migrations/` directory needs to ship alongside it.
///
/// Note: every replica races to migrate on startup. SQLx uses an advisory lock
/// internally so concurrent runs are safe, but for large schema changes prefer
/// running migrations as a separate deploy step (e.g. a Kubernetes Job).
pub async fn migration(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    tracing::info!("Running embedded migrations");
    sqlx::migrate!("../../migrations").run(pool).await
}
