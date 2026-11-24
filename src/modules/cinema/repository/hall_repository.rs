use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::modules::cinema::dto::request::HallCommand;
use crate::modules::cinema::entity::HallRecord;

const HALL_COLUMNS: &str =
   "hall.id, hall.cinema_id, hall.name, hall.screen_type, hall.total_seats, hall.status, hall.version, hall.is_deleted";

const HALL_RETURNING: &str =
   "RETURNING hall.id, hall.cinema_id, hall.name, hall.screen_type, hall.total_seats, hall.status, hall.version, hall.is_deleted";

pub async fn find_by_id(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   deleted: bool,
) -> Result<Option<HallRecord>, sqlx::Error> {
   let sql: String = format!("SELECT {HALL_COLUMNS} FROM halls hall WHERE hall.id = $1 AND hall.is_deleted = $2");
   sqlx::query_as::<_, HallRecord>(&sql).bind(hall_id).bind(deleted).fetch_optional(executor).await
}

pub async fn find_active_by_id_for_update(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
) -> Result<Option<HallRecord>, sqlx::Error> {
   let sql: String =
      format!("SELECT {HALL_COLUMNS} FROM halls hall WHERE hall.id = $1 AND NOT hall.is_deleted FOR UPDATE");
   sqlx::query_as::<_, HallRecord>(&sql).bind(hall_id).fetch_optional(executor).await
}

pub async fn find_by_cinema(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   deleted: bool,
) -> Result<Vec<HallRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {HALL_COLUMNS} FROM halls hall WHERE hall.cinema_id = $1 AND hall.is_deleted = $2 ORDER BY hall.name, hall.id"
   );
   sqlx::query_as::<_, HallRecord>(&sql).bind(cinema_id).bind(deleted).fetch_all(executor).await
}

pub async fn find_active_by_cinema(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
) -> Result<Vec<HallRecord>, sqlx::Error> {
   let sql: String = format!(
      "SELECT {HALL_COLUMNS} FROM halls hall WHERE hall.cinema_id = $1 AND hall.status = 'ACTIVE' AND NOT hall.is_deleted \
       ORDER BY hall.name, hall.id"
   );
   sqlx::query_as::<_, HallRecord>(&sql).bind(cinema_id).fetch_all(executor).await
}

pub async fn exists_name(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   name: &str,
   except_hall_id: Option<i64>,
) -> Result<bool, sqlx::Error> {
   sqlx::query_scalar::<_, bool>(
      "SELECT EXISTS (SELECT 1 FROM halls WHERE cinema_id = $1 AND lower(name) = lower($2) AND NOT is_deleted \
       AND ($3::bigint IS NULL OR id <> $3))",
   )
   .bind(cinema_id)
   .bind(name)
   .bind(except_hall_id)
   .fetch_one(executor)
   .await
}

pub async fn insert(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   hall_command: &HallCommand,
) -> Result<HallRecord, sqlx::Error> {
   let sql: String = format!(
      "INSERT INTO halls AS hall (cinema_id, name, screen_type, status) VALUES ($1, $2, $3, $4) {HALL_RETURNING}"
   );
   sqlx::query_as::<_, HallRecord>(&sql)
      .bind(cinema_id)
      .bind(&hall_command.name)
      .bind(hall_command.screen_type.as_str())
      .bind(hall_command.status.as_str())
      .fetch_one(executor)
      .await
}

pub async fn update(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   hall_command: &HallCommand,
) -> Result<HallRecord, sqlx::Error> {
   let sql: String = format!(
      "UPDATE halls AS hall SET name = $2, screen_type = $3, status = $4, version = hall.version + 1 \
       WHERE hall.id = $1 {HALL_RETURNING}"
   );
   sqlx::query_as::<_, HallRecord>(&sql)
      .bind(hall_id)
      .bind(&hall_command.name)
      .bind(hall_command.screen_type.as_str())
      .bind(hall_command.status.as_str())
      .fetch_one(executor)
      .await
}

pub async fn mark_deleted(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   deleted_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE halls SET is_deleted = TRUE, deleted_at = $2, version = version + 1 WHERE id = $1")
      .bind(hall_id)
      .bind(deleted_at)
      .execute(executor)
      .await
      .map(|_| ())
}

pub async fn restore(executor: impl PgExecutor<'_>, hall_id: i64) -> Result<HallRecord, sqlx::Error> {
   let sql: String = format!(
      "UPDATE halls AS hall SET is_deleted = FALSE, deleted_at = NULL, version = hall.version + 1 \
       WHERE hall.id = $1 {HALL_RETURNING}"
   );
   sqlx::query_as::<_, HallRecord>(&sql).bind(hall_id).fetch_one(executor).await
}

pub async fn update_seat_map_summary(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   total_seats: i32,
   updated_at: DateTime<Utc>,
) -> Result<HallRecord, sqlx::Error> {
   let sql: String = format!(
      "UPDATE halls AS hall SET total_seats = $2, seat_map_updated_at = $3, version = hall.version + 1 \
       WHERE hall.id = $1 {HALL_RETURNING}"
   );
   sqlx::query_as::<_, HallRecord>(&sql).bind(hall_id).bind(total_seats).bind(updated_at).fetch_one(executor).await
}
