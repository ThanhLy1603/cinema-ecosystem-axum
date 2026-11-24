use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::modules::cinema::dto::request::CinemaCommand;
use crate::modules::cinema::dto::response::CinemaOptionResponse;
use crate::modules::cinema::entity::{CinemaFacilityRecord, CinemaRecord};

const CINEMA_COLUMNS: &str = "cinema.id, cinema.name, cinema.slug, cinema.city, cinema.address, cinema.phone, \
   cinema.latitude::float8 AS latitude, cinema.longitude::float8 AS longitude, cinema.status, cinema.is_deleted";

pub struct CinemaSearchFilter {
   pub keyword_pattern: Option<String>,
   pub city: Option<String>,
   pub status: Option<String>,
   pub cinema_ids: Option<Vec<i64>>,
   pub deleted: bool,
}

const SEARCH_CONDITION: &str = "cinema.is_deleted = $1 \
   AND ($2::text IS NULL OR cinema.status = $2) \
   AND ($3::text IS NULL OR lower(cinema.name) LIKE $3 ESCAPE '\\' OR lower(cinema.address) LIKE $3 ESCAPE '\\') \
   AND ($4::text IS NULL OR cinema.city = $4) \
   AND ($5::bigint[] IS NULL OR cinema.id = ANY($5))";

pub async fn search(
   executor: impl PgExecutor<'_>,
   cinema_search_filter: &CinemaSearchFilter,
   limit: i64,
   offset: i64,
) -> Result<Vec<CinemaRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {CINEMA_COLUMNS} FROM cinemas cinema WHERE {SEARCH_CONDITION} ORDER BY cinema.name, cinema.id LIMIT $6 OFFSET $7"
   );
   sqlx::query_as::<_, CinemaRecord>(&sql)
      .bind(cinema_search_filter.deleted)
      .bind(cinema_search_filter.status.as_deref())
      .bind(cinema_search_filter.keyword_pattern.as_deref())
      .bind(cinema_search_filter.city.as_deref())
      .bind(cinema_search_filter.cinema_ids.as_deref())
      .bind(limit)
      .bind(offset)
      .fetch_all(executor)
      .await
}

pub async fn count_search(
   executor: impl PgExecutor<'_>,
   cinema_search_filter: &CinemaSearchFilter,
) -> Result<i64, sqlx::Error> {
   let sql: String = format!("SELECT count(*) FROM cinemas cinema WHERE {SEARCH_CONDITION}");
   sqlx::query_scalar::<_, i64>(&sql)
      .bind(cinema_search_filter.deleted)
      .bind(cinema_search_filter.status.as_deref())
      .bind(cinema_search_filter.keyword_pattern.as_deref())
      .bind(cinema_search_filter.city.as_deref())
      .bind(cinema_search_filter.cinema_ids.as_deref())
      .fetch_one(executor)
      .await
}

pub async fn find_by_id(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   deleted: bool,
) -> Result<Option<CinemaRecord>, sqlx::Error> {
   let sql: String =
      format!("SELECT {CINEMA_COLUMNS} FROM cinemas cinema WHERE cinema.id = $1 AND cinema.is_deleted = $2");
   sqlx::query_as::<_, CinemaRecord>(&sql).bind(cinema_id).bind(deleted).fetch_optional(executor).await
}

pub async fn find_active_by_slug(
   executor: impl PgExecutor<'_>,
   slug: &str,
) -> Result<Option<CinemaRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {CINEMA_COLUMNS} FROM cinemas cinema WHERE cinema.slug = $1 AND cinema.status = 'ACTIVE' AND NOT cinema.is_deleted"
   );
   sqlx::query_as::<_, CinemaRecord>(&sql).bind(slug).fetch_optional(executor).await
}

pub async fn find_all_active(
   executor: impl PgExecutor<'_>,
   city: Option<&str>,
) -> Result<Vec<CinemaRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {CINEMA_COLUMNS} FROM cinemas cinema WHERE cinema.status = 'ACTIVE' AND NOT cinema.is_deleted \
       AND ($1::text IS NULL OR cinema.city = $1) ORDER BY cinema.city, cinema.name, cinema.id"
   );
   sqlx::query_as::<_, CinemaRecord>(&sql).bind(city).fetch_all(executor).await
}

pub async fn find_active_cities(executor: impl PgExecutor<'_>) -> Result<Vec<String>, sqlx::Error> {
   sqlx::query_scalar::<_, String>(
      "SELECT DISTINCT city FROM cinemas WHERE status = 'ACTIVE' AND NOT is_deleted ORDER BY city",
   )
   .fetch_all(executor)
   .await
}

pub async fn find_active_options(executor: impl PgExecutor<'_>) -> Result<Vec<CinemaOptionResponse>, sqlx::Error> {
   sqlx::query_as::<_, CinemaOptionResponse>(
      "SELECT cinema.id, cinema.name, cinema.city FROM cinemas cinema \
       WHERE cinema.status = 'ACTIVE' AND NOT cinema.is_deleted ORDER BY cinema.name, cinema.id",
   )
   .fetch_all(executor)
   .await
}

pub async fn exists_active_slug(executor: impl PgExecutor<'_>, slug: &str) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM cinemas WHERE slug = $1 AND NOT is_deleted)")
      .bind(slug)
      .fetch_one(executor)
      .await
}

pub async fn insert(
   executor: impl PgExecutor<'_>,
   cinema_command: &CinemaCommand,
   slug: &str,
) -> Result<CinemaRecord, sqlx::Error> {
   let sql: String = format!(
      "WITH cinema AS (INSERT INTO cinemas (name, slug, city, address, phone, latitude, longitude, status) \
       VALUES ($1, $2, $3, $4, $5, $6::float8::numeric, $7::float8::numeric, $8) RETURNING *) \
       SELECT {CINEMA_COLUMNS} FROM cinema"
   );
   sqlx::query_as::<_, CinemaRecord>(&sql)
      .bind(&cinema_command.name)
      .bind(slug)
      .bind(&cinema_command.city)
      .bind(&cinema_command.address)
      .bind(cinema_command.phone.as_deref())
      .bind(cinema_command.latitude)
      .bind(cinema_command.longitude)
      .bind(cinema_command.status.as_str())
      .fetch_one(executor)
      .await
}

pub async fn update(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   cinema_command: &CinemaCommand,
) -> Result<CinemaRecord, sqlx::Error> {
   let sql: String = format!(
      "WITH cinema AS (UPDATE cinemas SET name = $2, city = $3, address = $4, phone = $5, \
       latitude = $6::float8::numeric, longitude = $7::float8::numeric, status = $8 WHERE id = $1 RETURNING *) \
       SELECT {CINEMA_COLUMNS} FROM cinema"
   );
   sqlx::query_as::<_, CinemaRecord>(&sql)
      .bind(cinema_id)
      .bind(&cinema_command.name)
      .bind(&cinema_command.city)
      .bind(&cinema_command.address)
      .bind(cinema_command.phone.as_deref())
      .bind(cinema_command.latitude)
      .bind(cinema_command.longitude)
      .bind(cinema_command.status.as_str())
      .fetch_one(executor)
      .await
}

pub async fn mark_deleted(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   deleted_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE cinemas SET is_deleted = TRUE, deleted_at = $2 WHERE id = $1")
      .bind(cinema_id)
      .bind(deleted_at)
      .execute(executor)
      .await
      .map(|_| ())
}

pub async fn restore(executor: impl PgExecutor<'_>, cinema_id: i64, slug: &str) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE cinemas SET is_deleted = FALSE, deleted_at = NULL, slug = $2 WHERE id = $1")
      .bind(cinema_id)
      .bind(slug)
      .execute(executor)
      .await
      .map(|_| ())
}

pub async fn replace_facilities(
   transaction: &mut sqlx::PgConnection,
   cinema_id: i64,
   facility_ids: &[i64],
) -> Result<(), sqlx::Error> {
   sqlx::query("DELETE FROM cinema_facility_mappings WHERE cinema_id = $1")
      .bind(cinema_id)
      .execute(&mut *transaction)
      .await?;
   sqlx::query(
      "INSERT INTO cinema_facility_mappings (cinema_id, facility_id) SELECT $1, facility_id FROM UNNEST($2::bigint[]) AS facility_id",
   )
   .bind(cinema_id)
   .bind(facility_ids)
   .execute(&mut *transaction)
   .await
   .map(|_| ())
}

pub async fn find_facilities(
   executor: impl PgExecutor<'_>,
   cinema_ids: &[i64],
) -> Result<Vec<CinemaFacilityRecord>, sqlx::Error> {
   sqlx::query_as::<_, CinemaFacilityRecord>(
      "SELECT mapping.cinema_id, facility.id, facility.name, facility.icon_url, facility.description, facility.is_deleted \
       FROM cinema_facility_mappings mapping JOIN cinema_facilities facility ON facility.id = mapping.facility_id \
       WHERE mapping.cinema_id = ANY($1) AND NOT facility.is_deleted ORDER BY lower(facility.name), facility.id",
   )
   .bind(cinema_ids)
   .fetch_all(executor)
   .await
}

pub async fn count_halls(
   executor: impl PgExecutor<'_>,
   cinema_ids: &[i64],
   active_only: bool,
) -> Result<Vec<(i64, i64)>, sqlx::Error> {
   sqlx::query_as::<_, (i64, i64)>(
      "SELECT hall.cinema_id, count(*) FROM halls hall WHERE hall.cinema_id = ANY($1) AND NOT hall.is_deleted \
       AND (NOT $2 OR hall.status = 'ACTIVE') GROUP BY hall.cinema_id",
   )
   .bind(cinema_ids)
   .bind(active_only)
   .fetch_all(executor)
   .await
}
