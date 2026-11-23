#[derive(sqlx::FromRow)]
pub struct BranchAssignmentRecord {
   pub cinema_id: i64,
   pub cinema_name: Option<String>,
   pub is_primary: bool,
}
