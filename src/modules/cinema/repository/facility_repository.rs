use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::modules::cinema::dto::request::FacilityCommand;
use crate::modules::cinema::entity::FacilityRecord;

const FACILITY_COLUMNS: &str =
   "facility.id, facility.name, facility.icon_url, facility.description, facility.is_deleted";

pub async fn find_all(executor: impl PgExecutor<'_>, deleted: bool) -> Result<Vec<FacilityRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {FACILITY_COLUMNS} FROM cinema_facilities facility WHERE facility.is_deleted = $1 ORDER BY facility.name, facility.id"
   );
   sqlx::query_as::<_, FacilityRecord>(&sql).bind(deleted).fetch_all(executor).await
}

pub async fn find_by_id(
   executor: impl PgExecutor<'_>,
   facility_id: i64,
   deleted: bool,
) -> Result<Option<FacilityRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {FACILITY_COLUMNS} FROM cinema_facilities facility WHERE facility.id = $1 AND facility.is_deleted = $2"
   );
   sqlx::query_as::<_, FacilityRecord>(&sql).bind(facility_id).bind(deleted).fetch_optional(executor).await
}

pub async fn find_active_ids(executor: impl PgExecutor<'_>, facility_ids: &[i64]) -> Result<Vec<i64>, sqlx::Error> {
   sqlx::query_scalar::<_, i64>("SELECT id FROM cinema_facilities WHERE id = ANY($1) AND NOT is_deleted")
      .bind(facility_ids)
      .fetch_all(executor)
      .await
}

pub async fn exists_active_name(
   executor: impl PgExecutor<'_>,
   name: &str,
   except_facility_id: Option<i64>,
) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>(
      "SELECT EXISTS (SELECT 1 FROM cinema_facilities WHERE lower(name) = lower($1) AND NOT is_deleted \
       AND ($2::bigint IS NULL OR id <> $2))",
   )
   .bind(name)
   .bind(except_facility_id)
   .fetch_one(executor)
   .await
}

pub async fn insert(
   executor: impl PgExecutor<'_>,
   facility_command: &FacilityCommand,
) -> Result<FacilityRecord, sqlx::Error> {
   sqlx::query_as::<_, FacilityRecord>(
      "INSERT INTO cinema_facilities AS facility (name, icon_url, description) VALUES ($1, $2, $3) \
       RETURNING facility.id, facility.name, facility.icon_url, facility.description, facility.is_deleted",
   )
   .bind(&facility_command.name)
   .bind(facility_command.icon_url.as_deref())
   .bind(facility_command.description.as_deref())
   .fetch_one(executor)
   .await
}

pub async fn update(
   executor: impl PgExecutor<'_>,
   facility_id: i64,
   facility_command: &FacilityCommand,
) -> Result<FacilityRecord, sqlx::Error> {
   sqlx::query_as::<_, FacilityRecord>(
      "UPDATE cinema_facilities AS facility SET name = $2, icon_url = $3, description = $4 WHERE facility.id = $1 \
       RETURNING facility.id, facility.name, facility.icon_url, facility.description, facility.is_deleted",
   )
   .bind(facility_id)
   .bind(&facility_command.name)
   .bind(facility_command.icon_url.as_deref())
   .bind(facility_command.description.as_deref())
   .fetch_one(executor)
   .await
}

pub async fn mark_deleted(
   executor: impl PgExecutor<'_>,
   facility_id: i64,
   deleted_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE cinema_facilities SET is_deleted = TRUE, deleted_at = $2 WHERE id = $1")
      .bind(facility_id)
      .bind(deleted_at)
      .execute(executor)
      .await
      .map(|_| ())
}

pub async fn restore(executor: impl PgExecutor<'_>, facility_id: i64) -> Result<FacilityRecord, sqlx::Error> {
   sqlx::query_as::<_, FacilityRecord>(
      "UPDATE cinema_facilities AS facility SET is_deleted = FALSE, deleted_at = NULL WHERE facility.id = $1 \
       RETURNING facility.id, facility.name, facility.icon_url, facility.description, facility.is_deleted",
   )
   .bind(facility_id)
   .fetch_one(executor)
   .await
}
