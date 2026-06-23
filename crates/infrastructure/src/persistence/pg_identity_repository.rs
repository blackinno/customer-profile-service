use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use domain::entities::identity::{CreateIdentity, Identity};
use domain::errors::RepositoryError;
use domain::repositories::identity_repository::IdentityRepository;

use crate::persistence::map_sqlx_error;

#[derive(FromRow)]
struct IdentityRow {
    id: Uuid,
    user_uuid: Uuid,
    provider_name: String,
    external_id: String,
    provider_id_token: Option<String>,
    provider_access_token: Option<String>,
    provider_refresh_token: Option<String>,
    is_deleted: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<IdentityRow> for Identity {
    fn from(row: IdentityRow) -> Self {
        Self {
            id: row.id,
            user_uuid: row.user_uuid,
            provider_name: row.provider_name,
            external_id: row.external_id,
            provider_id_token: row.provider_id_token,
            provider_access_token: row.provider_access_token,
            provider_refresh_token: row.provider_refresh_token,
            is_deleted: row.is_deleted,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

const SELECT_BASE: &str = "SELECT id, user_uuid, provider_name, external_id, \
     provider_id_token, provider_access_token, provider_refresh_token, \
     is_deleted, created_at, updated_at \
     FROM identity_providers ";

const RETURNING: &str = " RETURNING id, user_uuid, provider_name, external_id, \
      provider_id_token, provider_access_token, provider_refresh_token, \
      is_deleted, created_at, updated_at";

pub struct PgIdentityRepository {
    pool: PgPool,
}

impl PgIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityRepository for PgIdentityRepository {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Vec<Identity>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE user_uuid = ")
            .push_bind(user_uuid)
            .push(" AND is_deleted = false");

        let rows: Vec<IdentityRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(Identity::from).collect())
    }

    async fn find_active(
        &self,
        user_uuid: Uuid,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE user_uuid = ")
            .push_bind(user_uuid)
            .push(" AND provider_name = ")
            .push_bind(provider)
            .push(" AND external_id = ")
            .push_bind(external_id)
            .push(" AND is_deleted = false");

        let row: Option<IdentityRow> = qb
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(Identity::from))
    }

    async fn find_deleted(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE provider_name = ")
            .push_bind(provider)
            .push(" AND external_id = ")
            .push_bind(external_id)
            .push(" AND is_deleted = true");

        let row: Option<IdentityRow> = qb
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(Identity::from))
    }

    async fn create(&self, data: CreateIdentity) -> Result<Identity, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO identity_providers \
             (id, user_uuid, provider_name, external_id, \
              provider_id_token, provider_access_token, provider_refresh_token, \
              is_deleted, created_at, updated_at) VALUES (",
        );
        qb.push_bind(Uuid::new_v4())
            .push(", ")
            .push_bind(data.user_uuid)
            .push(", ")
            .push_bind(data.provider_name)
            .push(", ")
            .push_bind(data.external_id)
            .push(", ")
            .push_bind(data.provider_id_token)
            .push(", ")
            .push_bind(data.provider_access_token)
            .push(", ")
            .push_bind(data.provider_refresh_token)
            .push(", false, NOW(), NOW())")
            .push(RETURNING);

        let row: IdentityRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    async fn restore(
        &self,
        id: Uuid,
        user_uuid: Uuid,
        tokens: CreateIdentity,
    ) -> Result<Identity, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "UPDATE identity_providers SET \
             user_uuid = ",
        );
        qb.push_bind(user_uuid)
            .push(", is_deleted = false, provider_id_token = ")
            .push_bind(tokens.provider_id_token)
            .push(", provider_access_token = ")
            .push_bind(tokens.provider_access_token)
            .push(", provider_refresh_token = ")
            .push_bind(tokens.provider_refresh_token)
            .push(", updated_at = NOW() WHERE id = ")
            .push_bind(id)
            .push(RETURNING);

        let row: IdentityRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    async fn soft_delete(&self, id: Uuid, user_uuid: Uuid) -> Result<Identity, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "UPDATE identity_providers SET is_deleted = true, updated_at = NOW() WHERE id = ",
        );
        qb.push_bind(id)
            .push(" AND user_uuid = ")
            .push_bind(user_uuid)
            .push(RETURNING);

        let row: IdentityRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    async fn update_tokens(
        &self,
        id: Uuid,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<Identity, RepositoryError> {
        let mut qb =
            QueryBuilder::<Postgres>::new("UPDATE identity_providers SET provider_access_token = ");
        qb.push_bind(access_token)
            .push(", provider_refresh_token = ")
            .push_bind(refresh_token)
            .push(", updated_at = NOW() WHERE id = ")
            .push_bind(id)
            .push(RETURNING);

        let row: IdentityRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    async fn log_transaction(
        &self,
        user_uuid: Uuid,
        action: &str,
        provider: &str,
        external_id: &str,
    ) -> Result<(), RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO identity_provider_transactions \
             (id, user_uuid, action_type, provider_name, external_id) VALUES (",
        );
        qb.push_bind(Uuid::new_v4())
            .push(", ")
            .push_bind(user_uuid)
            .push(", ")
            .push_bind(action)
            .push(", ")
            .push_bind(provider)
            .push(", ")
            .push_bind(external_id)
            .push(")");

        qb.build()
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(())
    }
}
