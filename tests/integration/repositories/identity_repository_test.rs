use sqlx::PgPool;
use uuid::Uuid;

use domain::{
    entities::{
        customer::CreateCustomer,
        identity::CreateIdentity,
    },
    repositories::{
        customer_repository::CustomerRepository,
        identity_repository::IdentityRepository,
    },
};
use infrastructure::persistence::{
    pg_customer_repository::PgCustomerRepository,
    pg_identity_repository::PgIdentityRepository,
};

async fn seed_user(pool: &PgPool) -> Uuid {
    let repo = PgCustomerRepository::new(pool.clone());
    repo.create(CreateCustomer {
        email: Some(format!("user-{}@test.com", Uuid::new_v4())),
        phone: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
        locale: None,
        has_consent: None,
        client_id: None,
    }).await.unwrap().id
}

fn make_identity(user_uuid: Uuid, provider: &str, external_id: &str) -> CreateIdentity {
    CreateIdentity {
        user_uuid,
        provider_name: provider.to_string(),
        external_id: external_id.to_string(),
        provider_id_token: Some("id-tok".to_string()),
        provider_access_token: Some("acc-tok".to_string()),
        provider_refresh_token: Some("ref-tok".to_string()),
    }
}

// ---------------------------------------------------------------------------
// create / find_by_user
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn create_and_find_by_user(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    repo.create(make_identity(user, "google", "ext-1")).await.unwrap();

    let identities = repo.find_by_user(user).await.unwrap();
    assert_eq!(identities.len(), 1);
    assert_eq!(identities[0].provider_name, "google");
    assert_eq!(identities[0].external_id, "ext-1");
    assert!(!identities[0].is_deleted);
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_user_excludes_deleted(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    let identity = repo.create(make_identity(user, "google", "ext-del")).await.unwrap();
    repo.soft_delete(identity.id, user).await.unwrap();

    let active = repo.find_by_user(user).await.unwrap();
    assert!(active.is_empty());
}

// ---------------------------------------------------------------------------
// find_active / find_deleted
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn find_active_returns_existing(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    repo.create(make_identity(user, "apple", "apple-ext")).await.unwrap();

    let found = repo.find_active(user, "apple", "apple-ext").await.unwrap();
    assert!(found.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn find_active_returns_none_for_deleted(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    let identity = repo.create(make_identity(user, "apple", "apple-ext")).await.unwrap();
    repo.soft_delete(identity.id, user).await.unwrap();

    let found = repo.find_active(user, "apple", "apple-ext").await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn find_deleted_returns_soft_deleted(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    let identity = repo.create(make_identity(user, "the1", "the1-ext")).await.unwrap();
    repo.soft_delete(identity.id, user).await.unwrap();

    let found = repo.find_deleted("the1", "the1-ext").await.unwrap();
    assert!(found.is_some());
    assert!(found.unwrap().is_deleted);
}

// ---------------------------------------------------------------------------
// soft_delete / restore
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn soft_delete_marks_deleted(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    let identity = repo.create(make_identity(user, "fb", "fb-ext")).await.unwrap();
    let deleted = repo.soft_delete(identity.id, user).await.unwrap();

    assert!(deleted.is_deleted);
}

#[sqlx::test(migrations = "./migrations")]
async fn restore_reactivates_deleted_identity(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    let identity = repo.create(make_identity(user, "fb", "fb-ext")).await.unwrap();
    repo.soft_delete(identity.id, user).await.unwrap();

    let new_tokens = make_identity(user, "fb", "fb-ext");
    let restored = repo.restore(identity.id, user, new_tokens).await.unwrap();

    assert!(!restored.is_deleted);
    assert_eq!(restored.id, identity.id);
}

// ---------------------------------------------------------------------------
// update_tokens
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn update_tokens_replaces_stored_tokens(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool);

    let identity = repo.create(make_identity(user, "the1", "the1-update")).await.unwrap();
    let updated = repo.update_tokens(
        identity.id,
        Some("new-access".to_string()),
        Some("new-refresh".to_string()),
    ).await.unwrap();

    assert_eq!(updated.provider_access_token.as_deref(), Some("new-access"));
    assert_eq!(updated.provider_refresh_token.as_deref(), Some("new-refresh"));
}

// ---------------------------------------------------------------------------
// log_transaction
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn log_transaction_inserts_audit_row(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgIdentityRepository::new(pool.clone());

    repo.log_transaction(user, "delete", "google", "ext-audit").await.unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity_provider_transactions WHERE user_uuid = $1"
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1);
}
