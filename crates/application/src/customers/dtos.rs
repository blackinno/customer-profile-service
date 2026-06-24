use domain::entities::customer::{Customer, Gender, Locale};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct SearchCustomerQuery {
    pub id: Option<String>,
    pub phone: Option<String>,
    pub the1_member_id: Option<String>,
    pub the1_card_number: Option<String>,
}

impl From<Customer> for CustomerResponse {
    fn from(customer: Customer) -> Self {
        let (first_name, last_name, birthdate, gender, profile_image, nationality) =
            match customer.profile {
                Some(profile) => (
                    profile.first_name,
                    profile.last_name,
                    profile.birthdate.map(|d| d.to_string()),
                    profile.gender.as_ref().map(gender_to_str),
                    profile.profile_image,
                    profile.nationality,
                ),
                None => (None, None, None, None, None, None),
            };

        CustomerResponse {
            id: customer.id.to_string(),
            email: customer.email,
            phone: customer.phone,
            email_verified: customer.email_verified,
            phone_verified: customer.phone_verified,
            locale: locale_to_str(&customer.locale).to_string(),
            has_consent: customer.has_consent,
            is_deleted: customer.is_deleted,
            client_id: customer.client_id,
            created_at: customer.created_at.to_rfc3339(),
            updated_at: customer.updated_at.to_rfc3339(),
            first_name,
            last_name,
            birthdate,
            gender,
            profile_image,
            nationality,
        }
    }
}

fn locale_to_str(locale: &Locale) -> &'static str {
    match locale {
        Locale::Th => "th",
        Locale::En => "en",
    }
}

fn gender_to_str(gender: &Gender) -> String {
    match gender {
        Gender::Male => "male".to_string(),
        Gender::Female => "female".to_string(),
        Gender::Other => "other".to_string(),
        Gender::Unspecified => "unspecified".to_string(),
        Gender::NotToSay => "not_to_say".to_string(),
    }
}
