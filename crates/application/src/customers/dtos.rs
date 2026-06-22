use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<String>,
    pub has_consent: Option<bool>,
    pub client_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub nationality: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    pub email: Option<String>,
    pub locale: Option<String>,
    pub has_consent: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub nationality: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub locale: String,
    pub has_consent: bool,
    pub is_deleted: bool,
    pub client_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub profile_image: Option<String>,
    pub nationality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchCustomerQuery {
    pub id: Option<String>,
    pub phone: Option<String>,
    pub the1_member_id: Option<String>,
    pub the1_card_number: Option<String>,
}
