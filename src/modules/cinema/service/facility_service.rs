use chrono::Utc;

use crate::common::error::AppError;
use crate::modules::cinema::dto::request::FacilityCommand;
use crate::modules::cinema::dto::response::FacilityResponse;
use crate::modules::cinema::entity::FacilityRecord;
use crate::modules::cinema::repository::{facility_repository, is_integrity_violation};
use crate::modules::cinema::service::public_cinema_cache::PUBLIC_CINEMA_CACHE;
use crate::state::AppState;

fn name_already_used() -> AppError {
   AppError::conflict("FACILITY_NAME_EXISTS", "Tên tiện ích đã tồn tại")
}

fn facility_not_found() -> AppError {
   AppError::not_found("FACILITY_NOT_FOUND", "Không tìm thấy tiện ích")
}

async fn load_facility(app_state: &AppState, facility_id: i64, deleted: bool) -> Result<FacilityRecord, AppError> {
   facility_repository::find_by_id(&app_state.pg_pool, facility_id, deleted).await?.ok_or_else(facility_not_found)
}

pub async fn list_facilities(app_state: &AppState, deleted: bool) -> Result<Vec<FacilityResponse>, AppError> {
   Ok(facility_repository::find_all(&app_state.pg_pool, deleted)
      .await?
      .into_iter()
      .map(FacilityResponse::from)
      .collect())
}

pub async fn create_facility(
   app_state: &AppState,
   facility_command: FacilityCommand,
) -> Result<FacilityResponse, AppError> {
   if facility_repository::exists_active_name(&app_state.pg_pool, &facility_command.name, None).await? {
      return Err(name_already_used());
   }
   let facility_record: FacilityRecord = match facility_repository::insert(&app_state.pg_pool, &facility_command).await
   {
      Ok(facility_record) => facility_record,
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(name_already_used()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(FacilityResponse::from(facility_record))
}

pub async fn update_facility(
   app_state: &AppState,
   facility_id: i64,
   facility_command: FacilityCommand,
) -> Result<FacilityResponse, AppError> {
   load_facility(app_state, facility_id, false).await?;
   if facility_repository::exists_active_name(&app_state.pg_pool, &facility_command.name, Some(facility_id)).await? {
      return Err(name_already_used());
   }
   let facility_record: FacilityRecord =
      match facility_repository::update(&app_state.pg_pool, facility_id, &facility_command).await {
         Ok(facility_record) => facility_record,
         Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(name_already_used()),
         Err(sqlx_error) => return Err(sqlx_error.into()),
      };
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(FacilityResponse::from(facility_record))
}

pub async fn delete_facility(app_state: &AppState, facility_id: i64) -> Result<(), AppError> {
   load_facility(app_state, facility_id, false).await?;
   facility_repository::mark_deleted(&app_state.pg_pool, facility_id, Utc::now()).await?;
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(())
}

pub async fn restore_facility(app_state: &AppState, facility_id: i64) -> Result<FacilityResponse, AppError> {
   let deleted_facility_record: FacilityRecord = load_facility(app_state, facility_id, true).await?;
   if facility_repository::exists_active_name(&app_state.pg_pool, &deleted_facility_record.name, None).await? {
      return Err(name_already_used());
   }
   let facility_record: FacilityRecord = match facility_repository::restore(&app_state.pg_pool, facility_id).await {
      Ok(facility_record) => facility_record,
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(name_already_used()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(FacilityResponse::from(facility_record))
}
