use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::identity::{CreateIdentity, Identity};
use crate::errors::RepositoryError;

#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Vec<Identity>, RepositoryError>;
    async fn find_active(
        &self,
        user_uuid: Uuid,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError>;
    async fn find_deleted(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError>;
    async fn create(&self, data: CreateIdentity) -> Result<Identity, RepositoryError>;
    async fn restore(
        &self,
        id: Uuid,
        user_uuid: Uuid,
        tokens: CreateIdentity,
    ) -> Result<Identity, RepositoryError>;
    async fn soft_delete(&self, id: Uuid, user_uuid: Uuid) -> Result<Identity, RepositoryError>;
    async fn update_tokens(
        &self,
        id: Uuid,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<Identity, RepositoryError>;
    async fn log_transaction(
        &self,
        user_uuid: Uuid,
        action: &str,
        provider: &str,
        external_id: &str,
    ) -> Result<(), RepositoryError>;
}
