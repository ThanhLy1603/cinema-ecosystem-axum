use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

pub async fn find_linked_providers(executor: impl PgExecutor<'_>, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
   sqlx::query_scalar::<_, String>("SELECT provider FROM user_identities WHERE user_id = $1")
      .bind(user_id)
      .fetch_all(executor)
      .await
}

const GOOGLE_PROVIDER: &str = "GOOGLE";

pub async fn find_user_id_by_google_subject(
   executor: impl PgExecutor<'_>,
   subject: &str,
) -> Result<Option<i64>, sqlx::Error> {
   sqlx::query_scalar::<_, i64>("SELECT user_id FROM user_identities WHERE provider = $1 AND provider_user_id = $2")
      .bind(GOOGLE_PROVIDER)
      .bind(subject)
      .fetch_optional(executor)
      .await
}

pub async fn exists_google_identity_for_user(executor: impl PgExecutor<'_>, user_id: i64) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM user_identities WHERE user_id = $1 AND provider = $2)")
      .bind(user_id)
      .bind(GOOGLE_PROVIDER)
      .fetch_one(executor)
      .await
}

pub async fn insert_google_identity(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   subject: &str,
   email: &str,
   now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query(
      "INSERT INTO user_identities (user_id, provider, provider_user_id, provider_email, last_login_at) \
       VALUES ($1, $2, $3, $4, $5)",
   )
   .bind(user_id)
   .bind(GOOGLE_PROVIDER)
   .bind(subject)
   .bind(email)
   .bind(now)
   .execute(executor)
   .await
   .map(|_| ())
}

pub async fn touch_google_identity(
   executor: impl PgExecutor<'_>,
   user_id: i64,
   now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE user_identities SET last_login_at = $3 WHERE user_id = $1 AND provider = $2")
      .bind(user_id)
      .bind(GOOGLE_PROVIDER)
      .bind(now)
      .execute(executor)
      .await
      .map(|_| ())
}
