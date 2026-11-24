use crate::modules::cinema::enums::{HallStatus, ScreenType};

#[derive(Clone, sqlx::FromRow)]
pub struct HallRecord {
   pub id: i64,
   pub cinema_id: i64,
   pub name: String,
   #[sqlx(try_from = "String")]
   pub screen_type: ScreenType,
   pub total_seats: i32,
   #[sqlx(try_from = "String")]
   pub status: HallStatus,
   pub version: i64,
   pub is_deleted: bool,
}
