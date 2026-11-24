use chrono::Utc;

use crate::common::error::AppError;
use crate::modules::cinema::dto::request::HallCommand;
use crate::modules::cinema::dto::response::HallResponse;
use crate::modules::cinema::entity::HallRecord;
use crate::modules::cinema::repository::{hall_repository, hall_usage_repository, is_integrity_violation};
use crate::modules::cinema::service::cinema_access_guard::{
   require_accessible_cinema, require_accessible_hall, require_deleted_hall_of_active_cinema,
};
use crate::modules::cinema::service::public_cinema_cache::PUBLIC_CINEMA_CACHE;
use crate::state::AppState;

fn name_already_used() -> AppError {
   AppError::conflict("HALL_NAME_EXISTS", "Tên phòng chiếu đã tồn tại trong cụm rạp này")
}

pub async fn list_halls(
   app_state: &AppState,
   actor_user_id: i64,
   cinema_id: i64,
   deleted: bool,
) -> Result<Vec<HallResponse>, AppError> {
   require_accessible_cinema(app_state, actor_user_id, cinema_id).await?;
   Ok(hall_repository::find_by_cinema(&app_state.pg_pool, cinema_id, deleted)
      .await?
      .into_iter()
      .map(HallResponse::from)
      .collect())
}

pub async fn create_hall(
   app_state: &AppState,
   actor_user_id: i64,
   cinema_id: i64,
   hall_command: HallCommand,
) -> Result<HallResponse, AppError> {
   require_accessible_cinema(app_state, actor_user_id, cinema_id).await?;
   if hall_repository::exists_name(&app_state.pg_pool, cinema_id, &hall_command.name, None).await? {
      return Err(name_already_used());
   }
   let hall_record: HallRecord = match hall_repository::insert(&app_state.pg_pool, cinema_id, &hall_command).await {
      Ok(hall_record) => hall_record,
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(name_already_used()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(HallResponse::from(hall_record))
}

pub async fn update_hall(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
   hall_command: HallCommand,
) -> Result<HallResponse, AppError> {
   let existing_hall_record: HallRecord = require_accessible_hall(app_state, actor_user_id, hall_id).await?;
   if hall_repository::exists_name(
      &app_state.pg_pool,
      existing_hall_record.cinema_id,
      &hall_command.name,
      Some(hall_id),
   )
   .await?
   {
      return Err(name_already_used());
   }
   let is_unchanged: bool = existing_hall_record.name == hall_command.name
      && existing_hall_record.screen_type == hall_command.screen_type
      && existing_hall_record.status == hall_command.status;
   if is_unchanged {
      return Ok(HallResponse::from(existing_hall_record));
   }
   let hall_record: HallRecord = match hall_repository::update(&app_state.pg_pool, hall_id, &hall_command).await {
      Ok(hall_record) => hall_record,
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(name_already_used()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(HallResponse::from(hall_record))
}

pub async fn delete_hall(app_state: &AppState, actor_user_id: i64, hall_id: i64) -> Result<(), AppError> {
   require_accessible_hall(app_state, actor_user_id, hall_id).await?;
   let now = Utc::now();
   if hall_usage_repository::count_upcoming_showtimes_by_hall(&app_state.pg_pool, hall_id, now).await? > 0 {
      return Err(AppError::conflict(
         "HALL_HAS_UPCOMING_SHOWTIMES",
         "Phòng chiếu còn suất chiếu chưa diễn ra, hãy hủy suất chiếu hoặc chuyển sang bảo trì",
      ));
   }
   hall_repository::mark_deleted(&app_state.pg_pool, hall_id, now).await?;
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(())
}

pub async fn restore_hall(app_state: &AppState, actor_user_id: i64, hall_id: i64) -> Result<HallResponse, AppError> {
   let deleted_hall_record: HallRecord =
      require_deleted_hall_of_active_cinema(app_state, actor_user_id, hall_id).await?;
   if hall_repository::exists_name(&app_state.pg_pool, deleted_hall_record.cinema_id, &deleted_hall_record.name, None)
      .await?
   {
      return Err(name_already_used());
   }
   let hall_record: HallRecord = match hall_repository::restore(&app_state.pg_pool, hall_id).await {
      Ok(hall_record) => hall_record,
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(name_already_used()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(HallResponse::from(hall_record))
}
