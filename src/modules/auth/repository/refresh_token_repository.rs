use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::modules::auth::entity::RefreshTokenRecord;

pub struct NewRefreshToken<'a> {
   pub user_id: i64,
   pub token_hash: &'a str,
   pub family_id: Uuid,
   pub device_info: Option<&'a str>,
   pub ip_address: Option<&'a str>,
   pub expires_at: DateTime<Utc>,
}

pub async fn insert_refresh_token(
   executor: impl PgExecutor<'_>,
   new_refresh_token: &NewRefreshToken<'_>,
) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>(
      "INSERT INTO refresh_tokens (user_id, token_hash, family_id, device_info, ip_address, expires_at) \
       VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
   )
   .bind(new_refresh_token.user_id)
   .bind(new_refresh_token.token_hash)
   .bind(new_refresh_token.family_id)
   .bind(new_refresh_token.device_info)
   .bind(new_refresh_token.ip_address)
   .bind(new_refresh_token.expires_at)
   .fetch_one(executor)
   .await
}

const REFRESH_TOKEN_COLUMNS_SQL: &str =
   "SELECT id, user_id, family_id, expires_at, revoked, revoked_at, replaced_by_id \
                                         FROM refresh_tokens WHERE token_hash = $1";

pub async fn find_refresh_token_by_hash(
   executor: impl PgExecutor<'_>,
   token_hash: &str,
) -> Result<Option<RefreshTokenRecord>, sqlx::Error> {
   sqlx::query_as::<_, RefreshTokenRecord>(REFRESH_TOKEN_COLUMNS_SQL).bind(token_hash).fetch_optional(executor).await
}

pub async fn find_refresh_token_by_hash_for_update(
   executor: impl PgExecutor<'_>,
   token_hash: &str,
) -> Result<Option<RefreshTokenRecord>, sqlx::Error> {
   sqlx::query_as::<_, RefreshTokenRecord>(
      "SELECT id, user_id, family_id, expires_at, revoked, revoked_at, replaced_by_id \
       FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE",
   )
   .bind(token_hash)
   .fetch_optional(executor)
   .await
}

pub async fn revoke_refresh_token_replaced_by(
   executor: impl PgExecutor<'_>,
   refresh_token_id: i64,
   replaced_by_id: i64,
   now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query(
      "UPDATE refresh_tokens SET revoked = TRUE, revoked_at = $3, replaced_by_id = $2 WHERE id = $1 AND revoked = FALSE",
   )
   .bind(refresh_token_id)
   .bind(replaced_by_id)
   .bind(now)
   .execute(executor)
   .await
   .map(|_| ())
}

pub async fn revoke_family(
   executor: impl PgExecutor<'_>,
   family_id: Uuid,
   now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
   sqlx::query("UPDATE refresh_tokens SET revoked = TRUE, revoked_at = $2 WHERE family_id = $1 AND revoked = FALSE")
      .bind(family_id)
      .bind(now)
      .execute(executor)
      .await
      .map(|query_result| query_result.rows_affected())
}

pub async fn revoke_all_by_user_id(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
   sqlx::query("UPDATE refresh_tokens SET revoked = TRUE, revoked_at = $2 WHERE user_id = $1 AND revoked = FALSE")
      .bind(user_id)
      .bind(now)
      .execute(executor)
      .await
      .map(|query_result| query_result.rows_affected())
}

pub async fn delete_refresh_tokens_expired_before(
   executor: impl PgExecutor<'_>,
   before: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
   sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < $1")
      .bind(before)
      .execute(executor)
      .await
      .map(|query_result| query_result.rows_affected())
}
