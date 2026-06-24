#[path = "helpers.rs"]
mod helpers;

use chrono::Utc;
use domain::entities::profile_change::{ChangeStatus, ChangeType, CreateProfileChange};
use domain::repositories::profile_change_repository::ProfileChangeRepository;
use infrastructure::persistence::pg_profile_change_repository::PgProfileChangeRepository;

fn future() -> chrono::DateTime<Utc> {
    Utc::now() + chrono::Duration::hours(1)
}

fn make_create(user_uuid: uuid::Uuid, change_type: ChangeType) -> CreateProfileChange {
    CreateProfileChange {
        user_uuid,
        change_type,
        identifier: Some("+66899999999".to_string()),
        old_value: Some("old@example.com".to_string()),
        new_value: Some("new@example.com".to_string()),
        status: ChangeStatus::PendingVerifyOtp,
        otp: Some("123456".to_string()),
        ref_code: Some("ABCDEF".to_string()),
        token_expired_at: future(),
        next_otp_request_at: future(),
        otp_expired_at: future(),
    }
}

// ── create ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_inserts_profile_change_row() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    let pc = repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();
    assert_eq!(pc.user_uuid, user_uuid);
    assert_eq!(pc.change_type, ChangeType::Email);
    assert_eq!(pc.status, ChangeStatus::PendingVerifyOtp);
    assert_eq!(pc.otp.as_deref(), Some("123456"));

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── find_by_id ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn find_by_id_returns_some() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, ChangeType::Telephone)).await.unwrap();
    let found = repo.find_by_id(created.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, created.id);

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_by_id_returns_none_for_missing() {
    let pool = helpers::pool().await;
    let repo = PgProfileChangeRepository::new(pool);
    let result = repo.find_by_id(uuid::Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

// ── find_active_by_user_and_type ──────────────────────────────────────────────

#[tokio::test]
async fn find_active_returns_pending_row() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();

    let found = repo
        .find_active_by_user_and_type(user_uuid, ChangeType::Email)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().status, ChangeStatus::PendingVerifyOtp);

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_active_excludes_completed_records() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();
    repo.update_status_and_token(created.id, ChangeStatus::Completed, None, None)
        .await
        .unwrap();

    let found = repo
        .find_active_by_user_and_type(user_uuid, ChangeType::Email)
        .await
        .unwrap();
    assert!(found.is_none());

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_active_returns_none_when_type_differs() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();

    let found = repo
        .find_active_by_user_and_type(user_uuid, ChangeType::Telephone)
        .await
        .unwrap();
    assert!(found.is_none());

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── update_otp ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_otp_persists_new_code() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();
    let updated = repo
        .update_otp(created.id, "999999".to_string(), "NEWREF".to_string(), future(), future())
        .await
        .unwrap();

    assert_eq!(updated.otp.as_deref(), Some("999999"));
    assert_eq!(updated.ref_code.as_deref(), Some("NEWREF"));

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── update_status_and_token ───────────────────────────────────────────────────

#[tokio::test]
async fn update_status_and_token_sets_completed() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();
    let updated = repo
        .update_status_and_token(created.id, ChangeStatus::Completed, None, None)
        .await
        .unwrap();

    assert_eq!(updated.status, ChangeStatus::Completed);
    assert!(updated.token.is_none());

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn update_status_and_token_persists_token() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool.clone());

    let created = repo.create(make_create(user_uuid, ChangeType::Email)).await.unwrap();
    let updated = repo
        .update_status_and_token(
            created.id,
            ChangeStatus::VerifyChangeCompleted,
            Some("jwt-token-here".to_string()),
            Some(future()),
        )
        .await
        .unwrap();

    assert_eq!(updated.status, ChangeStatus::VerifyChangeCompleted);
    assert_eq!(updated.token.as_deref(), Some("jwt-token-here"));

    helpers::cleanup_user(&pool, user_uuid).await;
}
