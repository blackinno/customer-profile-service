use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use application::{
    config::AppConfig,
    customers::{
        dtos::{CreateCustomerRequest, SearchCustomerQuery, UpdateCustomerRequest},
        use_cases::CustomerUseCases,
    },
    errors::ApplicationError,
};
use domain::{
    entities::customer::{
        CreateCustomer, Customer, CustomerProfile, Gender, Locale, SearchField, UpdateCustomer,
    },
    errors::RepositoryError,
    repositories::customer_repository::CustomerRepository,
};

// ---- mock repository ----

struct MockCustomerRepository {
    store: Arc<Mutex<HashMap<Uuid, Customer>>>,
    /// When `true` every async call returns `RepositoryError::Backend`.
    should_fail: bool,
}

impl MockCustomerRepository {
    fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            should_fail: false,
        }
    }

    fn failing() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            should_fail: true,
        }
    }

    fn with_customer(c: Customer) -> Self {
        let mut map = HashMap::new();
        map.insert(c.id, c);
        Self {
            store: Arc::new(Mutex::new(map)),
            should_fail: false,
        }
    }
}

#[async_trait]
impl CustomerRepository for MockCustomerRepository {
    async fn create(&self, data: CreateCustomer) -> Result<Customer, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let id = Uuid::new_v4();
        let now = Utc::now();
        let customer = Customer {
            id,
            email: data.email,
            phone: data.phone,
            email_verified: false,
            phone_verified: false,
            locale: data.locale.unwrap_or(Locale::Th),
            has_consent: data.has_consent.unwrap_or(false),
            is_deleted: false,
            client_id: data.client_id,
            created_at: now,
            updated_at: now,
            profile: Some(CustomerProfile {
                id: Uuid::new_v4(),
                user_uuid: id,
                first_name: data.first_name,
                last_name: data.last_name,
                birthdate: data.birthdate,
                gender: data.gender,
                profile_image: None,
                nationality: data.nationality,
                created_at: now,
                updated_at: now,
            }),
        };
        self.store.lock().unwrap().insert(id, customer.clone());
        Ok(customer)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_phone(&self, phone: &str) -> Result<Option<Customer>, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|c| c.phone.as_deref() == Some(phone))
            .cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Customer>, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|c| c.email.as_deref() == Some(email))
            .cloned())
    }

    async fn search(&self, field: SearchField) -> Result<Vec<Customer>, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let store = self.store.lock().unwrap();
        let results: Vec<Customer> = match field {
            SearchField::Id(id) => store.values().filter(|c| c.id == id).cloned().collect(),
            SearchField::Phone(phone) => store
                .values()
                .filter(|c| c.phone.as_deref() == Some(phone.as_str()))
                .cloned()
                .collect(),
            // The1 lookups not exercised in unit tests — return empty
            SearchField::The1MemberId(_) | SearchField::The1CardNumber(_) => vec![],
        };
        Ok(results)
    }

    async fn update(&self, id: Uuid, data: UpdateCustomer) -> Result<Customer, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let mut store = self.store.lock().unwrap();
        let customer = store
            .get_mut(&id)
            .ok_or_else(|| RepositoryError::NotFound(format!("customer {} not found", id)))?;

        if let Some(email) = data.email {
            customer.email = Some(email);
        }
        if let Some(phone) = data.phone {
            customer.phone = Some(phone);
        }
        if let Some(locale) = data.locale {
            customer.locale = locale;
        }
        if let Some(has_consent) = data.has_consent {
            customer.has_consent = has_consent;
        }
        if let Some(profile) = customer.profile.as_mut() {
            if let Some(v) = data.first_name {
                profile.first_name = Some(v);
            }
            if let Some(v) = data.last_name {
                profile.last_name = Some(v);
            }
            if let Some(v) = data.birthdate {
                profile.birthdate = Some(v);
            }
            if let Some(v) = data.gender {
                profile.gender = Some(v);
            }
            if let Some(v) = data.nationality {
                profile.nationality = Some(v);
            }
        }
        customer.updated_at = Utc::now();
        Ok(customer.clone())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<Customer, RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let mut store = self.store.lock().unwrap();
        let customer = store
            .get_mut(&id)
            .ok_or_else(|| RepositoryError::NotFound(format!("customer {} not found", id)))?;

        let new_phone = customer.phone.as_ref().map(|p| format!("{}-deleted-{}", p, id));
        let new_email = customer.email.as_ref().map(|e| format!("{}-deleted-{}", e, id));
        customer.is_deleted = true;
        customer.phone = new_phone;
        customer.email = new_email;
        customer.updated_at = Utc::now();
        Ok(customer.clone())
    }

    async fn update_profile_image(
        &self,
        user_uuid: Uuid,
        image_key: Option<String>,
    ) -> Result<(), RepositoryError> {
        if self.should_fail {
            return Err(RepositoryError::Backend("mock error".to_string()));
        }
        let mut store = self.store.lock().unwrap();
        if let Some(customer) = store.get_mut(&user_uuid) {
            if let Some(profile) = customer.profile.as_mut() {
                profile.profile_image = image_key;
            }
        }
        Ok(())
    }
}

// ---- helpers ----

fn test_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        country_code: "66".to_string(),
        phone_number_format: "+66".to_string(),
        otp_expired_time: 300,
        otp_text: "Your OTP: {otp}".to_string(),
        jwt_secret_key: "test_secret".to_string(),
        profile_change_expired_time: 3600,
        token_expired_time: 86400,
        allow_image_types: vec!["image/jpeg".to_string()],
        max_image_size_mb: 5,
        image_prefix: "profiles/".to_string(),
        image_expired_in_sec: 3600,
        sns_user_profile_changed: "topic-1".to_string(),
        sns_email_sent_requested: "topic-2".to_string(),
        sns_user_identity_linked_changed: "topic-3".to_string(),
        sns_user_the1_get_profile_updated: "topic-4".to_string(),
        s3_profile_bucket: "test-bucket".to_string(),
        cloudfront_base_endpoint: "https://cdn.example.com".to_string(),
        cloudfront_key_id: "test-key".to_string(),
    })
}

fn make_customer(email: Option<&str>, phone: Option<&str>, deleted: bool) -> Customer {
    let now = Utc::now();
    let id = Uuid::new_v4();
    Customer {
        id,
        email: email.map(str::to_string),
        phone: phone.map(str::to_string),
        email_verified: false,
        phone_verified: false,
        locale: Locale::Th,
        has_consent: false,
        is_deleted: deleted,
        client_id: None,
        created_at: now,
        updated_at: now,
        profile: Some(CustomerProfile {
            id: Uuid::new_v4(),
            user_uuid: id,
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            birthdate: None,
            gender: None,
            profile_image: None,
            nationality: None,
            created_at: now,
            updated_at: now,
        }),
    }
}

// ---- create tests ----

#[tokio::test]
async fn create_happy_path() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let req = CreateCustomerRequest {
        email: Some("alice@example.com".to_string()),
        phone: Some("0812345678".to_string()),
        locale: Some("en".to_string()),
        has_consent: Some(true),
        client_id: None,
        first_name: Some("Alice".to_string()),
        last_name: Some("Smith".to_string()),
        birthdate: Some("1990-01-15".to_string()),
        gender: Some("female".to_string()),
        nationality: Some("TH".to_string()),
    };

    let resp = uc.create(req).await.expect("should succeed");
    assert_eq!(resp.email, Some("alice@example.com".to_string()));
    // phone should be normalised: "0812345678" → "+66812345678"
    assert_eq!(resp.phone, Some("+66812345678".to_string()));
    assert_eq!(resp.locale, "en");
    assert_eq!(resp.first_name, Some("Alice".to_string()));
}

#[tokio::test]
async fn create_no_phone_no_email() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let req = CreateCustomerRequest {
        email: None,
        phone: None,
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    let resp = uc.create(req).await.expect("should succeed with no email/phone");
    assert_eq!(resp.locale, "th"); // defaults to Th
}

#[tokio::test]
async fn create_duplicate_email_returns_bad_request() {
    let existing = make_customer(Some("dup@example.com"), None, false);
    let repo = Arc::new(MockCustomerRepository::with_customer(existing));
    let uc = CustomerUseCases::new(repo, test_config());

    let req = CreateCustomerRequest {
        email: Some("dup@example.com".to_string()),
        phone: None,
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    let err = uc.create(req).await.expect_err("should fail");
    assert!(matches!(err, ApplicationError::BadRequest(m) if m.contains("email")));
}

#[tokio::test]
async fn create_duplicate_phone_returns_bad_request() {
    let existing = make_customer(None, Some("+66812345678"), false);
    let repo = Arc::new(MockCustomerRepository::with_customer(existing));
    let uc = CustomerUseCases::new(repo, test_config());

    let req = CreateCustomerRequest {
        email: None,
        // "0812345678" normalises to "+66812345678" which is already taken
        phone: Some("0812345678".to_string()),
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    let err = uc.create(req).await.expect_err("should fail");
    assert!(matches!(err, ApplicationError::BadRequest(m) if m.contains("phone")));
}

#[tokio::test]
async fn create_repo_error_propagated() {
    let repo = Arc::new(MockCustomerRepository::failing());
    let uc = CustomerUseCases::new(repo, test_config());

    let req = CreateCustomerRequest {
        email: None,
        phone: None,
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    let err = uc.create(req).await.expect_err("should propagate repo error");
    assert!(matches!(err, ApplicationError::Repository(_)));
}

// ---- get_by_id tests ----

#[tokio::test]
async fn get_by_id_found() {
    let customer = make_customer(Some("bob@example.com"), None, false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let resp = uc.get_by_id(id).await.expect("should find customer");
    assert_eq!(resp.id, id.to_string());
}

#[tokio::test]
async fn get_by_id_not_found_returns_not_found() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let err = uc
        .get_by_id(Uuid::new_v4())
        .await
        .expect_err("should return not found");
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

#[tokio::test]
async fn get_by_id_deleted_customer_returns_not_found() {
    let customer = make_customer(None, None, true); // is_deleted = true
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let err = uc
        .get_by_id(id)
        .await
        .expect_err("deleted customer should be not-found");
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

// ---- search tests ----

#[tokio::test]
async fn search_by_id_returns_results() {
    let customer = make_customer(Some("carol@example.com"), None, false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let query = SearchCustomerQuery {
        id: Some(id.to_string()),
        phone: None,
        the1_member_id: None,
        the1_card_number: None,
    };

    let results = uc.search(query).await.expect("should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id.to_string());
}

#[tokio::test]
async fn search_by_phone_returns_results() {
    let customer = make_customer(None, Some("+66888888888"), false);
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let query = SearchCustomerQuery {
        id: None,
        phone: Some("+66888888888".to_string()),
        the1_member_id: None,
        the1_card_number: None,
    };

    let results = uc.search(query).await.expect("should succeed");
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn search_no_params_returns_bad_request() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let query = SearchCustomerQuery {
        id: None,
        phone: None,
        the1_member_id: None,
        the1_card_number: None,
    };

    let err = uc.search(query).await.expect_err("should fail");
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn search_invalid_id_format_returns_bad_request() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let query = SearchCustomerQuery {
        id: Some("not-a-uuid".to_string()),
        phone: None,
        the1_member_id: None,
        the1_card_number: None,
    };

    let err = uc.search(query).await.expect_err("should fail");
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn search_empty_results() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let query = SearchCustomerQuery {
        id: None,
        phone: Some("+66999999999".to_string()),
        the1_member_id: None,
        the1_card_number: None,
    };

    let results = uc.search(query).await.expect("should succeed with empty");
    assert!(results.is_empty());
}

// ---- get_me tests ----

#[tokio::test]
async fn get_me_found() {
    let customer = make_customer(Some("dave@example.com"), None, false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let resp = uc.get_me(id).await.expect("should succeed");
    assert_eq!(resp.id, id.to_string());
}

#[tokio::test]
async fn get_me_not_found() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let err = uc
        .get_me(Uuid::new_v4())
        .await
        .expect_err("should return not found");
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

#[tokio::test]
async fn get_me_deleted_returns_not_found() {
    let customer = make_customer(None, None, true);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let err = uc
        .get_me(id)
        .await
        .expect_err("deleted customer should be not-found");
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

// ---- update_me tests ----

#[tokio::test]
async fn update_me_happy_path() {
    let customer = make_customer(Some("eve@example.com"), None, false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let req = UpdateCustomerRequest {
        email: None,
        locale: Some("en".to_string()),
        has_consent: Some(true),
        first_name: Some("Eve".to_string()),
        last_name: Some("Updated".to_string()),
        birthdate: Some("1992-06-10".to_string()),
        gender: Some("female".to_string()),
        nationality: Some("US".to_string()),
    };

    let resp = uc.update_me(id, req).await.expect("should succeed");
    assert_eq!(resp.locale, "en");
    assert_eq!(resp.first_name, Some("Eve".to_string()));
    assert_eq!(resp.gender, Some("female".to_string()));
}

#[tokio::test]
async fn update_me_email_conflict_with_other_user_returns_bad_request() {
    // Two customers: frank and grace. Frank tries to use grace's email.
    let frank = make_customer(Some("frank@example.com"), None, false);
    let grace = make_customer(Some("grace@example.com"), None, false);
    let frank_id = frank.id;

    let mut map = HashMap::new();
    map.insert(frank.id, frank);
    map.insert(grace.id, grace);
    let repo = Arc::new(MockCustomerRepository {
        store: Arc::new(Mutex::new(map)),
        should_fail: false,
    });
    let uc = CustomerUseCases::new(repo, test_config());

    let req = UpdateCustomerRequest {
        email: Some("grace@example.com".to_string()),
        locale: None,
        has_consent: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    let err = uc
        .update_me(frank_id, req)
        .await
        .expect_err("should fail");
    assert!(matches!(err, ApplicationError::BadRequest(m) if m.contains("email")));
}

#[tokio::test]
async fn update_me_same_email_allowed() {
    let customer = make_customer(Some("henry@example.com"), None, false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    // Sending the same email the user already owns should not return an error.
    let req = UpdateCustomerRequest {
        email: Some("henry@example.com".to_string()),
        locale: None,
        has_consent: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    uc.update_me(id, req).await.expect("same email should be allowed");
}

// ---- delete tests ----

#[tokio::test]
async fn delete_happy_path() {
    let customer = make_customer(Some("iris@example.com"), Some("+66811111111"), false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo, test_config());

    let resp = uc.delete(id).await.expect("should succeed");
    assert!(resp.is_deleted);
}

#[tokio::test]
async fn delete_not_found_propagates_error() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    // soft_delete on a missing id should propagate a repository NotFound as Repository error
    let err = uc
        .delete(Uuid::new_v4())
        .await
        .expect_err("should fail");
    assert!(matches!(err, ApplicationError::Repository(RepositoryError::NotFound(_))));
}

// ---- update_profile_image tests ----

#[tokio::test]
async fn update_profile_image_set_key() {
    let customer = make_customer(None, None, false);
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo.clone(), test_config());

    uc.update_profile_image(id, Some("profiles/abc.jpg".to_string()))
        .await
        .expect("should succeed");

    // Verify image was stored in the mock
    let store = repo.store.lock().unwrap();
    let profile_image = store[&id]
        .profile
        .as_ref()
        .and_then(|p| p.profile_image.as_deref());
    assert_eq!(profile_image, Some("profiles/abc.jpg"));
}

#[tokio::test]
async fn update_profile_image_clear_key() {
    let mut customer = make_customer(None, None, false);
    if let Some(ref mut p) = customer.profile {
        p.profile_image = Some("old-key.jpg".to_string());
    }
    let id = customer.id;
    let repo = Arc::new(MockCustomerRepository::with_customer(customer));
    let uc = CustomerUseCases::new(repo.clone(), test_config());

    uc.update_profile_image(id, None)
        .await
        .expect("should succeed");

    let store = repo.store.lock().unwrap();
    let profile_image = store[&id].profile.as_ref().and_then(|p| p.profile_image.as_deref());
    assert!(profile_image.is_none());
}

// ---- phone normalisation edge cases ----

#[tokio::test]
async fn phone_without_leading_zero_is_unchanged() {
    let repo = Arc::new(MockCustomerRepository::new());
    let uc = CustomerUseCases::new(repo, test_config());

    let req = CreateCustomerRequest {
        email: None,
        phone: Some("+66812345678".to_string()), // already normalised
        locale: None,
        has_consent: None,
        client_id: None,
        first_name: None,
        last_name: None,
        birthdate: None,
        gender: None,
        nationality: None,
    };

    let resp = uc.create(req).await.expect("should succeed");
    assert_eq!(resp.phone, Some("+66812345678".to_string()));
}
