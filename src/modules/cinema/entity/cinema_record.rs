use crate::modules::cinema::enums::CinemaStatus;

#[derive(Clone, sqlx::FromRow)]
pub struct CinemaRecord {
   pub id: i64,
   pub name: String,
   pub slug: String,
   pub city: String,
   pub address: String,
   pub phone: Option<String>,
   pub latitude: Option<f64>,
   pub longitude: Option<f64>,
   #[sqlx(try_from = "String")]
   pub status: CinemaStatus,
   pub is_deleted: bool,
}
