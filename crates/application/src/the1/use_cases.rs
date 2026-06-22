use std::sync::Arc;

use domain::entities::the1_user::The1User;
use domain::repositories::the1_user_repository::The1UserRepository;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::the1::dtos::{The1AccountResponse, TierResponse};

pub struct The1UseCases {
    the1_users: Arc<dyn The1UserRepository>,
}

impl The1UseCases {
    pub fn new(the1_users: Arc<dyn The1UserRepository>) -> Self {
        Self { the1_users }
    }

    /// Convert a domain `The1User` to the API-facing DTO.
    fn to_response(user: The1User) -> The1AccountResponse {
        The1AccountResponse {
            id: user.id.to_string(),
            user_uuid: user.user_uuid,
            member_id: user.member_id,
            account_id: user.account_id,
            profile_id: user.profile_id,
            card_number: user.card_number,
            tiers: user
                .tiers
                .into_iter()
                .map(|t| TierResponse {
                    id: t.id.to_string(),
                    code: t.code,
                    name: t.name,
                    expired_date: t.expired_date,
                })
                .collect(),
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
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

        Ok(Self::to_response(user))
    }

    /// Look up a The1 account by card number. Returns `None` when no record
    /// is found — callers choose how to surface that to the API layer.
    pub async fn get_by_card_number(
        &self,
        card_number: &str,
    ) -> Result<Option<The1AccountResponse>, ApplicationError> {
        let user = self.the1_users.find_by_card_number(card_number).await?;
        Ok(user.map(Self::to_response))
    }

    /// Look up a The1 account by member ID. Returns `None` when not found.
    pub async fn get_by_member_id(
        &self,
        member_id: &str,
    ) -> Result<Option<The1AccountResponse>, ApplicationError> {
        let user = self.the1_users.find_by_member_id(member_id).await?;
        Ok(user.map(Self::to_response))
    }
}
