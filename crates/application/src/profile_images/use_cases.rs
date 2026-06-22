use std::sync::Arc;

use domain::repositories::customer_repository::CustomerRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApplicationError;
use crate::profile_images::dtos::ProfileImageResponse;

pub struct ProfileImageUseCases {
    customers: Arc<dyn CustomerRepository>,
    config: Arc<AppConfig>,
}

impl ProfileImageUseCases {
    pub fn new(customers: Arc<dyn CustomerRepository>, config: Arc<AppConfig>) -> Self {
        Self { customers, config }
    }

    pub async fn upload(
        &self,
        user_uuid: Uuid,
        data: Vec<u8>,
        content_type: String,
        file_size: usize,
    ) -> Result<ProfileImageResponse, ApplicationError> {
        todo!("eng-profile-images: implement upload with type/size validation, S3, CloudFront")
    }

    pub async fn get_image(
        &self,
        user_uuid: Uuid,
    ) -> Result<ProfileImageResponse, ApplicationError> {
        todo!("eng-profile-images: implement get_image with signed CloudFront URL")
    }

    pub async fn delete_image(&self, user_uuid: Uuid) -> Result<(), ApplicationError> {
        todo!("eng-profile-images: implement delete_image")
    }
}
