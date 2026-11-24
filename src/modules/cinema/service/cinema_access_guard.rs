use crate::common::error::AppError;
use crate::modules::auth::assert_can_access_cinema;
use crate::modules::cinema::entity::{CinemaRecord, HallRecord};
use crate::modules::cinema::repository::{cinema_repository, hall_repository};
use crate::state::AppState;

pub fn cinema_not_found() -> AppError {
   AppError::not_found("CINEMA_NOT_FOUND", "Không tìm thấy cụm rạp")
}

pub fn hall_not_found() -> AppError {
   AppError::not_found("HALL_NOT_FOUND", "Không tìm thấy phòng chiếu")
}

pub async fn require_accessible_cinema(
   app_state: &AppState,
   actor_user_id: i64,
   cinema_id: i64,
) -> Result<CinemaRecord, AppError> {
   let cinema_record: CinemaRecord =
      cinema_repository::find_by_id(&app_state.pg_pool, cinema_id, false).await?.ok_or_else(cinema_not_found)?;
   assert_can_access_cinema(app_state, actor_user_id, cinema_record.id).await?;
   Ok(cinema_record)
}

pub async fn require_accessible_hall(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
) -> Result<HallRecord, AppError> {
   let hall_record: HallRecord =
      hall_repository::find_by_id(&app_state.pg_pool, hall_id, false).await?.ok_or_else(hall_not_found)?;
   require_accessible_cinema(app_state, actor_user_id, hall_record.cinema_id).await?;
   Ok(hall_record)
}

pub async fn require_deleted_hall_of_active_cinema(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
) -> Result<HallRecord, AppError> {
   let hall_record: HallRecord =
      hall_repository::find_by_id(&app_state.pg_pool, hall_id, true).await?.ok_or_else(hall_not_found)?;
   assert_can_access_cinema(app_state, actor_user_id, hall_record.cinema_id).await?;
   if cinema_repository::find_by_id(&app_state.pg_pool, hall_record.cinema_id, false).await?.is_none() {
      return Err(AppError::conflict(
         "CINEMA_DELETED",
         "Cụm rạp của phòng chiếu này đã bị xóa, hãy khôi phục cụm rạp trước",
      ));
   }
   Ok(hall_record)
}
