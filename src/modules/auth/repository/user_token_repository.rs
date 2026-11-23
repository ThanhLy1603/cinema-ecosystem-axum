use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::modules::auth::entity::UserTokenRecord;
use crate::modules::auth::enums::UserTokenType;

pub async fn invalidate_unused_user_tokens(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   user_token_type: UserTokenType,
   now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
   sqlx::query("UPDATE user_tokens SET used_at = $3 WHERE user_id = $1 AND token_type = $2 AND used_at IS NULL")
      .bind(user_id)
      .bind(user_token_type.as_str())
      .bind(now)
      .execute(executor)
      .await
      .map(|query_result| query_result.rows_affected())
}

pub async fn insert_user_token(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   user_token_type: UserTokenType,
   token_hash: &str,
   expires_at: DateTime<Utc>,
   ip_address: Option<&str>,
) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>(
      "INSERT INTO user_tokens (user_id, token_type, token_hash, expires_at, ip_address) \
       VALUES ($1, $2, $3, $4, $5) RETURNING id",
   )
   .bind(user_id)
   .bind(user_token_type.as_str())
   .bind(token_hash)
   .bind(expires_at)
   .bind(ip_address)
   .fetch_one(executor)
   .await
}

pub async fn find_user_token_by_hash_for_update(
   executor: impl PgExecutor<'_>,
   token_hash: &str,
   user_token_type: UserTokenType,
) -> Result<Option<UserTokenRecord>, sqlx::Error> {
   sqlx::query_as::<_, UserTokenRecord>(
      "SELECT id, user_id, expires_at, used_at FROM user_tokens WHERE token_hash = $1 AND token_type = $2 FOR UPDATE",
   )
   .bind(token_hash)
   .bind(user_token_type.as_str())
   .fetch_optional(executor)
   .await
}

pub async fn mark_user_token_used(
   executor: impl PgExecutor<'_>,
   user_token_id: i64,
   now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE user_tokens SET used_at = $2 WHERE id = $1")
      .bind(user_token_id)
      .bind(now)
      .execute(executor)
      .await
      .map(|_| ())
}

pub async fn delete_user_tokens_expired_before(
   executor: impl PgExecutor<'_>,
   before: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
   sqlx::query("DELETE FROM user_tokens WHERE expires_at < $1")
      .bind(before)
      .execute(executor)
      .await
      .map(|query_result| query_result.rows_affected())
}
