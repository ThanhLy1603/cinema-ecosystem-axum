pub mod refresh_token_repository;
pub mod role_repository;
pub mod user_branch_assignment_repository;
pub mod user_identity_repository;
pub mod user_repository;
pub mod user_token_repository;

pub fn is_unique_violation(sqlx_error: &sqlx::Error) -> bool {
   sqlx_error.as_database_error().and_then(|database_error| database_error.code()).is_some_and(|code| code == "23505")
}
