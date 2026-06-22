use std::sync::Arc;

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

    pub async fn get_identities(
        &self,
        user_uuid: Uuid,
    ) -> Result<Vec<IdentityResponse>, ApplicationError> {
        todo!("eng-identities: implement get_identities")
    }

    pub async fn get_identities_internal(
        &self,
        user_uuid: Uuid,
    ) -> Result<Vec<IdentityResponse>, ApplicationError> {
        todo!("eng-identities: implement get_identities_internal")
    }

    pub async fn create_identity(
        &self,
        user_uuid: Uuid,
        req: CreateIdentityRequest,
    ) -> Result<IdentityResponse, ApplicationError> {
        todo!("eng-identities: implement create_identity with link/restore/reassign logic")
    }

    pub async fn delete_identity(
        &self,
        user_uuid: Uuid,
        provider: String,
        external_id: String,
    ) -> Result<(), ApplicationError> {
        todo!("eng-identities: implement delete_identity")
    }

    pub async fn invoke_token(
        &self,
        user_uuid: Uuid,
        provider_name: String,
    ) -> Result<InvokeTokenResponse, ApplicationError> {
        todo!("eng-identities: implement invoke_token")
    }
}
