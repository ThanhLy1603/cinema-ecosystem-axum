use crate::common::error::AppError;
use crate::modules::auth::{resolve_branch_scope, BranchScope};
use crate::modules::cinema::dto::response::CinemaOptionResponse;
use crate::modules::cinema::repository::cinema_repository;
use crate::state::AppState;

pub async fn find_cinema_options(
   app_state: &AppState,
   actor_user_id: i64,
) -> Result<Vec<CinemaOptionResponse>, AppError> {
   let cinema_option_responses: Vec<CinemaOptionResponse> =
      cinema_repository::find_active_options(&app_state.pg_pool).await?;
   Ok(match resolve_branch_scope(app_state, actor_user_id).await? {
      BranchScope::All => cinema_option_responses,
      BranchScope::Only(accessible_cinema_ids) => cinema_option_responses
         .into_iter()
         .filter(|cinema_option_response| accessible_cinema_ids.contains(&cinema_option_response.id))
         .collect(),
   })
}
