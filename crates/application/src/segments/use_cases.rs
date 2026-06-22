use std::sync::Arc;

use domain::repositories::the1_user_repository::The1UserRepository;

use crate::errors::ApplicationError;
use crate::segments::dtos::SegmentResponse;

pub struct SegmentUseCases {
    the1_users: Arc<dyn The1UserRepository>,
}

impl SegmentUseCases {
    pub fn new(the1_users: Arc<dyn The1UserRepository>) -> Self {
        Self { the1_users }
    }

    pub async fn get_segment(
        &self,
        card_number: String,
    ) -> Result<SegmentResponse, ApplicationError> {
        todo!("eng-the1-segments: implement get_segment via The1 partner member API")
    }
}
