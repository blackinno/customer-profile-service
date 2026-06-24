#[path = "helpers.rs"]
mod helpers;

use domain::entities::the1_user::{UpsertThe1User, UpsertTier};
use domain::repositories::the1_user_repository::The1UserRepository;
use infrastructure::persistence::pg_the1_user_repository::PgThe1UserRepository;
use uuid::Uuid;

fn make_upsert(member_id: &str, card: Option<&str>, tiers: Vec<UpsertTier>) -> UpsertThe1User {
    UpsertThe1User {
        member_id: member_id.to_string(),
        account_id: format!("ACC-{member_id}"),
        profile_id: format!("PRF-{member_id}"),
        card_number: card.map(str::to_string),
        tiers,
    }
}

fn gold_tier() -> UpsertTier {
    UpsertTier {
        code: "GOLD".to_string(),
        name: Some("Gold Member".to_string()),
        expired_date: None,
    }
}

fn silver_tier() -> UpsertTier {
    UpsertTier {
        code: "SILVER".to_string(),
        name: Some("Silver Member".to_string()),
        expired_date: None,
    }
}

// ── upsert (insert path) ──────────────────────────────────────────────────────

#[tokio::test]
async fn upsert_creates_new_record_with_tiers() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let member_id = format!("MEM-{}", Uuid::new_v4());
    let card = Uuid::new_v4().simple().to_string();
    let repo = PgThe1UserRepository::new(pool.clone());

    let result = repo
        .upsert(user_uuid, make_upsert(&member_id, Some(&card), vec![gold_tier()]))
        .await
        .unwrap();

    assert_eq!(result.user_uuid, user_uuid);
    assert_eq!(result.member_id, member_id);
    assert_eq!(result.card_number.as_deref(), Some(card.as_str()));
    assert_eq!(result.tiers.len(), 1);
    assert_eq!(result.tiers[0].code, "GOLD");

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── upsert (update path) ──────────────────────────────────────────────────────

#[tokio::test]
async fn upsert_updates_existing_record_and_replaces_tiers() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let member_id = format!("MEM-{}", Uuid::new_v4());
    let card1 = Uuid::new_v4().simple().to_string();
    let card2 = Uuid::new_v4().simple().to_string();
    let repo = PgThe1UserRepository::new(pool.clone());

    repo.upsert(user_uuid, make_upsert(&member_id, Some(&card1), vec![gold_tier()]))
        .await
        .unwrap();

    let member_id2 = format!("MEM2-{}", Uuid::new_v4());
    let result = repo
        .upsert(user_uuid, make_upsert(&member_id2, Some(&card2), vec![silver_tier()]))
        .await
        .unwrap();

    assert_eq!(result.member_id, member_id2);
    assert_eq!(result.card_number.as_deref(), Some(card2.as_str()));
    assert_eq!(result.tiers.len(), 1);
    assert_eq!(result.tiers[0].code, "SILVER");

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn upsert_replaces_tiers_with_empty_set() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let member_id = format!("MEM-{}", Uuid::new_v4());
    let card = Uuid::new_v4().simple().to_string();
    let repo = PgThe1UserRepository::new(pool.clone());

    repo.upsert(user_uuid, make_upsert(&member_id, Some(&card), vec![gold_tier()]))
        .await
        .unwrap();

    let result = repo
        .upsert(user_uuid, make_upsert(&member_id, Some(&card), vec![]))
        .await
        .unwrap();

    assert!(result.tiers.is_empty());

    helpers::cleanup_user(&pool, user_uuid).await;
}

// ── find_by_user ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn find_by_user_returns_record_with_tiers() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let member_id = format!("MEM-{}", Uuid::new_v4());
    let card = Uuid::new_v4().simple().to_string();
    let repo = PgThe1UserRepository::new(pool.clone());

    repo.upsert(user_uuid, make_upsert(&member_id, Some(&card), vec![gold_tier()]))
        .await
        .unwrap();

    let found = repo.find_by_user(user_uuid).await.unwrap();
    assert!(found.is_some());
    let the1 = found.unwrap();
    assert_eq!(the1.card_number.as_deref(), Some(card.as_str()));
    assert_eq!(the1.tiers.len(), 1);
    assert_eq!(the1.tiers[0].code, "GOLD");

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_by_user_returns_none_for_missing() {
    let pool = helpers::pool().await;
    let repo = PgThe1UserRepository::new(pool);
    let result = repo.find_by_user(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

// ── find_by_card_number ───────────────────────────────────────────────────────

#[tokio::test]
async fn find_by_card_number_returns_correct_record() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let member_id = format!("MEM-{}", Uuid::new_v4());
    let card = Uuid::new_v4().simple().to_string();
    let repo = PgThe1UserRepository::new(pool.clone());

    repo.upsert(user_uuid, make_upsert(&member_id, Some(&card), vec![]))
        .await
        .unwrap();

    let found = repo.find_by_card_number(&card).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().user_uuid, user_uuid);

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_by_card_number_returns_none_for_unknown() {
    let pool = helpers::pool().await;
    let repo = PgThe1UserRepository::new(pool);
    let result = repo.find_by_card_number("NO-SUCH-CARD").await.unwrap();
    assert!(result.is_none());
}

// ── find_by_member_id ─────────────────────────────────────────────────────────

#[tokio::test]
async fn find_by_member_id_returns_correct_record() {
    let pool = helpers::pool().await;
    let user_uuid = helpers::seed_user(&pool).await;
    let member_id = format!("MXYZ-{}", Uuid::new_v4());
    let repo = PgThe1UserRepository::new(pool.clone());

    repo.upsert(user_uuid, make_upsert(&member_id, None, vec![]))
        .await
        .unwrap();

    let found = repo.find_by_member_id(&member_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().member_id, member_id);

    helpers::cleanup_user(&pool, user_uuid).await;
}

#[tokio::test]
async fn find_by_member_id_returns_none_for_unknown() {
    let pool = helpers::pool().await;
    let repo = PgThe1UserRepository::new(pool);
    let result = repo.find_by_member_id("NO-SUCH-MEMBER").await.unwrap();
    assert!(result.is_none());
}
