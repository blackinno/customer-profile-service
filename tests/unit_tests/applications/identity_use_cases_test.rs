use async_trait::async_trait;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use application::errors::ApplicationError;
use application::identities::use_cases::IdentityUseCases;
use domain::entities::customer::{CreateCustomer, Customer, SearchField, UpdateCustomer};
use domain::entities::identity::{CreateIdentity, Identity};
use domain::errors::RepositoryError;
use domain::repositories::customer_repository::CustomerRepository;
use domain::repositories::identity_repository::IdentityRepository;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_identity(id: Uuid, user_uuid: Uuid, provider: &str, external_id: &str) -> Identity {
    Identity {
        id,
        user_uuid,
        provider_name: provider.to_string(),
        external_id: external_id.to_string(),
        provider_id_token: Some("id-token".to_string()),
        provider_access_token: Some("access-token".to_string()),
        provider_refresh_token: Some("refresh-token".to_string()),
        is_deleted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_deleted_identity(id: Uuid, user_uuid: Uuid, provider: &str, external_id: &str) -> Identity {
    Identity {
        is_deleted: true,
        ..make_identity(id, user_uuid, provider, external_id)
    }
}

// ---------------------------------------------------------------------------
// Mock IdentityRepository
// ---------------------------------------------------------------------------

struct MockIdentityRepo {
    /// All identities held in the store (active and deleted).
    store: Mutex<Vec<Identity>>,
    /// Recorded audit log entries.
    log: Mutex<Vec<(Uuid, String, String, String)>>,
}

impl MockIdentityRepo {
    fn new(initial: Vec<Identity>) -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(initial),
            log: Mutex::new(vec![]),
        })
    }

    fn log_entries(&self) -> Vec<(Uuid, String, String, String)> {
        self.log.lock().unwrap().clone()
    }
}

#[async_trait]
impl IdentityRepository for MockIdentityRepo {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Vec<Identity>, RepositoryError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|i| i.user_uuid == user_uuid && !i.is_deleted)
            .cloned()
            .collect())
    }

    async fn find_active(
        &self,
        user_uuid: Uuid,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .find(|i| {
                i.user_uuid == user_uuid
                    && i.provider_name == provider
                    && i.external_id == external_id
                    && !i.is_deleted
            })
            .cloned())
    }

    async fn find_deleted(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .find(|i| i.provider_name == provider && i.external_id == external_id && i.is_deleted)
            .cloned())
    }

    async fn create(&self, data: CreateIdentity) -> Result<Identity, RepositoryError> {
        let identity = Identity {
            id: Uuid::new_v4(),
            user_uuid: data.user_uuid,
            provider_name: data.provider_name,
            external_id: data.external_id,
            provider_id_token: data.provider_id_token,
            provider_access_token: data.provider_access_token,
            provider_refresh_token: data.provider_refresh_token,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.store.lock().unwrap().push(identity.clone());
        Ok(identity)
    }

    async fn restore(
        &self,
        id: Uuid,
        user_uuid: Uuid,
        tokens: CreateIdentity,
    ) -> Result<Identity, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let entry = store
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| RepositoryError::NotFound("identity not found".into()))?;

        entry.user_uuid = user_uuid;
        entry.is_deleted = false;
        entry.provider_id_token = tokens.provider_id_token;
        entry.provider_access_token = tokens.provider_access_token;
        entry.provider_refresh_token = tokens.provider_refresh_token;
        entry.updated_at = Utc::now();

        Ok(entry.clone())
    }

    async fn soft_delete(&self, id: Uuid, user_uuid: Uuid) -> Result<Identity, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let entry = store
            .iter_mut()
            .find(|i| i.id == id && i.user_uuid == user_uuid)
            .ok_or_else(|| RepositoryError::NotFound("identity not found".into()))?;

        entry.is_deleted = true;
        entry.updated_at = Utc::now();

        Ok(entry.clone())
    }

    async fn update_tokens(
        &self,
        id: Uuid,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<Identity, RepositoryError> {
        let mut store = self.store.lock().unwrap();
        let entry = store
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| RepositoryError::NotFound("identity not found".into()))?;

        entry.provider_access_token = access_token;
        entry.provider_refresh_token = refresh_token;
        entry.updated_at = Utc::now();

        Ok(entry.clone())
    }

    async fn log_transaction(
        &self,
        user_uuid: Uuid,
        action: &str,
        provider: &str,
        external_id: &str,
    ) -> Result<(), RepositoryError> {
        self.log.lock().unwrap().push((
            user_uuid,
            action.to_string(),
            provider.to_string(),
            external_id.to_string(),
        ));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mock CustomerRepository (no-op — IdentityUseCases holds the dep but the
// identity use-case methods tested here do not invoke it)
// ---------------------------------------------------------------------------

struct MockCustomerRepo;

#[async_trait]
impl CustomerRepository for MockCustomerRepo {
    async fn create(&self, _: CreateCustomer) -> Result<Customer, RepositoryError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<Customer>, RepositoryError> {
        unimplemented!()
    }
    async fn find_by_phone(&self, _: &str) -> Result<Option<Customer>, RepositoryError> {
        unimplemented!()
    }
    async fn find_by_email(&self, _: &str) -> Result<Option<Customer>, RepositoryError> {
        unimplemented!()
    }
    async fn search(&self, _: SearchField) -> Result<Vec<Customer>, RepositoryError> {
        unimplemented!()
    }
    async fn update(&self, _: Uuid, _: UpdateCustomer) -> Result<Customer, RepositoryError> {
        unimplemented!()
    }
    async fn soft_delete(&self, _: Uuid) -> Result<Customer, RepositoryError> {
        unimplemented!()
    }
    async fn update_profile_image(
        &self,
        _: Uuid,
        _: Option<String>,
    ) -> Result<(), RepositoryError> {
        unimplemented!()
    }
}

fn build_use_cases(repo: Arc<MockIdentityRepo>) -> IdentityUseCases {
    IdentityUseCases::new(repo, Arc::new(MockCustomerRepo))
}

// ---------------------------------------------------------------------------
// Tests — get_identities
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_identities_returns_active_identities_for_user() {
    let user = Uuid::new_v4();
    let id = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(id, user, "google", "ext-1")]);
    let uc = build_use_cases(repo);

    let result = uc.get_identities(user).await.unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, id.to_string());
    assert_eq!(result[0].provider_name, "google");
}

#[tokio::test]
async fn get_identities_returns_empty_for_user_with_no_identities() {
    let repo = MockIdentityRepo::new(vec![]);
    let uc = build_use_cases(repo);

    let result = uc.get_identities(Uuid::new_v4()).await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn get_identities_does_not_return_other_users_identities() {
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(Uuid::new_v4(), user_b, "google", "ext-1")]);
    let uc = build_use_cases(repo);

    let result = uc.get_identities(user_a).await.unwrap();
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Tests — get_identities_internal (same logic, separate route)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_identities_internal_returns_active_identities() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(Uuid::new_v4(), user, "the1", "ext-99")]);
    let uc = build_use_cases(repo);

    let result = uc.get_identities_internal(user).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].provider_name, "the1");
}

// ---------------------------------------------------------------------------
// Tests — create_identity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_identity_happy_path_creates_new_identity() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![]);
    let uc = build_use_cases(repo);

    let req = application::identities::dtos::CreateIdentityRequest {
        provider_name: "google".to_string(),
        external_id: "ext-new".to_string(),
        provider_id_token: Some("id-tok".to_string()),
        provider_access_token: Some("acc-tok".to_string()),
        provider_refresh_token: Some("ref-tok".to_string()),
    };

    let result = uc.create_identity(user, req).await.unwrap();
    assert_eq!(result.provider_name, "google");
    assert_eq!(result.external_id, "ext-new");
    assert!(!result.is_deleted);
}

#[tokio::test]
async fn create_identity_returns_bad_request_when_already_linked() {
    let user = Uuid::new_v4();
    let id = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(id, user, "google", "ext-1")]);
    let uc = build_use_cases(repo);

    let req = application::identities::dtos::CreateIdentityRequest {
        provider_name: "google".to_string(),
        external_id: "ext-1".to_string(),
        provider_id_token: None,
        provider_access_token: None,
        provider_refresh_token: None,
    };

    let err = uc.create_identity(user, req).await.unwrap_err();
    assert!(
        matches!(err, ApplicationError::BadRequest(ref msg) if msg == "identity already linked"),
        "expected BadRequest(identity already linked), got {err:?}"
    );
}

#[tokio::test]
async fn create_identity_restores_deleted_identity_for_same_user() {
    let user = Uuid::new_v4();
    let old_id = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_deleted_identity(old_id, user, "google", "ext-del")]);
    let uc = build_use_cases(repo);

    let req = application::identities::dtos::CreateIdentityRequest {
        provider_name: "google".to_string(),
        external_id: "ext-del".to_string(),
        provider_id_token: Some("new-id-tok".to_string()),
        provider_access_token: Some("new-acc".to_string()),
        provider_refresh_token: Some("new-ref".to_string()),
    };

    let result = uc.create_identity(user, req).await.unwrap();

    // The restored row reuses the same ID
    assert_eq!(result.id, old_id.to_string());
    assert!(!result.is_deleted);
    assert_eq!(result.user_uuid, user.to_string());
    assert_eq!(result.provider_access_token, Some("new-acc".to_string()));
}

#[tokio::test]
async fn create_identity_restores_and_reassigns_deleted_identity_to_new_user() {
    let original_user = Uuid::new_v4();
    let new_user = Uuid::new_v4();
    let old_id = Uuid::new_v4();

    // Deleted row still carries the original user's UUID
    let repo = MockIdentityRepo::new(vec![make_deleted_identity(
        old_id,
        original_user,
        "apple",
        "ext-apple",
    )]);
    let uc = build_use_cases(repo);

    let req = application::identities::dtos::CreateIdentityRequest {
        provider_name: "apple".to_string(),
        external_id: "ext-apple".to_string(),
        provider_id_token: None,
        provider_access_token: Some("new-acc".to_string()),
        provider_refresh_token: None,
    };

    let result = uc.create_identity(new_user, req).await.unwrap();

    assert_eq!(result.id, old_id.to_string());
    // Must be reassigned to the new user
    assert_eq!(result.user_uuid, new_user.to_string());
    assert!(!result.is_deleted);
}

// ---------------------------------------------------------------------------
// Tests — delete_identity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_identity_happy_path_soft_deletes_and_logs() {
    let user = Uuid::new_v4();
    let id = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(id, user, "google", "ext-del")]);
    let log_ref = Arc::clone(&repo);
    let uc = build_use_cases(repo);

    uc.delete_identity(user, "google".into(), "ext-del".into())
        .await
        .unwrap();

    let entries = log_ref.log_entries();
    assert_eq!(entries.len(), 1);
    let (logged_user, action, provider, ext_id) = &entries[0];
    assert_eq!(*logged_user, user);
    assert_eq!(action, "delete");
    assert_eq!(provider, "google");
    assert_eq!(ext_id, "ext-del");
}

#[tokio::test]
async fn delete_identity_returns_not_found_when_no_active_identity() {
    let repo = MockIdentityRepo::new(vec![]);
    let uc = build_use_cases(repo);

    let err = uc
        .delete_identity(Uuid::new_v4(), "google".into(), "ext-missing".into())
        .await
        .unwrap_err();

    assert!(
        matches!(err, ApplicationError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Tests — invoke_token
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invoke_token_returns_stored_tokens_for_existing_provider() {
    let user = Uuid::new_v4();
    let mut identity = make_identity(Uuid::new_v4(), user, "the1", "ext-the1");
    identity.provider_access_token = Some("the1-access".to_string());
    identity.provider_refresh_token = Some("the1-refresh".to_string());

    let repo = MockIdentityRepo::new(vec![identity]);
    let uc = build_use_cases(repo);

    let result = uc.invoke_token(user, "the1".to_string()).await.unwrap();

    assert_eq!(result.access_token, Some("the1-access".to_string()));
    assert_eq!(result.refresh_token, Some("the1-refresh".to_string()));
}

#[tokio::test]
async fn invoke_token_returns_not_found_when_provider_absent() {
    let user = Uuid::new_v4();
    let repo = MockIdentityRepo::new(vec![make_identity(Uuid::new_v4(), user, "google", "ext-1")]);
    let uc = build_use_cases(repo);

    // User has a google identity but not a 'the1' identity
    let err = uc
        .invoke_token(user, "the1".to_string())
        .await
        .unwrap_err();

    assert!(
        matches!(err, ApplicationError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}

#[tokio::test]
async fn invoke_token_returns_not_found_when_user_has_no_identities() {
    let repo = MockIdentityRepo::new(vec![]);
    let uc = build_use_cases(repo);

    let err = uc
        .invoke_token(Uuid::new_v4(), "the1".to_string())
        .await
        .unwrap_err();

    assert!(
        matches!(err, ApplicationError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}
