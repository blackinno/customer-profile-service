use std::sync::Arc;

use domain::entities::identity::CreateIdentity;
use domain::repositories::customer_repository::CustomerRepository;
use domain::repositories::identity_repository::IdentityRepository;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::identities::dtos::{CreateIdentityRequest, IdentityResponse, InvokeTokenResponse};

pub struct IdentityUseCases {
    identities: Arc<dyn IdentityRepository>,
    customers: Arc<dyn CustomerRepository>,
}

impl IdentityUseCases {
    pub fn new(
        identities: Arc<dyn IdentityRepository>,
        customers: Arc<dyn CustomerRepository>,
    ) -> Self {
        Self { identities, customers }
    }

    /// Map a domain `Identity` into the API-facing `IdentityResponse` DTO.
    fn to_response(identity: domain::entities::identity::Identity) -> IdentityResponse {
        IdentityResponse {
            id: identity.id.to_string(),
            user_uuid: identity.user_uuid.to_string(),
            provider_name: identity.provider_name,
            external_id: identity.external_id,
            provider_id_token: identity.provider_id_token,
            provider_access_token: identity.provider_access_token,
            provider_refresh_token: identity.provider_refresh_token,
            is_deleted: identity.is_deleted,
            created_at: identity.created_at.to_rfc3339(),
            updated_at: identity.updated_at.to_rfc3339(),
        }
    }

    /// Return all active identities for a user (customer-facing route).
    pub async fn get_identities(
        &self,
        user_uuid: Uuid,
    ) -> Result<Vec<IdentityResponse>, ApplicationError> {
        let identities = self.identities.find_by_user(user_uuid).await?;
        Ok(identities.into_iter().map(Self::to_response).collect())
    }

    /// Return all active identities for a user (internal/admin route).
    /// Logic is identical to `get_identities` but kept separate to allow
    /// independent auth policies on each route.
    pub async fn get_identities_internal(
        &self,
        user_uuid: Uuid,
    ) -> Result<Vec<IdentityResponse>, ApplicationError> {
        let identities = self.identities.find_by_user(user_uuid).await?;
        Ok(identities.into_iter().map(Self::to_response).collect())
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
            return Ok(Self::to_response(restored));
        }

        // No recyclable row — create fresh
        let created = self.identities.create(tokens).await?;
        Ok(Self::to_response(created))
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
