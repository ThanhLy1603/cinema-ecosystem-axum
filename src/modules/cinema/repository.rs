use sqlx::types::Json as SqlxJson;
use sqlx::{FromRow, PgPool};

use super::dto::FacilityResponse;

#[derive(FromRow)]
pub struct CinemaRow {
   pub id: i64,
   pub name: String,
   pub slug: String,
   pub city: String,
   pub address: String,
   pub phone: Option<String>,
   pub latitude: Option<f64>,
   pub longitude: Option<f64>,
   pub status: String,
   pub hall_count: i32,
   pub facilities: SqlxJson<Vec<FacilityResponse>>,
}

const FIND_ACTIVE_CINEMAS_SQL: &str = r#"
SELECT cinema.id, cinema.name, cinema.slug, cinema.city, cinema.address, cinema.phone,
       cinema.latitude::float8 AS latitude, cinema.longitude::float8 AS longitude, cinema.status,
       (SELECT count(*) FROM halls hall
         WHERE hall.cinema_id = cinema.id AND NOT hall.is_deleted AND hall.status = 'ACTIVE')::int4 AS hall_count,
       COALESCE((SELECT json_agg(json_build_object(
                    'id', facility.id, 'name', facility.name, 'iconUrl', facility.icon_url,
                    'description', facility.description, 'deleted', facility.is_deleted) ORDER BY facility.id)
                   FROM cinema_facility_mappings mapping
                   JOIN cinema_facilities facility ON facility.id = mapping.facility_id
                  WHERE mapping.cinema_id = cinema.id AND NOT facility.is_deleted), '[]'::json) AS facilities
  FROM cinemas cinema
 WHERE cinema.status = 'ACTIVE' AND NOT cinema.is_deleted
   AND ($1::text IS NULL OR cinema.city = $1)
 ORDER BY cinema.city, cinema.name, cinema.id
"#;

pub async fn find_active_cinemas(pg_pool: &PgPool, city: Option<&str>) -> Result<Vec<CinemaRow>, sqlx::Error> {
   sqlx::query_as::<_, CinemaRow>(FIND_ACTIVE_CINEMAS_SQL).bind(city).fetch_all(pg_pool).await
}

pub async fn find_active_cities(pg_pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
   sqlx::query_scalar::<_, String>(
      "SELECT DISTINCT city FROM cinemas WHERE status = 'ACTIVE' AND NOT is_deleted ORDER BY city",
   )
   .fetch_all(pg_pool)
   .await
}
