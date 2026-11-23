use crate::modules::auth::entity::UserTokenRecord;
use crate::modules::auth::enums::UserTokenType;
use crate::modules::auth::repository::user_token_repository;
use crate::modules::auth::security::generate_random_token;
use crate::modules::auth::security::sha256_hex;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;

const IP_ADDRESS_MAX_LENGTH: usize = 45;

pub async fn issue(
   connection: &mut PgConnection,
   user_id: i64,
   user_token_type: UserTokenType,
   ttl: Duration,
   ip_address: Option<&str>,
   now: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
   user_token_repository::invalidate_unused_user_tokens(&mut *connection, user_id, user_token_type, now).await?;
   let raw_token: String = generate_random_token();
   let truncated_ip_address: Option<String> = ip_address.map(|ip| ip.chars().take(IP_ADDRESS_MAX_LENGTH).collect());
   user_token_repository::insert_user_token(
      &mut *connection,
      user_id,
      user_token_type,
      &sha256_hex(&raw_token),
      now + ttl,
      truncated_ip_address.as_deref(),
   )
   .await?;
   Ok(raw_token)
}

pub async fn find_usable_for_update(
   connection: &mut PgConnection,
   raw_token: &str,
   user_token_type: UserTokenType,
   now: DateTime<Utc>,
) -> Result<Option<UserTokenRecord>, sqlx::Error> {
   if raw_token.trim().is_empty() {
      return Ok(None);
   }
   let user_token_record: Option<UserTokenRecord> = user_token_repository::find_user_token_by_hash_for_update(
      &mut *connection,
      &sha256_hex(raw_token.trim()),
      user_token_type,
   )
   .await?;
   Ok(user_token_record.filter(|user_token_record| user_token_record.is_usable(now)))
}
