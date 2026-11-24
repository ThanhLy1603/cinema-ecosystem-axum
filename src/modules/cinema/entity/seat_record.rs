use crate::modules::cinema::enums::SeatStatus;

#[derive(Clone, sqlx::FromRow)]
pub struct SeatRecord {
   pub id: i64,
   pub seat_row: String,
   pub seat_number: i32,
   pub seat_type_code: String,
   pub surcharge: f64,
   pub seat_span: i32,
   #[sqlx(try_from = "String")]
   pub status: SeatStatus,
}
