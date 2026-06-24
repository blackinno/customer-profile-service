#[path = "helpers.rs"]
mod helpers;

use domain::entities::identity::CreateIdentity;
use domain::repositories::identity_repository::IdentityRepository;
use infrastructure::persistence::pg_identity_repository::PgIdentityRepository;
use uuid::Uuid;

fn make_create(user_uuid: Uuid, external_id: &str) -> CreateIdentity {
    CreateIdentity {
        user_uuid,
        provider_name: "google".to_string(),
        external_id: external_id.to_string(),
        provider_id_token: Some("id-token".to_string()),
        provider_access_token: Some("access-token".to_string()),
        provider_refresh_token: Some("refresh-token".to_string()),
    }
}

// ── create / find_by_user ─────────────────────────────────────────────────────

#[tokio::test]
async fn create_and_find_by_user() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, &ext_id)).await.unwrap();
    assert_eq!(created.user_uuid, user_uuid);
    assert_eq!(created.provider_name, "google");
    assert!(!created.is_deleted);

    let list = repo.find_by_user(user_uuid).await.unwrap();
    assert!(list.iter().any(|i| i.id == created.id));

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── find_active ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn find_active_returns_existing_identity() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    repo.create(make_create(user_uuid, &ext_id)).await.unwrap();

    let found = repo.find_active(user_uuid, "google", &ext_id).await.unwrap();
    assert!(found.is_some());

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_active_returns_none_after_soft_delete() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, &ext_id)).await.unwrap();
    repo.soft_delete(created.id, user_uuid).await.unwrap();

    let found = repo.find_active(user_uuid, "google", &ext_id).await.unwrap();
    assert!(found.is_none());

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── soft_delete ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn soft_delete_marks_is_deleted() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, &ext_id)).await.unwrap();
    let deleted = repo.soft_delete(created.id, user_uuid).await.unwrap();
    assert!(deleted.is_deleted);

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── find_deleted ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn find_deleted_returns_soft_deleted_row() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, &ext_id)).await.unwrap();
    repo.soft_delete(created.id, user_uuid).await.unwrap();

    let found = repo.find_deleted("google", &ext_id).await.unwrap();
    assert!(found.is_some());
    assert!(found.unwrap().is_deleted);

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── restore ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn restore_clears_is_deleted_and_reassigns_user() {
    let pool = helpers::pool().await;
    let user_a = helpers::seed_user(&pool).await;
    let user_b = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    let created = repo.create(make_create(user_a, &ext_id)).await.unwrap();
    repo.soft_delete(created.id, user_a).await.unwrap();

    let restored = repo
        .restore(created.id, user_b, make_create(user_b, &ext_id))
        .await
        .unwrap();

    assert!(!restored.is_deleted);
    assert_eq!(restored.user_uuid, user_b);

    helpers::cleanup_user(&pool, user_a).await;
    helpers::cleanup_user(&pool, user_b).await;
}

// ── update_tokens ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_tokens_persists_new_values() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let ext_id = format!("gext-{}", Uuid::new_v4());
    let repo = PgIdentityRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, &ext_id)).await.unwrap();
    let updated = repo
        .update_tokens(
            created.id,
            Some("new-access".to_string()),
            Some("new-refresh".to_string()),
        )
        .await
        .unwrap();

    assert_eq!(updated.provider_access_token.as_deref(), Some("new-access"));
    assert_eq!(updated.provider_refresh_token.as_deref(), Some("new-refresh"));

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── log_transaction ───────────────────────────────────────────────────────────

#[tokio::test]
async fn log_transaction_inserts_audit_row() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool.clone());

    repo.log_transaction(user_uuid, "delete", "google", "gext-audit-test")
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity_provider_transactions \
         WHERE user_uuid = $1 AND action_type = 'delete'",
    )
    .bind(user_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);

    helpers::cleanup_user(&pool, user_uuid).await;
}
