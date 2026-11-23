use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgExecutor;

use crate::modules::auth::entity::UserRecord;

macro_rules! select_user {
   ($tail:literal) => {
      concat!(
         "SELECT id, email, password_hash, email_verified, full_name, phone, avatar_url, date_of_birth, gender, tier, \
          loyalty_points, status, failed_login_attempts, locked_until, last_login_at \
          FROM users ",
         $tail
      )
   };
}

pub async fn find_user_by_id(executor: impl PgExecutor<'_>, user_id: i64) -> Result<Option<UserRecord>, sqlx::Error> {
   sqlx::query_as::<_, UserRecord>(select_user!("WHERE id = $1")).bind(user_id).fetch_optional(executor).await
}

pub async fn find_user_by_id_for_update(
   executor: impl PgExecutor<'_>,
   user_id: i64,
) -> Result<Option<UserRecord>, sqlx::Error> {
   sqlx::query_as::<_, UserRecord>(select_user!("WHERE id = $1 FOR UPDATE"))
      .bind(user_id)
      .fetch_optional(executor)
      .await
}

pub async fn find_user_by_email(executor: impl PgExecutor<'_>, email: &str) -> Result<Option<UserRecord>, sqlx::Error> {
   sqlx::query_as::<_, UserRecord>(select_user!("WHERE email = $1")).bind(email).fetch_optional(executor).await
}

pub async fn find_user_by_email_for_update(
   executor: impl PgExecutor<'_>,
   email: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
   sqlx::query_as::<_, UserRecord>(select_user!("WHERE email = $1 FOR UPDATE"))
      .bind(email)
      .fetch_optional(executor)
      .await
}

pub async fn exists_user_by_email(executor: impl PgExecutor<'_>, email: &str) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
      .bind(email)
      .fetch_one(executor)
      .await
}

pub async fn exists_user_by_phone(executor: impl PgExecutor<'_>, phone: &str) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE phone = $1)")
      .bind(phone)
      .fetch_one(executor)
      .await
}

pub async fn exists_user_by_phone_and_id_not(
   executor: impl PgExecutor<'_>,
   phone: &str,
   user_id: i64,
) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE phone = $1 AND id <> $2)")
      .bind(phone)
      .bind(user_id)
      .fetch_one(executor)
      .await
}

pub async fn insert_user(
   executor: impl PgExecutor<'_>,
   email: &str,
   password_hash: &str,
   full_name: &str,
   phone: Option<&str>,
) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>(
      "INSERT INTO users (email, password_hash, full_name, phone) VALUES ($1, $2, $3, $4) RETURNING id",
   )
   .bind(email)
   .bind(password_hash)
   .bind(full_name)
   .bind(phone)
   .fetch_one(executor)
   .await
}

pub async fn update_login_state(executor: impl PgExecutor<'_>, user_record: &UserRecord) -> Result<(), sqlx::Error> {
   sqlx::query(
      "UPDATE users SET failed_login_attempts = $2, status = $3, locked_until = $4, last_login_at = $5, \
       updated_at = CURRENT_TIMESTAMP WHERE id = $1",
   )
   .bind(user_record.id)
   .bind(user_record.failed_login_attempts)
   .bind(user_record.status.as_str())
   .bind(user_record.locked_until)
   .bind(user_record.last_login_at)
   .execute(executor)
   .await
   .map(|_| ())
}

pub async fn update_password_after_reset(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   password_hash: &str,
   now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query(
      "UPDATE users SET password_hash = $2, password_changed_at = $3, failed_login_attempts = 0, locked_until = NULL, \
       status = CASE WHEN status = 'LOCKED' THEN 'ACTIVE' ELSE status END, email_verified = TRUE, \
       updated_at = CURRENT_TIMESTAMP WHERE id = $1",
   )
   .bind(user_id)
   .bind(password_hash)
   .bind(now)
   .execute(executor)
   .await
   .map(|_| ())
}

pub async fn update_password_after_change(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   password_hash: &str,
   now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query(
      "UPDATE users SET password_hash = $2, password_changed_at = $3, failed_login_attempts = 0, locked_until = NULL, \
       updated_at = CURRENT_TIMESTAMP WHERE id = $1",
   )
   .bind(user_id)
   .bind(password_hash)
   .bind(now)
   .execute(executor)
   .await
   .map(|_| ())
}

pub struct ProfileUpdate<'a> {
   pub full_name: &'a str,
   pub phone: Option<&'a str>,
   pub date_of_birth: Option<NaiveDate>,
   pub gender: Option<&'a str>,
   pub avatar_url: Option<&'a str>,
}

pub async fn update_profile(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   profile_update: &ProfileUpdate<'_>,
) -> Result<(), sqlx::Error> {
   sqlx::query(
      "UPDATE users SET full_name = $2, phone = $3, date_of_birth = $4, gender = $5, avatar_url = $6, \
       updated_at = CURRENT_TIMESTAMP WHERE id = $1",
   )
   .bind(user_id)
   .bind(profile_update.full_name)
   .bind(profile_update.phone)
   .bind(profile_update.date_of_birth)
   .bind(profile_update.gender)
   .bind(profile_update.avatar_url)
   .execute(executor)
   .await
   .map(|_| ())
}

pub async fn mark_email_verified(executor: impl PgExecutor<'_>, user_id: i64) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE users SET email_verified = TRUE, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
      .bind(user_id)
      .execute(executor)
      .await
      .map(|_| ())
}
