use std::sync::Arc;

use domain::repositories::customer_repository::CustomerRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::customers::dtos::{
    CreateCustomerRequest, CustomerResponse, SearchCustomerQuery, UpdateCustomerRequest,
};
use crate::errors::ApplicationError;

pub struct CustomerUseCases {
    customers: Arc<dyn CustomerRepository>,
    config: Arc<AppConfig>,
}

impl CustomerUseCases {
    pub fn new(customers: Arc<dyn CustomerRepository>, config: Arc<AppConfig>) -> Self {
        Self { customers, config }
    }

    pub async fn create(
        &self,
        req: CreateCustomerRequest,
    ) -> Result<CustomerResponse, ApplicationError> {
        todo!("eng-customers: implement create customer use case")
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CustomerResponse, ApplicationError> {
        todo!("eng-customers: implement get_by_id use case")
    }

    pub async fn search(
        &self,
        query: SearchCustomerQuery,
    ) -> Result<Vec<CustomerResponse>, ApplicationError> {
        todo!("eng-customers: implement search use case")
    }

    pub async fn get_me(&self, user_uuid: Uuid) -> Result<CustomerResponse, ApplicationError> {
        todo!("eng-customers: implement get_me use case")
    }

    pub async fn update_me(
        &self,
        user_uuid: Uuid,
        req: UpdateCustomerRequest,
    ) -> Result<CustomerResponse, ApplicationError> {
        todo!("eng-customers: implement update_me use case")
    }

    pub async fn delete(&self, id: Uuid) -> Result<CustomerResponse, ApplicationError> {
        todo!("eng-customers: implement delete use case")
    }

    pub async fn update_profile_image(
        &self,
        user_uuid: Uuid,
        image_key: Option<String>,
    ) -> Result<(), ApplicationError> {
        todo!("eng-customers: implement update_profile_image")
    }
}
