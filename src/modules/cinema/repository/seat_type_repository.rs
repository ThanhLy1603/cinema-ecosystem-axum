use sqlx::PgExecutor;

use crate::modules::cinema::entity::SeatTypeRecord;

pub async fn find_all(executor: impl PgExecutor<'_>) -> Result<Vec<SeatTypeRecord>, sqlx::Error> {
   sqlx::query_as::<_, SeatTypeRecord>(
      "SELECT seat_type.id, seat_type.code, seat_type.name, seat_type.default_surcharge::float8 AS default_surcharge, \
       seat_type.seat_span::int4 AS seat_span FROM seat_types seat_type ORDER BY seat_type.id",
   )
   .fetch_all(executor)
   .await
}
