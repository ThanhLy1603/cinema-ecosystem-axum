use axum::extract::State;
use axum::response::Json;

use crate::common::error::AppError;
use crate::common::response::ApiResponse;
use crate::modules::auth::AuthenticatedUser;
use crate::modules::cinema::dto::response::CinemaOptionResponse;
use crate::modules::cinema::service::cinema_option_service;
use crate::state::AppState;

pub async fn list_cinema_options(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
) -> Result<Json<ApiResponse<Vec<CinemaOptionResponse>>>, AppError> {
   authenticated_user.require_any_role(&["ROLE_ADMIN", "ROLE_MANAGER"])?;
   let cinema_option_responses: Vec<CinemaOptionResponse> =
      cinema_option_service::find_cinema_options(&app_state, authenticated_user.user_id).await?;
   Ok(Json(ApiResponse::success(cinema_option_responses)))
}
