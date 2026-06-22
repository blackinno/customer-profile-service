use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use domain::entities::identity::{CreateIdentity, Identity};
use domain::errors::RepositoryError;
use domain::repositories::identity_repository::IdentityRepository;

use crate::persistence::map_sqlx_error;

/// Internal row type matching the `identity_providers` table schema.
/// Not exposed outside this module — the domain `Identity` entity is the
/// canonical representation returned to callers.
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

/// Column list shared across all SELECT queries — avoids drift between
/// individual query strings and the FromRow derive on IdentityRow.
const IDENTITY_COLS: &str = r#"
    id, user_uuid, provider_name, external_id,
    provider_id_token, provider_access_token, provider_refresh_token,
    is_deleted, created_at, updated_at
"#;

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
    /// Return all non-deleted identities for a user.
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Vec<Identity>, RepositoryError> {
        let sql = format!(
            "SELECT {IDENTITY_COLS} FROM identity_providers \
             WHERE user_uuid = $1 AND is_deleted = false"
        );

        let rows: Vec<IdentityRow> = sqlx::query_as(&sql)
            .bind(user_uuid)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(Identity::from).collect())
    }

    /// Return a single active (non-deleted) identity matching the provider
    /// triple (user, provider name, external ID).
    async fn find_active(
        &self,
        user_uuid: Uuid,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        let sql = format!(
            "SELECT {IDENTITY_COLS} FROM identity_providers \
             WHERE user_uuid = $1 AND provider_name = $2 AND external_id = $3 \
             AND is_deleted = false"
        );

        let row: Option<IdentityRow> = sqlx::query_as(&sql)
            .bind(user_uuid)
            .bind(provider)
            .bind(external_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(Identity::from))
    }

    /// Return a soft-deleted identity row for any user that matches the
    /// provider + external_id pair. Used to detect recyclable rows before
    /// creating a fresh one.
    async fn find_deleted(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Identity>, RepositoryError> {
        let sql = format!(
            "SELECT {IDENTITY_COLS} FROM identity_providers \
             WHERE provider_name = $1 AND external_id = $2 AND is_deleted = true"
        );

        let row: Option<IdentityRow> = sqlx::query_as(&sql)
            .bind(provider)
            .bind(external_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(Identity::from))
    }

    /// Insert a new identity row.
    async fn create(&self, data: CreateIdentity) -> Result<Identity, RepositoryError> {
        let sql = format!(
            "INSERT INTO identity_providers \
             (id, user_uuid, provider_name, external_id, \
              provider_id_token, provider_access_token, provider_refresh_token, \
              is_deleted, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, false, NOW(), NOW()) \
             RETURNING {IDENTITY_COLS}"
        );

        let row: IdentityRow = sqlx::query_as(&sql)
            .bind(Uuid::new_v4())
            .bind(data.user_uuid)
            .bind(data.provider_name)
            .bind(data.external_id)
            .bind(data.provider_id_token)
            .bind(data.provider_access_token)
            .bind(data.provider_refresh_token)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    /// Restore a previously soft-deleted identity, optionally re-assigning it
    /// to a different user and refreshing all token fields.
    async fn restore(
        &self,
        id: Uuid,
        user_uuid: Uuid,
        tokens: CreateIdentity,
    ) -> Result<Identity, RepositoryError> {
        let sql = format!(
            "UPDATE identity_providers \
             SET user_uuid            = $1, \
                 is_deleted           = false, \
                 provider_id_token    = $2, \
                 provider_access_token  = $3, \
                 provider_refresh_token = $4, \
                 updated_at           = NOW() \
             WHERE id = $5 \
             RETURNING {IDENTITY_COLS}"
        );

        let row: IdentityRow = sqlx::query_as(&sql)
            .bind(user_uuid)
            .bind(tokens.provider_id_token)
            .bind(tokens.provider_access_token)
            .bind(tokens.provider_refresh_token)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    /// Mark an identity as deleted without removing it from the database.
    /// Scoped to `user_uuid` to prevent cross-user deletions.
    async fn soft_delete(&self, id: Uuid, user_uuid: Uuid) -> Result<Identity, RepositoryError> {
        let sql = format!(
            "UPDATE identity_providers \
             SET is_deleted = true, updated_at = NOW() \
             WHERE id = $1 AND user_uuid = $2 \
             RETURNING {IDENTITY_COLS}"
        );

        let row: IdentityRow = sqlx::query_as(&sql)
            .bind(id)
            .bind(user_uuid)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    /// Replace the provider access and refresh tokens for an identity.
    async fn update_tokens(
        &self,
        id: Uuid,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<Identity, RepositoryError> {
        let sql = format!(
            "UPDATE identity_providers \
             SET provider_access_token  = $1, \
                 provider_refresh_token = $2, \
                 updated_at             = NOW() \
             WHERE id = $3 \
             RETURNING {IDENTITY_COLS}"
        );

        let row: IdentityRow = sqlx::query_as(&sql)
            .bind(access_token)
            .bind(refresh_token)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(Identity::from(row))
    }

    /// Append an audit row to `identity_provider_transactions`.
    async fn log_transaction(
        &self,
        user_uuid: Uuid,
        action: &str,
        provider: &str,
        external_id: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO identity_provider_transactions \
             (id, user_uuid, action_type, provider_name, external_id) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(user_uuid)
        .bind(action)
        .bind(provider)
        .bind(external_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}
