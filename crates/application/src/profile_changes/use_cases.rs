use std::sync::Arc;

use domain::repositories::customer_repository::CustomerRepository;
use domain::repositories::profile_change_repository::ProfileChangeRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApplicationError;
use crate::profile_changes::dtos::{
    CreateProfileChangeRequest, ProfileChangeResponse, UpdateProfileChangeRequest,
    VerifyProfileChangeRequest,
};

pub struct ProfileChangeUseCases {
    profile_changes: Arc<dyn ProfileChangeRepository>,
    customers: Arc<dyn CustomerRepository>,
    config: Arc<AppConfig>,
}

impl ProfileChangeUseCases {
    pub fn new(
        profile_changes: Arc<dyn ProfileChangeRepository>,
        customers: Arc<dyn CustomerRepository>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self { profile_changes, customers, config }
    }

    pub async fn create_profile_change(
        &self,
        user_uuid: Uuid,
        req: CreateProfileChangeRequest,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        todo!("eng-profile-changes: implement create_profile_change")
    }

    pub async fn update_profile_change(
        &self,
        user_uuid: Uuid,
        profile_id: Uuid,
        req: UpdateProfileChangeRequest,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        todo!("eng-profile-changes: implement update_profile_change")
    }

    pub async fn verify_profile_change(
        &self,
        user_uuid: Uuid,
        profile_id: Uuid,
        req: VerifyProfileChangeRequest,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        todo!("eng-profile-changes: implement verify_profile_change")
    }

    pub async fn confirm_profile_change(
        &self,
        user_uuid: Uuid,
        profile_id: Uuid,
    ) -> Result<ProfileChangeResponse, ApplicationError> {
        todo!("eng-profile-changes: implement confirm_profile_change")
    }
}
