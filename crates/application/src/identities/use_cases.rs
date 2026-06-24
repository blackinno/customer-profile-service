use std::sync::Arc;

use domain::entities::identity::CreateIdentity;
use domain::repositories::identity_repository::IdentityRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApplicationError;
use crate::events::{IdentityLinkedChangedPayload, NoopPublisher, Publisher};
use crate::identities::dtos::{CreateIdentityRequest, IdentityResponse, InvokeTokenResponse};

pub struct IdentityUseCases {
    identities: Arc<dyn IdentityRepository>,
    publisher: Arc<dyn Publisher>,
    sns_topic: String,
}

impl IdentityUseCases {
    pub fn new(identities: Arc<dyn IdentityRepository>) -> Self {
        Self {
            identities,
            publisher: Arc::new(NoopPublisher),
            sns_topic: String::new(),
        }
    }

    pub fn with_publisher(mut self, publisher: Arc<dyn Publisher>, config: &AppConfig) -> Self {
        self.publisher = publisher;
        self.sns_topic = config.sns_user_identity_linked_changed.clone();
        self
    }

    /// Return all active identities for a user (customer-facing route).
    pub async fn get_identities(
        &self,
        user_uuid: Uuid,
    ) -> Result<Vec<IdentityResponse>, ApplicationError> {
        let identities = self.identities.find_by_user(user_uuid).await?;
        Ok(identities.into_iter().map(IdentityResponse::from).collect())
    }

    /// Return all active identities for a user (internal/admin route).
    /// Logic is identical to `get_identities` but kept separate to allow
    /// independent auth policies on each route.
    pub async fn get_identities_internal(
        &self,
        user_uuid: Uuid,
    ) -> Result<Vec<IdentityResponse>, ApplicationError> {
        let identities = self.identities.find_by_user(user_uuid).await?;
        Ok(identities.into_iter().map(IdentityResponse::from).collect())
    }

    /// Link a new provider identity to a user.
    ///
    /// Three paths:
    /// 1. Active identity already exists for this user/provider/external_id → `BadRequest`.
    /// 2. A soft-deleted row exists for this provider/external_id:
    ///    - If it belonged to a *different* user → restore and reassign to the
    ///      calling user (provider account was transferred).
    ///    - If it belonged to the *same* user → restore with updated tokens.
    /// 3. No row exists → fresh INSERT.
    pub async fn create_identity(
        &self,
        user_uuid: Uuid,
        req: CreateIdentityRequest,
    ) -> Result<IdentityResponse, ApplicationError> {
        // Guard: already linked and active
        if self
            .identities
            .find_active(user_uuid, &req.provider_name, &req.external_id)
            .await?
            .is_some()
        {
            return Err(ApplicationError::BadRequest(
                "identity already linked".to_string(),
            ));
        }

        let tokens = CreateIdentity {
            user_uuid,
            provider_name: req.provider_name.clone(),
            external_id: req.external_id.clone(),
            provider_id_token: req.provider_id_token.clone(),
            provider_access_token: req.provider_access_token.clone(),
            provider_refresh_token: req.provider_refresh_token.clone(),
        };

        // Check whether a deleted row can be recycled
        if let Some(deleted) = self
            .identities
            .find_deleted(&req.provider_name, &req.external_id)
            .await?
        {
            // Restore and potentially re-assign to a new user
            let restored = self
                .identities
                .restore(deleted.id, user_uuid, tokens)
                .await?;
            let payload = serde_json::to_string(&IdentityLinkedChangedPayload {
                user_uuid: user_uuid.to_string(),
                provider_name: req.provider_name.clone(),
                action: "linked".to_string(),
            })
            .unwrap_or_default();
            if let Err(e) = self.publisher.publish(&self.sns_topic, &payload).await {
                tracing::warn!("sns publish failed (identity linked): {e}");
            }
            return Ok(IdentityResponse::from(restored));
        }

        // No recyclable row — create fresh
        let created = self.identities.create(tokens).await?;
        let payload = serde_json::to_string(&IdentityLinkedChangedPayload {
            user_uuid: user_uuid.to_string(),
            provider_name: req.provider_name.clone(),
            action: "linked".to_string(),
        })
        .unwrap_or_default();
        if let Err(e) = self.publisher.publish(&self.sns_topic, &payload).await {
            tracing::warn!("sns publish failed (identity linked): {e}");
        }
        Ok(IdentityResponse::from(created))
    }

    /// Soft-delete an identity and append an audit transaction record.
    pub async fn delete_identity(
        &self,
        user_uuid: Uuid,
        provider: String,
        external_id: String,
    ) -> Result<(), ApplicationError> {
        let identity = self
            .identities
            .find_active(user_uuid, &provider, &external_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound("identity not found".to_string()))?;

        self.identities.soft_delete(identity.id, user_uuid).await?;

        self.identities
            .log_transaction(user_uuid, "delete", &provider, &external_id)
            .await?;

        let payload = serde_json::to_string(&IdentityLinkedChangedPayload {
            user_uuid: user_uuid.to_string(),
            provider_name: provider.clone(),
            action: "unlinked".to_string(),
        })
        .unwrap_or_default();
        if let Err(e) = self.publisher.publish(&self.sns_topic, &payload).await {
            tracing::warn!("sns publish failed (identity unlinked): {e}");
        }

        Ok(())
    }

    /// Return the currently stored provider tokens for a user's identity.
    ///
    /// Note: The live The1 token-refresh HTTP call is deferred to Task 24.
    /// For now this returns whatever tokens are already persisted.
    pub async fn invoke_token(
        &self,
        user_uuid: Uuid,
        provider_name: String,
    ) -> Result<InvokeTokenResponse, ApplicationError> {
        let identities = self.identities.find_by_user(user_uuid).await?;

        let identity = identities
            .into_iter()
            .find(|i| i.provider_name == provider_name)
            .ok_or_else(|| {
                ApplicationError::NotFound(format!(
                    "no active identity found for provider '{provider_name}'"
                ))
            })?;

        // TODO(task-24): call the1_client.invoke_token(refresh_token) and
        // persist the updated tokens before returning.

        Ok(InvokeTokenResponse {
            access_token: identity.provider_access_token,
            refresh_token: identity.provider_refresh_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use domain::{
        entities::identity::{CreateIdentity, Identity},
        errors::RepositoryError,
        repositories::identity_repository::IdentityRepository,
    };

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
}
