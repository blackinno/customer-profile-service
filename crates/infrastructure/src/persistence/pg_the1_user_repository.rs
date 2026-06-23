use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use domain::entities::the1_user::{The1User, Tier, UpsertThe1User};
use domain::errors::RepositoryError;
use domain::repositories::the1_user_repository::The1UserRepository;

use crate::persistence::map_sqlx_error;

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

const THE1_SELECT: &str =
    "SELECT id, user_uuid, member_id, account_id, profile_id, \
     card_number, created_at, updated_at FROM the1_users ";

const THE1_RETURNING: &str =
    " RETURNING id, user_uuid, member_id, account_id, profile_id, \
      card_number, created_at, updated_at";

const TIER_RETURNING: &str =
    " RETURNING id, code, name, expired_date, the1_users_id, created_at, updated_at";

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

    async fn load_tiers(&self, the1_users_id: Uuid) -> Result<Vec<Tier>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT id, code, name, expired_date, the1_users_id, created_at, updated_at \
             FROM tiers WHERE the1_users_id = ",
        );
        qb.push_bind(the1_users_id);

        let rows: Vec<TierRow> = qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(Tier::from).collect())
    }
}

#[async_trait]
impl The1UserRepository for PgThe1UserRepository {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Option<The1User>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(THE1_SELECT);
        qb.push("WHERE user_uuid = ").push_bind(user_uuid).push(" LIMIT 1");

        let row: Option<The1UserRow> = qb.build_query_as()
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

    async fn find_by_card_number(&self, card_number: &str) -> Result<Option<The1User>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(THE1_SELECT);
        qb.push("WHERE card_number = ").push_bind(card_number).push(" LIMIT 1");

        let row: Option<The1UserRow> = qb.build_query_as()
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

    async fn find_by_member_id(&self, member_id: &str) -> Result<Option<The1User>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(THE1_SELECT);
        qb.push("WHERE member_id = ").push_bind(member_id).push(" LIMIT 1");

        let row: Option<The1UserRow> = qb.build_query_as()
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

    async fn upsert(&self, user_uuid: Uuid, profile: UpsertThe1User) -> Result<The1User, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        // Lock the existing row (if any) to guard against concurrent INSERT races.
        let mut select_qb = QueryBuilder::<Postgres>::new(THE1_SELECT);
        select_qb.push("WHERE user_uuid = ").push_bind(user_uuid).push(" LIMIT 1 FOR UPDATE");

        let existing: Option<The1UserRow> = select_qb.build_query_as()
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

        let the1_users_id: Uuid;
        let the1_user_row: The1UserRow;

        if let Some(row) = existing {
            the1_users_id = row.id;

            let mut qb = QueryBuilder::<Postgres>::new("UPDATE the1_users SET member_id = ");
            qb.push_bind(&profile.member_id)
              .push(", account_id = ").push_bind(&profile.account_id)
              .push(", profile_id = ").push_bind(&profile.profile_id)
              .push(", card_number = ").push_bind(&profile.card_number)
              .push(", updated_at = NOW() WHERE id = ").push_bind(the1_users_id)
              .push(THE1_RETURNING);

            the1_user_row = qb.build_query_as()
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        } else {
            let mut qb = QueryBuilder::<Postgres>::new(
                "INSERT INTO the1_users \
                 (id, user_uuid, member_id, account_id, profile_id, \
                  card_number, created_at, updated_at) VALUES (",
            );
            qb.push_bind(Uuid::new_v4()).push(", ")
              .push_bind(user_uuid).push(", ")
              .push_bind(&profile.member_id).push(", ")
              .push_bind(&profile.account_id).push(", ")
              .push_bind(&profile.profile_id).push(", ")
              .push_bind(&profile.card_number)
              .push(", NOW(), NOW())")
              .push(THE1_RETURNING);

            the1_user_row = qb.build_query_as()
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            the1_users_id = the1_user_row.id;
        }

        // Replace all tiers: delete existing, then insert the new set.
        let mut del_qb = QueryBuilder::<Postgres>::new("DELETE FROM tiers WHERE the1_users_id = ");
        del_qb.push_bind(the1_users_id);
        del_qb.build().execute(&mut *tx).await.map_err(map_sqlx_error)?;

        let mut tiers: Vec<Tier> = Vec::with_capacity(profile.tiers.len());
        for tier in &profile.tiers {
            let mut qb = QueryBuilder::<Postgres>::new(
                "INSERT INTO tiers \
                 (id, code, name, expired_date, the1_users_id, created_at, updated_at) \
                 VALUES (",
            );
            qb.push_bind(Uuid::new_v4()).push(", ")
              .push_bind(&tier.code).push(", ")
              .push_bind(&tier.name).push(", ")
              .push_bind(tier.expired_date).push(", ")
              .push_bind(the1_users_id)
              .push(", NOW(), NOW())")
              .push(TIER_RETURNING);

            let tier_row: TierRow = qb.build_query_as()
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
