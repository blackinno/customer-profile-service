use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use domain::entities::the1_user::{The1User, Tier, UpsertThe1User};
use domain::errors::RepositoryError;
use domain::repositories::the1_user_repository::The1UserRepository;

use crate::persistence::map_sqlx_error;

/// Internal row type matching the `the1_users` table schema.
#[derive(FromRow)]
struct The1UserRow {
    id: Uuid,
    user_uuid: Uuid,
    member_id: String,
    account_id: String,
    profile_id: String,
    card_number: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Internal row type matching the `tiers` table schema.
#[derive(FromRow)]
struct TierRow {
    id: Uuid,
    code: String,
    name: Option<String>,
    expired_date: Option<DateTime<Utc>>,
    the1_users_id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const THE1_USER_COLS: &str =
    "id, user_uuid, member_id, account_id, profile_id, card_number, created_at, updated_at";

const TIER_COLS: &str =
    "id, code, name, expired_date, the1_users_id, created_at, updated_at";

impl From<The1UserRow> for The1User {
    fn from(row: The1UserRow) -> Self {
        Self {
            id: row.id,
            user_uuid: row.user_uuid,
            member_id: row.member_id,
            account_id: row.account_id,
            profile_id: row.profile_id,
            card_number: row.card_number,
            tiers: vec![],
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<TierRow> for Tier {
    fn from(row: TierRow) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            expired_date: row.expired_date,
        }
    }
}

pub struct PgThe1UserRepository {
    pool: PgPool,
}

impl PgThe1UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load all `tiers` rows for a given `the1_users` parent ID using the
    /// shared connection pool (not a transaction).
    async fn load_tiers(&self, the1_users_id: Uuid) -> Result<Vec<Tier>, RepositoryError> {
        let sql = format!("SELECT {TIER_COLS} FROM tiers WHERE the1_users_id = $1");
        let rows: Vec<TierRow> = sqlx::query_as(&sql)
            .bind(the1_users_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Tier::from).collect())
    }
}

#[async_trait]
impl The1UserRepository for PgThe1UserRepository {
    async fn find_by_user(
        &self,
        user_uuid: Uuid,
    ) -> Result<Option<The1User>, RepositoryError> {
        let sql = format!(
            "SELECT {THE1_USER_COLS} FROM the1_users WHERE user_uuid = $1 LIMIT 1"
        );
        let row: Option<The1UserRow> = sqlx::query_as(&sql)
            .bind(user_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let the1_users_id = r.id;
                let mut user = The1User::from(r);
                user.tiers = self.load_tiers(the1_users_id).await?;
                Ok(Some(user))
            }
        }
    }

    async fn find_by_card_number(
        &self,
        card_number: &str,
    ) -> Result<Option<The1User>, RepositoryError> {
        let sql = format!(
            "SELECT {THE1_USER_COLS} FROM the1_users WHERE card_number = $1 LIMIT 1"
        );
        let row: Option<The1UserRow> = sqlx::query_as(&sql)
            .bind(card_number)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let the1_users_id = r.id;
                let mut user = The1User::from(r);
                user.tiers = self.load_tiers(the1_users_id).await?;
                Ok(Some(user))
            }
        }
    }

    async fn find_by_member_id(
        &self,
        member_id: &str,
    ) -> Result<Option<The1User>, RepositoryError> {
        let sql = format!(
            "SELECT {THE1_USER_COLS} FROM the1_users WHERE member_id = $1 LIMIT 1"
        );
        let row: Option<The1UserRow> = sqlx::query_as(&sql)
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let the1_users_id = r.id;
                let mut user = The1User::from(r);
                user.tiers = self.load_tiers(the1_users_id).await?;
                Ok(Some(user))
            }
        }
    }

    /// Create or update the The1 membership record for a platform user, then
    /// replace all tier rows for that record in a single transaction.
    ///
    /// `the1_users` has no UNIQUE constraint on `user_uuid`, so this method
    /// uses a `SELECT … FOR UPDATE` inside the transaction to guard against
    /// concurrent inserts for the same user before deciding to INSERT or UPDATE.
    async fn upsert(
        &self,
        user_uuid: Uuid,
        profile: UpsertThe1User,
    ) -> Result<The1User, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        // Lock the existing row (if any) to prevent concurrent INSERT races.
        let select_sql = format!(
            "SELECT {THE1_USER_COLS} FROM the1_users \
             WHERE user_uuid = $1 LIMIT 1 FOR UPDATE"
        );
        let existing: Option<The1UserRow> = sqlx::query_as(&select_sql)
            .bind(user_uuid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

        let the1_users_id: Uuid;
        let the1_user_row: The1UserRow;

        if let Some(row) = existing {
            // UPDATE the existing record in-place.
            the1_users_id = row.id;
            let update_sql = format!(
                "UPDATE the1_users \
                 SET member_id = $1, \
                     account_id = $2, \
                     profile_id = $3, \
                     card_number = $4, \
                     updated_at = NOW() \
                 WHERE id = $5 \
                 RETURNING {THE1_USER_COLS}"
            );
            the1_user_row = sqlx::query_as(&update_sql)
                .bind(&profile.member_id)
                .bind(&profile.account_id)
                .bind(&profile.profile_id)
                .bind(&profile.card_number)
                .bind(the1_users_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        } else {
            // INSERT a fresh record.
            let new_id = Uuid::new_v4();
            let insert_sql = format!(
                "INSERT INTO the1_users \
                 (id, user_uuid, member_id, account_id, profile_id, \
                  card_number, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW()) \
                 RETURNING {THE1_USER_COLS}"
            );
            the1_user_row = sqlx::query_as(&insert_sql)
                .bind(new_id)
                .bind(user_uuid)
                .bind(&profile.member_id)
                .bind(&profile.account_id)
                .bind(&profile.profile_id)
                .bind(&profile.card_number)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            the1_users_id = the1_user_row.id;
        }

        // Replace all tiers: delete existing, then insert the new set.
        sqlx::query("DELETE FROM tiers WHERE the1_users_id = $1")
            .bind(the1_users_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

        let mut tiers: Vec<Tier> = Vec::with_capacity(profile.tiers.len());
        for tier in &profile.tiers {
            let tier_sql = format!(
                "INSERT INTO tiers \
                 (id, code, name, expired_date, the1_users_id, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) \
                 RETURNING {TIER_COLS}"
            );
            let tier_row: TierRow = sqlx::query_as(&tier_sql)
                .bind(Uuid::new_v4())
                .bind(&tier.code)
                .bind(&tier.name)
                .bind(tier.expired_date)
                .bind(the1_users_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            tiers.push(Tier::from(tier_row));
        }

        tx.commit().await.map_err(map_sqlx_error)?;

        let mut the1_user = The1User::from(the1_user_row);
        the1_user.tiers = tiers;
        Ok(the1_user)
    }
}
