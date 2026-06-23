use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::customer::{CreateCustomer, Customer, SearchField, UpdateCustomer};
use crate::errors::RepositoryError;

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn create(&self, data: CreateCustomer) -> Result<Customer, RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, RepositoryError>;
    async fn find_by_phone(&self, phone: &str) -> Result<Option<Customer>, RepositoryError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Customer>, RepositoryError>;
    async fn search(&self, field: SearchField) -> Result<Vec<Customer>, RepositoryError>;
    async fn update(&self, id: Uuid, data: UpdateCustomer) -> Result<Customer, RepositoryError>;
    async fn soft_delete(&self, id: Uuid) -> Result<Customer, RepositoryError>;
    async fn update_profile_image(
        &self,
        user_uuid: Uuid,
        image_key: Option<String>,
    ) -> Result<(), RepositoryError>;
}
