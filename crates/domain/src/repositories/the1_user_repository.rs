use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::the1_user::{The1User, UpsertThe1User};
use crate::errors::RepositoryError;

#[async_trait]
pub trait The1UserRepository: Send + Sync {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Option<The1User>, RepositoryError>;
    async fn find_by_card_number(
        &self,
        card_number: &str,
    ) -> Result<Option<The1User>, RepositoryError>;
    async fn find_by_member_id(
        &self,
        member_id: &str,
    ) -> Result<Option<The1User>, RepositoryError>;
    async fn upsert(
        &self,
        user_uuid: Uuid,
        profile: UpsertThe1User,
    ) -> Result<The1User, RepositoryError>;
}
