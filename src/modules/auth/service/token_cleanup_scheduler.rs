use std::time::Duration;

use crate::modules::auth::repository::refresh_token_repository;
use crate::modules::auth::repository::user_token_repository;
use chrono::Utc;
use sqlx::PgPool;
use tokio::time::{interval_at, Instant};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const FIRST_RUN_DELAY: Duration = Duration::from_secs(60);

pub async fn delete_expired_tokens(pg_pool: &PgPool) -> Result<(u64, u64), sqlx::Error> {
   let threshold = Utc::now() - chrono::Duration::days(1);
   let deleted_refresh_tokens: u64 =
      refresh_token_repository::delete_refresh_tokens_expired_before(pg_pool, threshold).await?;
   let deleted_user_tokens: u64 = user_token_repository::delete_user_tokens_expired_before(pg_pool, threshold).await?;
   Ok((deleted_refresh_tokens, deleted_user_tokens))
}

pub fn spawn_token_cleanup(pg_pool: PgPool) {
   tokio::spawn(async move {
      let mut cleanup_interval = interval_at(Instant::now() + FIRST_RUN_DELAY, CLEANUP_INTERVAL);
      loop {
         cleanup_interval.tick().await;
         match delete_expired_tokens(&pg_pool).await {
            Ok((deleted_refresh_tokens, deleted_user_tokens)) => {
               tracing::info!(deleted_refresh_tokens, deleted_user_tokens, "Đã dọn token hết hạn");
            }
            Err(sqlx_error) => tracing::warn!(error = %sqlx_error, "Không dọn được token hết hạn"),
         }
      }
   });
}
