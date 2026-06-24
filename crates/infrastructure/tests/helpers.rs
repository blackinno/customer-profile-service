use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Build a PgPool from DATABASE_URL for use in integration tests.
pub async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");
    PgPool::connect(&url)
        .await
        .expect("failed to connect to test database")
}

/// Insert a minimal `users` + `user_profiles` row and return the user UUID.
/// Uses a random UUID so parallel tests never collide.
pub async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("inttest-{}@example.com", id);

    sqlx::query(
        "INSERT INTO users \
         (id, email, locale, has_consent, is_deleted, created_at, updated_at) \
         VALUES ($1, $2, 'en'::locale_enum, true, false, $3, $3)",
    )
    .bind(id)
    .bind(&email)
    .bind(Utc::now())
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user: users insert failed: {e}"));

    sqlx::query(
        "INSERT INTO user_profiles (user_uuid, created_at, updated_at) \
         VALUES ($1, $2, $2)",
    )
    .bind(id)
    .bind(Utc::now())
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user: user_profiles insert failed: {e}"));

    id
}

/// Remove all rows seeded by a test, keyed by user UUID. Call at the end of
/// each test to keep the shared test database tidy.
pub async fn cleanup_user(pool: &PgPool, user_uuid: Uuid) {
    // FK cascades handle child rows (profile_changes, identities, the1_users, tiers)
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_uuid)
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("cleanup_user failed: {e}"));
}
