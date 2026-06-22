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

    pub async fn get_the1_account(
        &self,
        user_uuid: Uuid,
    ) -> Result<The1AccountResponse, ApplicationError> {
        todo!("eng-the1-segments: implement get_the1_account")
    }

    pub async fn get_by_card_number(
        &self,
        card_number: &str,
    ) -> Result<Option<The1AccountResponse>, ApplicationError> {
        todo!("eng-the1-segments: implement get_by_card_number")
    }

    pub async fn get_by_member_id(
        &self,
        member_id: &str,
    ) -> Result<Option<The1AccountResponse>, ApplicationError> {
        todo!("eng-the1-segments: implement get_by_member_id")
    }
}
