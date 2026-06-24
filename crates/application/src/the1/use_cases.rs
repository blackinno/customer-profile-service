use std::sync::Arc;

use domain::repositories::the1_user_repository::The1UserRepository;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::the1::dtos::The1AccountResponse;

pub struct The1UseCases {
    the1_users: Arc<dyn The1UserRepository>,
}

impl The1UseCases {
    pub fn new(the1_users: Arc<dyn The1UserRepository>) -> Self {
        Self { the1_users }
    }

    /// Return the The1 account for a given platform user. Returns `NotFound`
    /// if no The1 membership has been linked yet.
    pub async fn get_the1_account(
        &self,
        user_uuid: Uuid,
    ) -> Result<The1AccountResponse, ApplicationError> {
        let user = self
            .the1_users
            .find_by_user(user_uuid)
            .await?
            .ok_or_else(|| ApplicationError::NotFound("the1 account not found".to_string()))?;

        Ok(user.into())
    }

    /// Look up a The1 account by card number. Returns `None` when no record
    /// is found — callers choose how to surface that to the API layer.
    pub async fn get_by_card_number(
        &self,
        card_number: &str,
    ) -> Result<Option<The1AccountResponse>, ApplicationError> {
        let user = self.the1_users.find_by_card_number(card_number).await?;
        Ok(user.map(The1AccountResponse::from))
    }

    /// Look up a The1 account by member ID. Returns `None` when not found.
    pub async fn get_by_member_id(
        &self,
        member_id: &str,
    ) -> Result<Option<The1AccountResponse>, ApplicationError> {
        let user = self.the1_users.find_by_member_id(member_id).await?;
        Ok(user.map(The1AccountResponse::from))
    }
}
