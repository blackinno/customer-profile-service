use sqlx::PgPool;
use uuid::Uuid;

use domain::{
    entities::customer::{CreateCustomer, Gender, Locale, SearchField, UpdateCustomer},
    repositories::customer_repository::CustomerRepository,
};
use infrastructure::persistence::pg_customer_repository::PgCustomerRepository;

fn make_customer(email: Option<&str>, phone: Option<&str>) -> CreateCustomer {
    CreateCustomer {
        email: email.map(str::to_string),
        phone: phone.map(str::to_string),
        first_name: Some("Jane".to_string()),
        last_name: Some("Doe".to_string()),
        birthdate: None,
        gender: Some(Gender::Female),
        nationality: Some("TH".to_string()),
        locale: Some(Locale::Th),
        has_consent: Some(true),
        client_id: None,
    }
}

// ---------------------------------------------------------------------------
// create / find_by_id
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn create_and_find_by_id(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let customer = repo
        .create(make_customer(Some("jane@test.com"), None))
        .await
        .unwrap();

    assert!(customer.profile.is_some());
    assert_eq!(customer.email.as_deref(), Some("jane@test.com"));
    assert_eq!(
        customer.profile.as_ref().unwrap().first_name.as_deref(),
        Some("Jane")
    );

    let found = repo.find_by_id(customer.id).await.unwrap().unwrap();
    assert_eq!(found.id, customer.id);
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_id_returns_none_for_unknown(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let result = repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// find_by_email / find_by_phone
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn find_by_email(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    repo.create(make_customer(Some("find@test.com"), None))
        .await
        .unwrap();

    let found = repo.find_by_email("find@test.com").await.unwrap().unwrap();
    assert_eq!(found.email.as_deref(), Some("find@test.com"));
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_email_returns_none_for_unknown(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let result = repo.find_by_email("ghost@test.com").await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_phone(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    repo.create(make_customer(None, Some("+66811111111")))
        .await
        .unwrap();

    let found = repo.find_by_phone("+66811111111").await.unwrap().unwrap();
    assert_eq!(found.phone.as_deref(), Some("+66811111111"));
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn search_by_id(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let customer = repo
        .create(make_customer(Some("search@test.com"), None))
        .await
        .unwrap();

    let results = repo.search(SearchField::Id(customer.id)).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, customer.id);
}

#[sqlx::test(migrations = "./migrations")]
async fn search_by_phone(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    repo.create(make_customer(None, Some("+66822222222")))
        .await
        .unwrap();

    let results = repo
        .search(SearchField::Phone("+66822222222".to_string()))
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].phone.as_deref(), Some("+66822222222"));
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn update_customer_fields(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let customer = repo
        .create(make_customer(Some("upd@test.com"), None))
        .await
        .unwrap();

    let updated = repo
        .update(
            customer.id,
            UpdateCustomer {
                email: None,
                phone: Some("+66899999999".to_string()),
                first_name: Some("Updated".to_string()),
                last_name: None,
                birthdate: None,
                gender: Some(Gender::Male),
                nationality: None,
                locale: Some(Locale::En),
                has_consent: Some(false),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.phone.as_deref(), Some("+66899999999"));
    assert_eq!(
        updated.profile.as_ref().unwrap().first_name.as_deref(),
        Some("Updated")
    );
    assert_eq!(updated.profile.as_ref().unwrap().gender, Some(Gender::Male));
}

// ---------------------------------------------------------------------------
// soft_delete
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn soft_delete_marks_deleted_and_frees_email(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let customer = repo
        .create(make_customer(Some("del@test.com"), None))
        .await
        .unwrap();

    let deleted = repo.soft_delete(customer.id).await.unwrap();
    assert!(deleted.is_deleted);
    // email is mangled so the same address can be re-registered
    assert!(
        deleted
            .email
            .as_deref()
            .map(|e| e.contains("-deleted-"))
            .unwrap_or(false)
    );

    // same email can now be reused
    let new = repo
        .create(make_customer(Some("del@test.com"), None))
        .await
        .unwrap();
    assert!(!new.is_deleted);
}

// ---------------------------------------------------------------------------
// update_profile_image
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn update_profile_image_sets_key(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let customer = repo
        .create(make_customer(Some("img@test.com"), None))
        .await
        .unwrap();

    repo.update_profile_image(customer.id, Some("profiles/img.jpg".to_string()))
        .await
        .unwrap();

    let found = repo.find_by_id(customer.id).await.unwrap().unwrap();
    assert_eq!(
        found.profile.unwrap().profile_image.as_deref(),
        Some("profiles/img.jpg")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn update_profile_image_clears_key(pool: PgPool) {
    let repo = PgCustomerRepository::new(pool);
    let customer = repo
        .create(make_customer(Some("imgclear@test.com"), None))
        .await
        .unwrap();

    repo.update_profile_image(customer.id, Some("profiles/img.jpg".to_string()))
        .await
        .unwrap();
    repo.update_profile_image(customer.id, None).await.unwrap();

    let found = repo.find_by_id(customer.id).await.unwrap().unwrap();
    assert!(found.profile.unwrap().profile_image.is_none());
}
