use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pg_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
   PgPoolOptions::new()
      .max_connections(15)
      .min_connections(2)
      .acquire_timeout(Duration::from_secs(5))
      .connect(database_url)
      .await
}

pub fn create_lazy_pg_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
   PgPoolOptions::new().acquire_timeout(Duration::from_secs(2)).connect_lazy(database_url)
}
