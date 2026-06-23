use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use domain::{
    entities::{
        customer::CreateCustomer,
        profile_change::{ChangeStatus, ChangeType, CreateProfileChange},
    },
    repositories::{
        customer_repository::CustomerRepository, profile_change_repository::ProfileChangeRepository,
    },
};
use infrastructure::persistence::{
    pg_customer_repository::PgCustomerRepository,
    pg_profile_change_repository::PgProfileChangeRepository,
};

async fn seed_user(pool: &PgPool) -> Uuid {
    PgCustomerRepository::new(pool.clone())
        .create(CreateCustomer {
            email: Some(format!("pc-{}@test.com", Uuid::new_v4())),
            phone: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
            locale: None,
            has_consent: None,
            client_id: None,
        })
        .await
        .unwrap()
        .id
}

fn pending_change(user_uuid: Uuid, change_type: ChangeType) -> CreateProfileChange {
    let now = Utc::now();
    CreateProfileChange {
        user_uuid,
        change_type,
        identifier: Some("test-identifier".to_string()),
        old_value: Some("old".to_string()),
        new_value: Some("new".to_string()),
        status: ChangeStatus::PendingVerifyOtp,
        otp: Some("123456".to_string()),
        ref_code: Some("ABCDEF".to_string()),
        next_otp_request_at: now + chrono::Duration::seconds(60),
        otp_expired_at: now + chrono::Duration::seconds(300),
        token_expired_at: now + chrono::Duration::seconds(3600),
    }
}

// ---------------------------------------------------------------------------
// create / find_by_id
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn create_and_find_by_id(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool);

    let pc = repo
        .create(pending_change(user, ChangeType::Telephone))
        .await
        .unwrap();

    assert_eq!(pc.user_uuid, user);
    assert_eq!(pc.change_type, ChangeType::Telephone);
    assert_eq!(pc.status, ChangeStatus::PendingVerifyOtp);

    let found = repo.find_by_id(pc.id).await.unwrap().unwrap();
    assert_eq!(found.id, pc.id);
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_id_returns_none_for_unknown(pool: PgPool) {
    let repo = PgProfileChangeRepository::new(pool);
    let result = repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// find_active_by_user_and_type
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn find_active_by_user_and_type_returns_pending(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool);

    repo.create(pending_change(user, ChangeType::Email))
        .await
        .unwrap();

    let found = repo
        .find_active_by_user_and_type(user, ChangeType::Email)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().status, ChangeStatus::PendingVerifyOtp);
}

#[sqlx::test(migrations = "./migrations")]
async fn find_active_by_user_and_type_returns_none_for_completed(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool);

    let pc = repo
        .create(pending_change(user, ChangeType::Email))
        .await
        .unwrap();
    repo.update_status_and_token(pc.id, ChangeStatus::Completed, None, None)
        .await
        .unwrap();

    let found = repo
        .find_active_by_user_and_type(user, ChangeType::Email)
        .await
        .unwrap();
    assert!(found.is_none());
}

// ---------------------------------------------------------------------------
// update_otp
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn update_otp_replaces_otp_and_ref(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool);

    let pc = repo
        .create(pending_change(user, ChangeType::Telephone))
        .await
        .unwrap();
    let now = Utc::now();

    let updated = repo
        .update_otp(
            pc.id,
            "999999".to_string(),
            "NEWREF".to_string(),
            now + chrono::Duration::seconds(60),
            now + chrono::Duration::seconds(300),
        )
        .await
        .unwrap();

    assert_eq!(updated.otp.as_deref(), Some("999999"));
    assert_eq!(updated.ref_code.as_deref(), Some("NEWREF"));
}

// ---------------------------------------------------------------------------
// update_status_and_token
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn update_status_sets_verified(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool);

    let pc = repo
        .create(pending_change(user, ChangeType::Email))
        .await
        .unwrap();
    let expiry = Utc::now() + chrono::Duration::seconds(3600);

    let updated = repo
        .update_status_and_token(
            pc.id,
            ChangeStatus::VerifyChangeCompleted,
            Some("jwt-token".to_string()),
            Some(expiry),
        )
        .await
        .unwrap();

    assert_eq!(updated.status, ChangeStatus::VerifyChangeCompleted);
    assert_eq!(updated.token.as_deref(), Some("jwt-token"));
}

#[sqlx::test(migrations = "./migrations")]
async fn update_status_to_completed(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgProfileChangeRepository::new(pool);

    let pc = repo
        .create(pending_change(user, ChangeType::Email))
        .await
        .unwrap();
    let updated = repo
        .update_status_and_token(pc.id, ChangeStatus::Completed, None, None)
        .await
        .unwrap();

    assert_eq!(updated.status, ChangeStatus::Completed);
}
