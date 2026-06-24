use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    entities::identity::{CreateIdentity, Identity},
    errors::RepositoryError,
    repositories::identity_repository::IdentityRepository,
};
use uuid::Uuid;

use application::errors::ApplicationError;
use application::identities::dtos::CreateIdentityRequest;
use application::identities::use_cases::IdentityUseCases;

fn make_identity(id: Uuid, user_uuid: Uuid) -> Identity {
    Identity {
        id,
        user_uuid,
        provider_name: "google".to_string(),
        external_id: "google-123".to_string(),
        provider_id_token: None,
        provider_access_token: Some("access".to_string()),
        provider_refresh_token: Some("refresh".to_string()),
        is_deleted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

struct MockIdentityRepo {
    find_by_user: Vec<Identity>,
    find_active: Option<Identity>,
    find_deleted: Option<Identity>,
    create_result: Identity,
    restore_result: Identity,
}

impl MockIdentityRepo {
    fn new(user_uuid: Uuid) -> Self {
        let i = make_identity(Uuid::new_v4(), user_uuid);
        Self {
            find_by_user: vec![i.clone()],
            find_active: None,
            find_deleted: None,
            create_result: i.clone(),
            restore_result: i,
        }
    }
}

#[async_trait]
impl IdentityRepository for MockIdentityRepo {
    async fn find_by_user(&self, _: Uuid) -> Result<Vec<Identity>, RepositoryError> {
        Ok(self.find_by_user.clone())
    }
    async fn find_active(&self, _: Uuid, _: &str, _: &str) -> Result<Option<Identity>, RepositoryError> {
        Ok(self.find_active.clone())
    }
    async fn find_deleted(&self, _: &str, _: &str) -> Result<Option<Identity>, RepositoryError> {
        Ok(self.find_deleted.clone())
    }
    async fn create(&self, _: CreateIdentity) -> Result<Identity, RepositoryError> {
        Ok(self.create_result.clone())
    }
    async fn restore(&self, _: Uuid, _: Uuid, _: CreateIdentity) -> Result<Identity, RepositoryError> {
        Ok(self.restore_result.clone())
    }
    async fn soft_delete(&self, _: Uuid, _: Uuid) -> Result<Identity, RepositoryError> {
        Ok(self.create_result.clone())
    }
    async fn update_tokens(&self, _: Uuid, _: Option<String>, _: Option<String>) -> Result<Identity, RepositoryError> {
        Ok(self.create_result.clone())
    }
    async fn log_transaction(&self, _: Uuid, _: &str, _: &str, _: &str) -> Result<(), RepositoryError> {
        Ok(())
    }
}

fn use_cases(repo: MockIdentityRepo) -> IdentityUseCases {
    IdentityUseCases::new(Arc::new(repo))
}

// ── get_identities ───────────────────────────────────────────────────────

#[tokio::test]
async fn get_identities_returns_mapped_list() {
    let user_uuid = Uuid::new_v4();
    let uc = use_cases(MockIdentityRepo::new(user_uuid));
    let result = uc.get_identities(user_uuid).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].provider_name, "google");
}

#[tokio::test]
async fn get_identities_returns_empty_list() {
    let user_uuid = Uuid::new_v4();
    let mut mock = MockIdentityRepo::new(user_uuid);
    mock.find_by_user = vec![];
    let uc = use_cases(mock);
    assert!(uc.get_identities(user_uuid).await.unwrap().is_empty());
}

// ── create_identity ──────────────────────────────────────────────────────

#[tokio::test]
async fn create_identity_fresh_insert() {
    let user_uuid = Uuid::new_v4();
    let uc = use_cases(MockIdentityRepo::new(user_uuid));
    let req = CreateIdentityRequest {
        provider_name: "google".to_string(),
        external_id: "google-123".to_string(),
        provider_id_token: None,
        provider_access_token: None,
        provider_refresh_token: None,
    };
    let res = uc.create_identity(user_uuid, req).await.unwrap();
    assert_eq!(res.provider_name, "google");
}

#[tokio::test]
async fn create_identity_rejects_already_linked() {
    let user_uuid = Uuid::new_v4();
    let mut mock = MockIdentityRepo::new(user_uuid);
    mock.find_active = Some(make_identity(Uuid::new_v4(), user_uuid));
    let uc = use_cases(mock);
    let req = CreateIdentityRequest {
        provider_name: "google".to_string(),
        external_id: "google-123".to_string(),
        provider_id_token: None,
        provider_access_token: None,
        provider_refresh_token: None,
    };
    let err = uc.create_identity(user_uuid, req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn create_identity_restores_deleted_row() {
    let user_uuid = Uuid::new_v4();
    let mut mock = MockIdentityRepo::new(user_uuid);
    mock.find_deleted = Some(make_identity(Uuid::new_v4(), Uuid::new_v4()));
    let uc = use_cases(mock);
    let req = CreateIdentityRequest {
        provider_name: "google".to_string(),
        external_id: "google-123".to_string(),
        provider_id_token: None,
        provider_access_token: None,
        provider_refresh_token: None,
    };
    let res = uc.create_identity(user_uuid, req).await.unwrap();
    assert_eq!(res.provider_name, "google");
}

// ── delete_identity ──────────────────────────────────────────────────────

#[tokio::test]
async fn delete_identity_succeeds() {
    let user_uuid = Uuid::new_v4();
    let mut mock = MockIdentityRepo::new(user_uuid);
    mock.find_active = Some(make_identity(Uuid::new_v4(), user_uuid));
    let uc = use_cases(mock);
    assert!(uc.delete_identity(user_uuid, "google".to_string(), "google-123".to_string()).await.is_ok());
}

#[tokio::test]
async fn delete_identity_not_found() {
    let user_uuid = Uuid::new_v4();
    let uc = use_cases(MockIdentityRepo::new(user_uuid)); // find_active is None
    let err = uc.delete_identity(user_uuid, "google".to_string(), "google-123".to_string()).await.unwrap_err();
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

// ── invoke_token ─────────────────────────────────────────────────────────

#[tokio::test]
async fn invoke_token_returns_stored_tokens() {
    let user_uuid = Uuid::new_v4();
    let uc = use_cases(MockIdentityRepo::new(user_uuid));
    let res = uc.invoke_token(user_uuid, "google".to_string()).await.unwrap();
    assert_eq!(res.access_token, Some("access".to_string()));
}

#[tokio::test]
async fn invoke_token_not_found_for_provider() {
    let user_uuid = Uuid::new_v4();
    let mut mock = MockIdentityRepo::new(user_uuid);
    mock.find_by_user = vec![];
    let uc = use_cases(mock);
    let err = uc.invoke_token(user_uuid, "facebook".to_string()).await.unwrap_err();
    assert!(matches!(err, ApplicationError::NotFound(_)));
}
