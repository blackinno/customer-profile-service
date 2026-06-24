use std::sync::Arc;

use chrono::NaiveDate;
use domain::entities::customer::{CreateCustomer, Gender, Locale, SearchField, UpdateCustomer};
use domain::repositories::customer_repository::CustomerRepository;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::customers::dtos::{CreateCustomerRequest, CustomerResponse, SearchCustomerQuery, UpdateCustomerRequest};
use crate::errors::ApplicationError;
use crate::events::{NoopPublisher, ProfileChangedPayload, Publisher};

pub struct CustomerUseCases {
    customers: Arc<dyn CustomerRepository>,
    config: Arc<AppConfig>,
    publisher: Arc<dyn Publisher>,
}

impl CustomerUseCases {
    pub fn new(customers: Arc<dyn CustomerRepository>, config: Arc<AppConfig>) -> Self {
        Self {
            customers,
            config,
            publisher: Arc::new(NoopPublisher),
        }
    }

    /// Override publisher for production (called by infrastructure factory).
    pub fn with_publisher(mut self, publisher: Arc<dyn Publisher>) -> Self {
        self.publisher = publisher;
        self
    }

    pub async fn create(
        &self,
        req: CreateCustomerRequest,
    ) -> Result<CustomerResponse, ApplicationError> {
        // Normalise phone: strip leading "0" and prepend the configured format (e.g. "+66").
        let phone = req
            .phone
            .map(|p| normalize_phone(&p, &self.config.phone_number_format));

        // Uniqueness guards – checked before touching the repo so the error messages
        // are clean business-rule violations rather than DB constraint errors.
        if let Some(ref email) = req.email
            && self.customers.find_by_email(email).await?.is_some()
        {
            return Err(ApplicationError::BadRequest(
                "email already in use".to_string(),
            ));
        }
        if let Some(ref p) = phone
            && self.customers.find_by_phone(p).await?.is_some()
        {
            return Err(ApplicationError::BadRequest(
                "phone already in use".to_string(),
            ));
        }

        let customer = self
            .customers
            .create(CreateCustomer {
                email: req.email,
                phone,
                locale: req.locale.as_deref().map(parse_locale),
                has_consent: req.has_consent,
                client_id: req.client_id,
                first_name: req.first_name,
                last_name: req.last_name,
                birthdate: req.birthdate.as_deref().and_then(parse_date),
                gender: req.gender.as_deref().and_then(parse_gender),
                nationality: req.nationality,
            })
            .await?;

        Ok(customer.into())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CustomerResponse, ApplicationError> {
        let customer = self
            .customers
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("customer {} not found", id)))?;

        if customer.is_deleted {
            return Err(ApplicationError::NotFound(format!(
                "customer {} not found",
                id
            )));
        }

        Ok(customer.into())
    }

    pub async fn search(
        &self,
        query: SearchCustomerQuery,
    ) -> Result<Vec<CustomerResponse>, ApplicationError> {
        let field = build_search_field(&query)?;
        let customers = self.customers.search(field).await?;
        Ok(customers.into_iter().map(CustomerResponse::from).collect())
    }

    pub async fn get_me(&self, user_uuid: Uuid) -> Result<CustomerResponse, ApplicationError> {
        let customer = self.customers.find_by_id(user_uuid).await?.ok_or_else(|| {
            ApplicationError::NotFound(format!("customer {} not found", user_uuid))
        })?;

        if customer.is_deleted {
            return Err(ApplicationError::NotFound(format!(
                "customer {} not found",
                user_uuid
            )));
        }

        Ok(customer.into())
    }

    pub async fn update_me(
        &self,
        user_uuid: Uuid,
        req: UpdateCustomerRequest,
    ) -> Result<CustomerResponse, ApplicationError> {
        // If the caller is changing their email, ensure the new address is free
        // (allow re-submitting the same email they already own).
        if let Some(ref email) = req.email
            && let Some(existing) = self.customers.find_by_email(email).await?
            && existing.id != user_uuid
        {
            return Err(ApplicationError::BadRequest(
                "email already in use".to_string(),
            ));
        }

        let customer = self
            .customers
            .update(
                user_uuid,
                UpdateCustomer {
                    email: req.email,
                    // Phone changes go through the profile_change flow, not here.
                    phone: None,
                    locale: req.locale.as_deref().map(parse_locale),
                    has_consent: req.has_consent,
                    first_name: req.first_name,
                    last_name: req.last_name,
                    birthdate: req.birthdate.as_deref().and_then(parse_date),
                    gender: req.gender.as_deref().and_then(parse_gender),
                    nationality: req.nationality,
                },
            )
            .await?;

        // Publish profile-changed event (best-effort: log on failure, don't abort)
        let payload = serde_json::to_string(&ProfileChangedPayload {
            user_uuid: user_uuid.to_string(),
        })
        .unwrap_or_default();
        if let Err(e) = self
            .publisher
            .publish(&self.config.sns_user_profile_changed, &payload)
            .await
        {
            tracing::warn!("sns publish failed (profile changed): {e}");
        }

        Ok(customer.into())
    }

    pub async fn delete(&self, id: Uuid) -> Result<CustomerResponse, ApplicationError> {
        let customer = self.customers.soft_delete(id).await?;
        Ok(customer.into())
    }

    pub async fn update_profile_image(
        &self,
        user_uuid: Uuid,
        image_key: Option<String>,
    ) -> Result<(), ApplicationError> {
        self.customers
            .update_profile_image(user_uuid, image_key)
            .await?;
        Ok(())
    }
}

// ---- private helpers ----

fn normalize_phone(phone: &str, format: &str) -> String {
    if let Some(stripped) = phone.strip_prefix('0') {
        format!("{}{}", format, stripped)
    } else {
        phone.to_string()
    }
}

fn parse_locale(s: &str) -> Locale {
    match s.to_lowercase().as_str() {
        "en" => Locale::En,
        _ => Locale::Th,
    }
}

fn parse_gender(s: &str) -> Option<Gender> {
    match s.to_lowercase().as_str() {
        "male" => Some(Gender::Male),
        "female" => Some(Gender::Female),
        "other" => Some(Gender::Other),
        "not_to_say" => Some(Gender::NotToSay),
        "unspecified" => Some(Gender::Unspecified),
        _ => None,
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Maps `SearchCustomerQuery` to a `SearchField`, returning `BadRequest` when no
/// search parameter is supplied.
fn build_search_field(query: &SearchCustomerQuery) -> Result<SearchField, ApplicationError> {
    if let Some(ref id) = query.id {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| ApplicationError::BadRequest("invalid id format".to_string()))?;
        return Ok(SearchField::Id(uuid));
    }
    if let Some(ref phone) = query.phone {
        return Ok(SearchField::Phone(phone.clone()));
    }
    if let Some(ref member_id) = query.the1_member_id {
        return Ok(SearchField::The1MemberId(member_id.clone()));
    }
    if let Some(ref card_number) = query.the1_card_number {
        return Ok(SearchField::The1CardNumber(card_number.clone()));
    }
    Err(ApplicationError::BadRequest(
        "at least one search parameter is required (id, phone, the1_member_id, or the1_card_number)".to_string(),
    ))
}

