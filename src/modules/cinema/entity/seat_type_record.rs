#[derive(Clone, sqlx::FromRow)]
pub struct SeatTypeRecord {
   pub id: i64,
   pub code: String,
   pub name: String,
   pub default_surcharge: f64,
   pub seat_span: i32,
}
