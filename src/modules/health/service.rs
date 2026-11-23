use std::time::Duration;

use sqlx::PgPool;
use tokio::time::timeout;

use super::dto::HealthResponse;
use crate::infrastructure::redis;

pub async fn check(pg_pool: &PgPool, redis_client: &::redis::Client) -> HealthResponse {
   let database_up: bool = timeout(Duration::from_secs(3), sqlx::query("SELECT 1").execute(pg_pool))
      .await
      .map(|query_result| query_result.is_ok())
      .unwrap_or(false);
   let redis_up: bool = timeout(Duration::from_secs(2), redis::ping(redis_client)).await.unwrap_or(false);
   HealthResponse {
      status: if database_up { "UP" } else { "DOWN" },
      database: if database_up { "UP" } else { "DOWN" },
      redis: if redis_up { "UP" } else { "DOWN" },
   }
}
