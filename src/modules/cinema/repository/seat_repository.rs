use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::modules::cinema::entity::SeatRecord;

pub async fn find_active_by_hall(executor: impl PgExecutor<'_>, hall_id: i64) -> Result<Vec<SeatRecord>, sqlx::Error> {
   sqlx::query_as::<_, SeatRecord>(
      "SELECT seat.id, seat.seat_row, seat.seat_number, seat_type.code AS seat_type_code, \
       seat_type.default_surcharge::float8 AS surcharge, seat_type.seat_span::int4 AS seat_span, seat.status \
       FROM seats seat JOIN seat_types seat_type ON seat_type.id = seat.seat_type_id \
       WHERE seat.hall_id = $1 AND NOT seat.is_deleted ORDER BY seat.seat_row, seat.seat_number",
   )
   .bind(hall_id)
   .fetch_all(executor)
   .await
}

pub async fn count_active_by_hall(executor: impl PgExecutor<'_>, hall_id: i64) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>("SELECT count(*) FROM seats WHERE hall_id = $1 AND NOT is_deleted")
      .bind(hall_id)
      .fetch_one(executor)
      .await
}

pub async fn mark_deleted(
   executor: impl PgExecutor<'_>,
   seat_ids: &[i64],
   deleted_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
   sqlx::query("UPDATE seats SET is_deleted = TRUE, deleted_at = $2 WHERE id = ANY($1)")
      .bind(seat_ids)
      .bind(deleted_at)
      .execute(executor)
      .await
      .map(|_| ())
}

pub struct SeatInsert {
   pub seat_type_id: i64,
   pub seat_row: String,
   pub seat_number: i32,
   pub status: String,
}

pub async fn insert_all(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   seat_inserts: &[SeatInsert],
) -> Result<(), sqlx::Error> {
   let seat_type_ids: Vec<i64> = seat_inserts.iter().map(|seat_insert| seat_insert.seat_type_id).collect();
   let seat_rows: Vec<String> = seat_inserts.iter().map(|seat_insert| seat_insert.seat_row.clone()).collect();
   let seat_numbers: Vec<i32> = seat_inserts.iter().map(|seat_insert| seat_insert.seat_number).collect();
   let statuses: Vec<String> = seat_inserts.iter().map(|seat_insert| seat_insert.status.clone()).collect();
   sqlx::query(
      "INSERT INTO seats (hall_id, seat_type_id, seat_row, seat_number, status) \
       SELECT $1, data.seat_type_id, data.seat_row, data.seat_number, data.status \
       FROM UNNEST($2::bigint[], $3::text[], $4::int[], $5::text[]) AS data(seat_type_id, seat_row, seat_number, status)",
   )
   .bind(hall_id)
   .bind(seat_type_ids)
   .bind(seat_rows)
   .bind(seat_numbers)
   .bind(statuses)
   .execute(executor)
   .await
   .map(|_| ())
}

pub struct SeatUpdate {
   pub seat_id: i64,
   pub seat_type_id: i64,
   pub status: String,
}

pub async fn update_all(executor: impl PgExecutor<'_>, seat_updates: &[SeatUpdate]) -> Result<(), sqlx::Error> {
   let seat_ids: Vec<i64> = seat_updates.iter().map(|seat_update| seat_update.seat_id).collect();
   let seat_type_ids: Vec<i64> = seat_updates.iter().map(|seat_update| seat_update.seat_type_id).collect();
   let statuses: Vec<String> = seat_updates.iter().map(|seat_update| seat_update.status.clone()).collect();
   sqlx::query(
      "UPDATE seats SET seat_type_id = data.seat_type_id, status = data.status \
       FROM UNNEST($1::bigint[], $2::bigint[], $3::text[]) AS data(seat_id, seat_type_id, status) \
       WHERE seats.id = data.seat_id",
   )
   .bind(seat_ids)
   .bind(seat_type_ids)
   .bind(statuses)
   .execute(executor)
   .await
   .map(|_| ())
}
