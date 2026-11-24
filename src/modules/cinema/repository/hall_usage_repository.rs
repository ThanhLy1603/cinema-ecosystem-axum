use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

pub async fn count_upcoming_showtimes_by_hall(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   now: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>(
      "SELECT count(*) FROM showtimes showtime \
       WHERE showtime.hall_id = $1 AND showtime.end_time > $2 AND showtime.status = 'ACTIVE'",
   )
   .bind(hall_id)
   .bind(now)
   .fetch_one(executor)
   .await
}

pub async fn count_upcoming_showtimes_by_cinema(
   executor: impl PgExecutor<'_>,
   cinema_id: i64,
   now: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>(
      "SELECT count(*) FROM showtimes showtime JOIN halls hall ON hall.id = showtime.hall_id \
       WHERE hall.cinema_id = $1 AND showtime.end_time > $2 AND showtime.status = 'ACTIVE'",
   )
   .bind(cinema_id)
   .bind(now)
   .fetch_one(executor)
   .await
}

pub async fn count_showtimes_with_bookings_by_hall(
   executor: impl PgExecutor<'_>,
   hall_id: i64,
   now: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
   sqlx::query_scalar::<_, i64>(
      "SELECT count(DISTINCT booked_showtime.showtime_id) FROM ( \
          SELECT seat_reservation.showtime_id FROM seat_reservations seat_reservation \
          JOIN showtimes showtime ON showtime.id = seat_reservation.showtime_id \
          WHERE showtime.hall_id = $1 AND showtime.end_time > $2 \
          AND NOT (seat_reservation.status = 'HELD' AND seat_reservation.expires_at < $2) \
          UNION ALL \
          SELECT ticket.showtime_id FROM tickets ticket \
          JOIN showtimes showtime ON showtime.id = ticket.showtime_id \
          WHERE showtime.hall_id = $1 AND showtime.end_time > $2 \
       ) booked_showtime",
   )
   .bind(hall_id)
   .bind(now)
   .fetch_one(executor)
   .await
}
