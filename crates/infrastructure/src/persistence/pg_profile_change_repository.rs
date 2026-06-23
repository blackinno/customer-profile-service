use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use domain::entities::profile_change::{
    ChangeStatus, ChangeType, CreateProfileChange, ProfileChange,
};
use domain::errors::RepositoryError;
use domain::repositories::profile_change_repository::ProfileChangeRepository;

use crate::persistence::map_sqlx_error;

#[derive(FromRow)]
struct ProfileChangeRow {
    id: Uuid,
    user_uuid: Uuid,
    change_type: String,
    identifier: Option<String>,
    old_value: Option<String>,
    new_value: Option<String>,
    status: String,
    token: Option<String>,
    token_expired_at: DateTime<Utc>,
    otp: Option<String>,
    ref_code: Option<String>,
    next_otp_request_at: DateTime<Utc>,
    otp_expired_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn change_type_to_str(ct: &ChangeType) -> &'static str {
    match ct {
        ChangeType::Telephone => "telephone",
        ChangeType::Email => "email",
    }
}

fn str_to_change_type(s: &str) -> ChangeType {
    match s {
        "email" => ChangeType::Email,
        _ => ChangeType::Telephone,
    }
}

fn change_status_to_str(cs: &ChangeStatus) -> &'static str {
    match cs {
        ChangeStatus::PendingVerifyOtp => "pending_verify_otp",
        ChangeStatus::VerifyChangeCompleted => "verify_change_completed",
        ChangeStatus::PendingChangeTopConfirmation => "pending_change_top_confirmation",
        ChangeStatus::Completed => "completed",
    }
}

fn str_to_change_status(s: &str) -> ChangeStatus {
    match s {
        "verify_change_completed" => ChangeStatus::VerifyChangeCompleted,
        "pending_change_top_confirmation" => ChangeStatus::PendingChangeTopConfirmation,
        "completed" => ChangeStatus::Completed,
        _ => ChangeStatus::PendingVerifyOtp,
    }
}

impl From<ProfileChangeRow> for ProfileChange {
    fn from(row: ProfileChangeRow) -> Self {
        Self {
            id: row.id,
            user_uuid: row.user_uuid,
            change_type: str_to_change_type(&row.change_type),
            identifier: row.identifier,
            old_value: row.old_value,
            new_value: row.new_value,
            status: str_to_change_status(&row.status),
            token: row.token,
            token_expired_at: row.token_expired_at,
            otp: row.otp,
            ref_code: row.ref_code,
            next_otp_request_at: row.next_otp_request_at,
            otp_expired_at: row.otp_expired_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// SELECT projection — casts PostgreSQL enums back to TEXT for FromRow.
const SELECT_BASE: &str = "SELECT id, user_uuid, \
     change_type::TEXT AS change_type, \
     identifier, old_value, new_value, \
     status::TEXT AS status, \
     token, token_expired_at, otp, ref_code, \
     next_otp_request_at, otp_expired_at, \
     created_at, updated_at \
     FROM profile_changes ";

const RETURNING: &str = " RETURNING id, user_uuid, \
      change_type::TEXT AS change_type, \
      identifier, old_value, new_value, \
      status::TEXT AS status, \
      token, token_expired_at, otp, ref_code, \
      next_otp_request_at, otp_expired_at, \
      created_at, updated_at";

pub struct PgProfileChangeRepository {
    pool: PgPool,
}

impl PgProfileChangeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProfileChangeRepository for PgProfileChangeRepository {
    async fn create(&self, data: CreateProfileChange) -> Result<ProfileChange, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO profile_changes \
             (id, user_uuid, change_type, identifier, old_value, new_value, status, \
              otp, ref_code, token_expired_at, next_otp_request_at, otp_expired_at) \
             VALUES (",
        );
        qb.push_bind(Uuid::new_v4())
            .push(", ")
            .push_bind(data.user_uuid)
            .push(", ")
            .push_bind(change_type_to_str(&data.change_type))
            .push("::change_type_enum, ")
            .push_bind(data.identifier)
            .push(", ")
            .push_bind(data.old_value)
            .push(", ")
            .push_bind(data.new_value)
            .push(", ")
            .push_bind(change_status_to_str(&data.status))
            .push("::change_type_status_enum, ")
            .push_bind(data.otp)
            .push(", ")
            .push_bind(data.ref_code)
            .push(", ")
            .push_bind(data.token_expired_at)
            .push(", ")
            .push_bind(data.next_otp_request_at)
            .push(", ")
            .push_bind(data.otp_expired_at)
            .push(")")
            .push(RETURNING);

        let row: ProfileChangeRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ProfileChange>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE id = ").push_bind(id);

        let row: Option<ProfileChangeRow> = qb
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(Into::into))
    }

    async fn find_active_by_user_and_type(
        &self,
        user_uuid: Uuid,
        change_type: ChangeType,
    ) -> Result<Option<ProfileChange>, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new(SELECT_BASE);
        qb.push("WHERE user_uuid = ")
            .push_bind(user_uuid)
            .push(" AND change_type = ")
            .push_bind(change_type_to_str(&change_type))
            .push("::change_type_enum")
            .push(
                " AND status NOT IN (\
               'completed'::change_type_status_enum, \
               'pending_change_top_confirmation'::change_type_status_enum\
               )",
            )
            .push(" ORDER BY created_at DESC LIMIT 1");

        let row: Option<ProfileChangeRow> = qb
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(Into::into))
    }

    async fn update_otp(
        &self,
        id: Uuid,
        otp: String,
        ref_code: String,
        expires: DateTime<Utc>,
        next_request: DateTime<Utc>,
    ) -> Result<ProfileChange, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE profile_changes SET otp = ");
        qb.push_bind(otp)
            .push(", ref_code = ")
            .push_bind(ref_code)
            .push(", otp_expired_at = ")
            .push_bind(expires)
            .push(", next_otp_request_at = ")
            .push_bind(next_request)
            .push(", updated_at = NOW() WHERE id = ")
            .push_bind(id)
            .push(RETURNING);

        let row: ProfileChangeRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.into())
    }

    async fn update_status_and_token(
        &self,
        id: Uuid,
        status: ChangeStatus,
        token: Option<String>,
        token_expires: Option<DateTime<Utc>>,
    ) -> Result<ProfileChange, RepositoryError> {
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE profile_changes SET status = ");
        qb.push_bind(change_status_to_str(&status))
            .push("::change_type_status_enum")
            .push(", token = ")
            .push_bind(token)
            .push(", token_expired_at = COALESCE(")
            .push_bind(token_expires)
            .push(", token_expired_at)")
            .push(", updated_at = NOW() WHERE id = ")
            .push_bind(id)
            .push(RETURNING);

        let row: ProfileChangeRow = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.into())
    }
}
