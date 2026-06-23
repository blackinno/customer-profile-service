use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use domain::entities::customer::{
    CreateCustomer, Customer, CustomerProfile, Gender, Locale, SearchField, UpdateCustomer,
};
use domain::errors::RepositoryError;
use domain::repositories::customer_repository::CustomerRepository;

use crate::persistence::map_sqlx_error;

/// Flat row type returned by the standard SELECT … LEFT JOIN query.
#[derive(Debug, FromRow)]
struct CustomerRow {
    id: Uuid,
    email: Option<String>,
    phone: Option<String>,
    email_verified: bool,
    phone_verified: bool,
    locale: String,
    has_consent: bool,
    is_deleted: bool,
    client_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // user_profiles columns (nullable due to LEFT JOIN)
    profile_id: Option<Uuid>,
    first_name: Option<String>,
    last_name: Option<String>,
    birthdate: Option<NaiveDate>,
    gender: Option<String>,
    profile_image: Option<String>,
    nationality: Option<String>,
    profile_created_at: Option<DateTime<Utc>>,
    profile_updated_at: Option<DateTime<Utc>>,
}

/// Standard SELECT projection for users + their profile (LEFT JOIN).
/// Callers append a WHERE clause (and optional extra JOINs before WHERE).
const SELECT_BASE: &str = r#"SELECT u.id, u.email, u.phone, u.email_verified, u.phone_verified,
       u.locale::TEXT AS locale, u.has_consent, u.is_deleted, u.client_id,
       u.created_at, u.updated_at,
       up.id AS profile_id, up.first_name, up.last_name, up.birthdate,
       up.gender::TEXT AS gender, up.profile_image, up.nationality,
       up.created_at AS profile_created_at, up.updated_at AS profile_updated_at
FROM users u
LEFT JOIN user_profiles up ON up.user_uuid = u.id
"#;

/// Same projection with an INNER JOIN on the1_users for The1 search variants.
const SELECT_BASE_THE1: &str = r#"SELECT u.id, u.email, u.phone, u.email_verified, u.phone_verified,
       u.locale::TEXT AS locale, u.has_consent, u.is_deleted, u.client_id,
       u.created_at, u.updated_at,
       up.id AS profile_id, up.first_name, up.last_name, up.birthdate,
       up.gender::TEXT AS gender, up.profile_image, up.nationality,
       up.created_at AS profile_created_at, up.updated_at AS profile_updated_at
FROM users u
LEFT JOIN user_profiles up ON up.user_uuid = u.id
JOIN the1_users t1 ON t1.user_uuid = u.id
"#;

// ---- enum ↔ string helpers ----

fn locale_to_str(locale: &Locale) -> &'static str {
    match locale {
        Locale::Th => "th",
        Locale::En => "en",
    }
}

fn str_to_locale(s: &str) -> Locale {
    match s {
        "en" => Locale::En,
        _ => Locale::Th,
    }
}

fn gender_to_str(gender: &Gender) -> &'static str {
    match gender {
        Gender::Male => "male",
        Gender::Female => "female",
        Gender::Other => "other",
        Gender::Unspecified => "unspecified",
        Gender::NotToSay => "not_to_say",
    }
}

fn str_to_gender(s: &str) -> Gender {
    match s {
        "male" => Gender::Male,
        "female" => Gender::Female,
        "other" => Gender::Other,
        "not_to_say" => Gender::NotToSay,
        _ => Gender::Unspecified,
    }
}

// ---- row → domain conversion ----

impl CustomerRow {
    fn into_customer(self) -> Customer {
        let profile = self.profile_id.map(|pid| CustomerProfile {
            id: pid,
            user_uuid: self.id,
            first_name: self.first_name,
            last_name: self.last_name,
            birthdate: self.birthdate,
            gender: self.gender.as_deref().map(str_to_gender),
            profile_image: self.profile_image,
            nationality: self.nationality,
            created_at: self.profile_created_at.unwrap_or_else(Utc::now),
            updated_at: self.profile_updated_at.unwrap_or_else(Utc::now),
        });

        Customer {
            id: self.id,
            email: self.email,
            phone: self.phone,
            email_verified: self.email_verified,
            phone_verified: self.phone_verified,
            locale: str_to_locale(&self.locale),
            has_consent: self.has_consent,
            is_deleted: self.is_deleted,
            client_id: self.client_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            profile,
        }
    }
}

// ---- repository ----

pub struct PgCustomerRepository {
    pool: PgPool,
}

impl PgCustomerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerRepository for PgCustomerRepository {
    async fn create(&self, data: CreateCustomer) -> Result<Customer, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let locale_str = data.locale.as_ref().map(locale_to_str).unwrap_or("th");

        // Insert into users
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO users \
             (id, email, phone, email_verified, phone_verified, locale, has_consent, is_deleted, client_id, created_at, updated_at) \
             VALUES (",
        );
        qb.push_bind(user_id)
            .push(", ")
            .push_bind(data.email)
            .push(", ")
            .push_bind(data.phone)
            .push(", false, false, ")
            .push_bind(locale_str)
            .push("::locale_enum, ")
            .push_bind(data.has_consent.unwrap_or(false))
            .push(", false, ")
            .push_bind(data.client_id)
            .push(", ")
            .push_bind(now)
            .push(", ")
            .push_bind(now)
            .push(")");

        qb.build().execute(&mut *tx).await.map_err(map_sqlx_error)?;

        // Insert into user_profiles (id uses DB DEFAULT gen_random_uuid())
        let mut qb2 = QueryBuilder::<Postgres>::new(
            "INSERT INTO user_profiles \
             (user_uuid, first_name, last_name, birthdate, gender, profile_image, nationality, created_at, updated_at) \
             VALUES (",
        );
        qb2.push_bind(user_id)
            .push(", ")
            .push_bind(data.first_name)
            .push(", ")
            .push_bind(data.last_name)
            .push(", ")
            .push_bind(data.birthdate)
            .push(", ");

        // gender requires an explicit cast to the PostgreSQL enum type
        match data.gender.as_ref() {
            Some(g) => {
                qb2.push_bind(gender_to_str(g)).push("::gender_enum");
            }
            None => {
                qb2.push("NULL");
            }
        }

        qb2.push(", NULL, ")
            .push_bind(data.nationality)
            .push(", ")
            .push_bind(now)
            .push(", ")
            .push_bind(now)
            .push(")");

        qb2.build()
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;

        self.find_by_id(user_id).await?.ok_or_else(|| {
            RepositoryError::Backend("created customer not found after insert".to_string())
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE u.id = ").push_bind(id);

        qb.build_query_as::<CustomerRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)
            .map(|opt| opt.map(CustomerRow::into_customer))
    }

    async fn find_by_phone(&self, phone: &str) -> Result<Option<Customer>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE u.phone = ").push_bind(phone);

        qb.build_query_as::<CustomerRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)
            .map(|opt| opt.map(CustomerRow::into_customer))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Customer>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE u.email = ").push_bind(email);

        qb.build_query_as::<CustomerRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)
            .map(|opt| opt.map(CustomerRow::into_customer))
    }

    async fn search(&self, field: SearchField) -> Result<Vec<Customer>, RepositoryError> {
        let rows: Vec<CustomerRow> = match field {
            SearchField::Id(id) => {
                let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
                qb.push("WHERE u.id = ").push_bind(id);
                qb.build_query_as::<CustomerRow>()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?
            }
            SearchField::Phone(phone) => {
                let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
                qb.push("WHERE u.phone = ").push_bind(phone);
                qb.build_query_as::<CustomerRow>()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?
            }
            SearchField::The1MemberId(member_id) => {
                let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE_THE1);
                qb.push("WHERE t1.member_id = ").push_bind(member_id);
                qb.build_query_as::<CustomerRow>()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?
            }
            SearchField::The1CardNumber(card_number) => {
                let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE_THE1);
                qb.push("WHERE t1.card_number = ").push_bind(card_number);
                qb.build_query_as::<CustomerRow>()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?
            }
        };

        Ok(rows.into_iter().map(CustomerRow::into_customer).collect())
    }

    async fn update(&self, id: Uuid, data: UpdateCustomer) -> Result<Customer, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        // users table — always bump updated_at, add optional fields
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE users SET updated_at = NOW()");

        if let Some(email) = data.email {
            qb.push(", email = ").push_bind(email);
        }
        if let Some(phone) = data.phone {
            qb.push(", phone = ").push_bind(phone);
        }
        if let Some(locale) = data.locale {
            qb.push(", locale = ")
                .push_bind(locale_to_str(&locale))
                .push("::locale_enum");
        }
        if let Some(has_consent) = data.has_consent {
            qb.push(", has_consent = ").push_bind(has_consent);
        }

        qb.push(" WHERE id = ").push_bind(id);
        qb.build().execute(&mut *tx).await.map_err(map_sqlx_error)?;

        // user_profiles table — always bump updated_at, add optional fields
        let mut qb2 = QueryBuilder::<Postgres>::new("UPDATE user_profiles SET updated_at = NOW()");

        if let Some(first_name) = data.first_name {
            qb2.push(", first_name = ").push_bind(first_name);
        }
        if let Some(last_name) = data.last_name {
            qb2.push(", last_name = ").push_bind(last_name);
        }
        if let Some(birthdate) = data.birthdate {
            qb2.push(", birthdate = ").push_bind(birthdate);
        }
        if let Some(gender) = data.gender {
            qb2.push(", gender = ")
                .push_bind(gender_to_str(&gender))
                .push("::gender_enum");
        }
        if let Some(nationality) = data.nationality {
            qb2.push(", nationality = ").push_bind(nationality);
        }

        qb2.push(" WHERE user_uuid = ").push_bind(id);
        qb2.build()
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("customer {} not found", id)))
    }

    async fn soft_delete(&self, id: Uuid) -> Result<Customer, RepositoryError> {
        // Append a stable suffix to unique columns so the values are freed for
        // re-registration while still being traceable.
        let mut qb = QueryBuilder::<Postgres>::new(
            "UPDATE users \
             SET is_deleted = true, \
                 phone = CASE WHEN phone IS NOT NULL THEN phone || '-deleted-' || id::TEXT ELSE NULL END, \
                 email = CASE WHEN email IS NOT NULL THEN email || '-deleted-' || id::TEXT ELSE NULL END, \
                 updated_at = NOW() \
             WHERE id = ",
        );
        qb.push_bind(id);
        qb.build()
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("customer {} not found", id)))
    }

    async fn update_profile_image(
        &self,
        user_uuid: Uuid,
        image_key: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE user_profiles SET profile_image = ");
        qb.push_bind(image_key)
            .push(", updated_at = NOW() WHERE user_uuid = ")
            .push_bind(user_uuid);

        qb.build()
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(())
    }
}
