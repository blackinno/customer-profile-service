use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Locale {
    #[default]
    Th,
    En,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Gender {
    Male,
    Female,
    Other,
    Unspecified,
    NotToSay,
}

#[derive(Debug, Clone)]
pub struct Customer {
    pub id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub locale: Locale,
    pub has_consent: bool,
    pub is_deleted: bool,
    pub client_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile: Option<CustomerProfile>,
}

#[derive(Debug, Clone)]
pub struct CustomerProfile {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub profile_image: Option<String>,
    pub nationality: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCustomer {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<Locale>,
    pub has_consent: Option<bool>,
    pub client_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub nationality: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateCustomer {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<Locale>,
    pub has_consent: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub nationality: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SearchField {
    Id(Uuid),
    Phone(String),
    The1MemberId(String),
    The1CardNumber(String),
}
