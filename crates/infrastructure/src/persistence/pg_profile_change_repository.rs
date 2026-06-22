use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use domain::entities::profile_change::{
    ChangeStatus, ChangeType, CreateProfileChange, ProfileChange,
};
use domain::errors::RepositoryError;
use domain::repositories::profile_change_repository::ProfileChangeRepository;

use crate::persistence::map_sqlx_error;

/// Flat row type matching `profile_changes` columns (enums read back as TEXT).
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

// ---- enum ↔ string helpers ----

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

/// Column list for SELECT queries — casts enum types back to TEXT.
const COLS: &str = r#"
    id, user_uuid,
    change_type::TEXT AS change_type,
    identifier, old_value, new_value,
    status::TEXT AS status,
    token, token_expired_at, otp, ref_code,
    next_otp_request_at, otp_expired_at,
    created_at, updated_at
"#;

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
        let id = Uuid::new_v4();
        let ct = change_type_to_str(&data.change_type);
        let st = change_status_to_str(&data.status);

        let row = sqlx::query_as::<_, ProfileChangeRow>(&format!(
            r#"
            INSERT INTO profile_changes
                (id, user_uuid, change_type, identifier, old_value, new_value, status,
                 otp, ref_code, token_expired_at, next_otp_request_at, otp_expired_at)
            VALUES
                ($1, $2, $3::change_type_enum, $4, $5, $6, $7::change_type_status_enum,
                 $8, $9, $10, $11, $12)
            RETURNING {COLS}
            "#
        ))
        .bind(id)
        .bind(data.user_uuid)
        .bind(ct)
        .bind(data.identifier)
        .bind(data.old_value)
        .bind(data.new_value)
        .bind(st)
        .bind(data.otp)
        .bind(data.ref_code)
        .bind(data.token_expired_at)
        .bind(data.next_otp_request_at)
        .bind(data.otp_expired_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ProfileChange>, RepositoryError> {
        let row = sqlx::query_as::<_, ProfileChangeRow>(&format!(
            "SELECT {COLS} FROM profile_changes WHERE id = $1"
        ))
        .bind(id)
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
        let ct = change_type_to_str(&change_type);

        let row = sqlx::query_as::<_, ProfileChangeRow>(&format!(
            r#"
            SELECT {COLS} FROM profile_changes
            WHERE user_uuid = $1
              AND change_type = $2::change_type_enum
              AND status NOT IN (
                  'completed'::change_type_status_enum,
                  'pending_change_top_confirmation'::change_type_status_enum
              )
            ORDER BY created_at DESC
            LIMIT 1
            "#
        ))
        .bind(user_uuid)
        .bind(ct)
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
        let row = sqlx::query_as::<_, ProfileChangeRow>(&format!(
            r#"
            UPDATE profile_changes
            SET otp = $2,
                ref_code = $3,
                otp_expired_at = $4,
                next_otp_request_at = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING {COLS}
            "#
        ))
        .bind(id)
        .bind(otp)
        .bind(ref_code)
        .bind(expires)
        .bind(next_request)
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
        let st = change_status_to_str(&status);

        let row = sqlx::query_as::<_, ProfileChangeRow>(&format!(
            r#"
            UPDATE profile_changes
            SET status = $2::change_type_status_enum,
                token = $3,
                token_expired_at = COALESCE($4, token_expired_at),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {COLS}
            "#
        ))
        .bind(id)
        .bind(st)
        .bind(token)
        .bind(token_expires)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.into())
    }
}
