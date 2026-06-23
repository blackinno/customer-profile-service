use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::profile_change::{
    ChangeStatus, ChangeType, CreateProfileChange, ProfileChange,
};
use crate::errors::RepositoryError;

#[async_trait]
pub trait ProfileChangeRepository: Send + Sync {
    async fn create(&self, data: CreateProfileChange) -> Result<ProfileChange, RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ProfileChange>, RepositoryError>;
    async fn find_active_by_user_and_type(
        &self,
        user_uuid: Uuid,
        change_type: ChangeType,
    ) -> Result<Option<ProfileChange>, RepositoryError>;
    async fn update_otp(
        &self,
        id: Uuid,
        otp: String,
        ref_code: String,
        expires: DateTime<Utc>,
        next_request: DateTime<Utc>,
    ) -> Result<ProfileChange, RepositoryError>;
    async fn update_status_and_token(
        &self,
        id: Uuid,
        status: ChangeStatus,
        token: Option<String>,
        token_expires: Option<DateTime<Utc>>,
    ) -> Result<ProfileChange, RepositoryError>;
}
