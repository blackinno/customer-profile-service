use sqlx::PgPool;
use uuid::Uuid;

use domain::{
    entities::{
        customer::CreateCustomer,
        the1_user::{UpsertThe1User, UpsertTier},
    },
    repositories::{
        customer_repository::CustomerRepository, the1_user_repository::The1UserRepository,
    },
};
use infrastructure::persistence::{
    pg_customer_repository::PgCustomerRepository, pg_the1_user_repository::PgThe1UserRepository,
};

async fn seed_user(pool: &PgPool) -> Uuid {
    PgCustomerRepository::new(pool.clone())
        .create(CreateCustomer {
            email: Some(format!("the1-{}@test.com", Uuid::new_v4())),
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

fn upsert_data(member_id: &str, card_number: Option<&str>) -> UpsertThe1User {
    UpsertThe1User {
        member_id: member_id.to_string(),
        account_id: "ACC001".to_string(),
        profile_id: "PRO001".to_string(),
        card_number: card_number.map(str::to_string),
        tiers: vec![UpsertTier {
            code: "GOLD".to_string(),
            name: Some("Gold".to_string()),
            expired_date: None,
        }],
    }
}

// ---------------------------------------------------------------------------
// upsert — creates a new record when none exists
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn upsert_creates_new_the1_user(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgThe1UserRepository::new(pool);

    let the1 = repo
        .upsert(user, upsert_data("MEM001", Some("1234567890123456")))
        .await
        .unwrap();

    assert_eq!(the1.user_uuid, user);
    assert_eq!(the1.member_id, "MEM001");
    assert_eq!(the1.card_number.as_deref(), Some("1234567890123456"));
    assert_eq!(the1.tiers.len(), 1);
    assert_eq!(the1.tiers[0].code, "GOLD");
}

// ---------------------------------------------------------------------------
// upsert — updates an existing record
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn upsert_updates_existing_the1_user(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgThe1UserRepository::new(pool);

    repo.upsert(user, upsert_data("MEM001", Some("1111111111111111")))
        .await
        .unwrap();
    let updated = repo
        .upsert(user, upsert_data("MEM001", Some("2222222222222222")))
        .await
        .unwrap();

    assert_eq!(updated.card_number.as_deref(), Some("2222222222222222"));
    // tiers are replaced on each upsert
    assert_eq!(updated.tiers.len(), 1);
}

// ---------------------------------------------------------------------------
// find_by_user
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn find_by_user_returns_existing(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgThe1UserRepository::new(pool);

    repo.upsert(user, upsert_data("MEM002", None))
        .await
        .unwrap();

    let found = repo.find_by_user(user).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().member_id, "MEM002");
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_user_returns_none_for_unknown(pool: PgPool) {
    let repo = PgThe1UserRepository::new(pool);
    let found = repo.find_by_user(Uuid::new_v4()).await.unwrap();
    assert!(found.is_none());
}

// ---------------------------------------------------------------------------
// find_by_card_number / find_by_member_id
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn find_by_card_number_returns_existing(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgThe1UserRepository::new(pool);

    repo.upsert(user, upsert_data("MEM003", Some("9999999999999999")))
        .await
        .unwrap();

    let found = repo.find_by_card_number("9999999999999999").await.unwrap();
    assert!(found.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_member_id_returns_existing(pool: PgPool) {
    let user = seed_user(&pool).await;
    let repo = PgThe1UserRepository::new(pool);

    repo.upsert(user, upsert_data("MEMBER-FIND", None))
        .await
        .unwrap();

    let found = repo.find_by_member_id("MEMBER-FIND").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().member_id, "MEMBER-FIND");
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_card_number_returns_none_for_unknown(pool: PgPool) {
    let repo = PgThe1UserRepository::new(pool);
    let found = repo.find_by_card_number("0000000000000000").await.unwrap();
    assert!(found.is_none());
}
