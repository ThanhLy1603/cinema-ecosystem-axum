pub mod cinema_repository;
pub mod facility_repository;
pub mod hall_repository;
pub mod hall_usage_repository;
pub mod seat_repository;
pub mod seat_type_repository;

pub fn is_integrity_violation(sqlx_error: &sqlx::Error) -> bool {
   sqlx_error
      .as_database_error()
      .and_then(|database_error| database_error.code())
      .is_some_and(|code| code.starts_with("23"))
}
