#[path = "helpers.rs"]
mod helpers;

use domain::entities::customer::{CreateCustomer, Locale, SearchField, UpdateCustomer};
use domain::repositories::customer_repository::CustomerRepository;
use infrastructure::persistence::pg_customer_repository::PgCustomerRepository;

// ── create ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_inserts_user_and_profile() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());

    let customer = repo
        .create(CreateCustomer {
            email: Some(format!("alice-{}@example.com", uuid::Uuid::new_v4())),
            phone: Some(format!("+668{}", &uuid::Uuid::new_v4().to_string()[..8])),
            locale: Some(Locale::En),
            has_consent: Some(true),
            client_id: None,
            first_name: Some("Alice".to_string()),
            last_name: Some("Smith".to_string()),
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    assert_eq!(customer.locale, Locale::En);
    assert!(customer.has_consent);
    let profile = customer.profile.unwrap();
    assert_eq!(profile.first_name.as_deref(), Some("Alice"));
    assert_eq!(profile.last_name.as_deref(), Some("Smith"));

    helpers::cleanup_user(&pool, customer.id).await;
}

// ── find_by_id ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn find_by_id_returns_some() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());

    let created = repo
        .create(CreateCustomer {
            email: Some(format!("bob-{}@example.com", uuid::Uuid::new_v4())),
            phone: None,
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let found = repo.find_by_id(created.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, created.id);

    helpers::cleanup_user(&pool, created.id).await;
}

#[tokio::test]
async fn find_by_id_returns_none_for_missing() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool);
    let result = repo.find_by_id(uuid::Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

// ── find_by_email / find_by_phone ─────────────────────────────────────────────

#[tokio::test]
async fn find_by_email_returns_customer() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());
    let email = format!("carol-{}@example.com", uuid::Uuid::new_v4());

    let created = repo
        .create(CreateCustomer {
            email: Some(email.clone()),
            phone: None,
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let found = repo.find_by_email(&email).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().email.as_deref(), Some(email.as_str()));

    helpers::cleanup_user(&pool, created.id).await;
}

#[tokio::test]
async fn find_by_phone_returns_customer() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());
    let phone = format!("+6699{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..7]);

    let created = repo
        .create(CreateCustomer {
            email: None,
            phone: Some(phone.clone()),
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let found = repo.find_by_phone(&phone).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().phone.as_deref(), Some(phone.as_str()));

    helpers::cleanup_user(&pool, created.id).await;
}

// ── search ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_by_id_returns_results() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());

    let created = repo
        .create(CreateCustomer {
            email: Some(format!("dave-{}@example.com", uuid::Uuid::new_v4())),
            phone: None,
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let results = repo.search(SearchField::Id(created.id)).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, created.id);

    helpers::cleanup_user(&pool, created.id).await;
}

#[tokio::test]
async fn search_by_phone_returns_results() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());
    let phone = format!("+6688{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..7]);

    let created = repo
        .create(CreateCustomer {
            email: None,
            phone: Some(phone.clone()),
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let results = repo.search(SearchField::Phone(phone)).await.unwrap();
    assert_eq!(results.len(), 1);

    helpers::cleanup_user(&pool, created.id).await;
}

// ── update ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_sets_fields_on_both_tables() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());

    let created = repo
        .create(CreateCustomer {
            email: Some(format!("eve-{}@example.com", uuid::Uuid::new_v4())),
            phone: None,
            locale: Some(Locale::Th),
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let updated = repo
        .update(
            created.id,
            UpdateCustomer {
                email: None,
                phone: None,
                locale: Some(Locale::En),
                has_consent: Some(true),
                first_name: Some("Eve".to_string()),
                last_name: Some("Updated".to_string()),
                birthdate: None,
                gender: None,
                nationality: Some("TH".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.locale, Locale::En);
    assert!(updated.has_consent);
    let profile = updated.profile.unwrap();
    assert_eq!(profile.first_name.as_deref(), Some("Eve"));
    assert_eq!(profile.nationality.as_deref(), Some("TH"));

    helpers::cleanup_user(&pool, created.id).await;
}

// ── soft_delete ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn soft_delete_marks_deleted_and_mangles_unique_columns() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());
    let email = format!("frank-{}@example.com", uuid::Uuid::new_v4());
    let phone = format!("+6677{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..7]);

    let created = repo
        .create(CreateCustomer {
            email: Some(email.clone()),
            phone: Some(phone.clone()),
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    let deleted = repo.soft_delete(created.id).await.unwrap();
    assert!(deleted.is_deleted);
    assert_ne!(deleted.email.as_deref(), Some(email.as_str()));
    assert_ne!(deleted.phone.as_deref(), Some(phone.as_str()));

    helpers::cleanup_user(&pool, created.id).await;
}

// ── update_profile_image ──────────────────────────────────────────────────────

#[tokio::test]
async fn update_profile_image_persists_key() {
    let pool = helpers::pool().await;
    let repo = PgCustomerRepository::new(pool.clone());

    let created = repo
        .create(CreateCustomer {
            email: Some(format!("grace-{}@example.com", uuid::Uuid::new_v4())),
            phone: None,
            locale: None,
            has_consent: None,
            client_id: None,
            first_name: None,
            last_name: None,
            birthdate: None,
            gender: None,
            nationality: None,
        })
        .await
        .unwrap();

    repo.update_profile_image(created.id, Some("profiles/grace.jpg".to_string()))
        .await
        .unwrap();

    let found = repo.find_by_id(created.id).await.unwrap().unwrap();
    assert_eq!(
        found.profile.unwrap().profile_image.as_deref(),
        Some("profiles/grace.jpg")
    );

    helpers::cleanup_user(&pool, created.id).await;
}
