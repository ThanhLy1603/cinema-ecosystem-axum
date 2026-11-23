use sqlx::PgPool;

use super::dto::CinemaResponse;
use super::repository;
use crate::common::error::AppError;

pub async fn find_public_cinemas(pg_pool: &PgPool, city: Option<String>) -> Result<Vec<CinemaResponse>, AppError> {
   let normalized_city: Option<String> = city.map(|city| city.trim().to_string()).filter(|city| !city.is_empty());
   let cinema_rows = repository::find_active_cinemas(pg_pool, normalized_city.as_deref()).await?;
   Ok(cinema_rows.into_iter().map(CinemaResponse::from).collect())
}

pub async fn find_public_cities(pg_pool: &PgPool) -> Result<Vec<String>, AppError> {
   Ok(repository::find_active_cities(pg_pool).await?)
}
