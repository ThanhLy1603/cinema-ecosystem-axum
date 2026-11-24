#[derive(Clone, sqlx::FromRow)]
pub struct FacilityRecord {
   pub id: i64,
   pub name: String,
   pub icon_url: Option<String>,
   pub description: Option<String>,
   pub is_deleted: bool,
}
