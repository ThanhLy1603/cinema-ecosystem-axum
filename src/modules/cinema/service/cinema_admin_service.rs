use chrono::Utc;

use crate::common::error::AppError;
use crate::common::response::PageResponse;
use crate::common::slug::to_slug;
use crate::modules::auth::{resolve_branch_scope, BranchScope};
use crate::modules::cinema::dto::request::CinemaCommand;
use crate::modules::cinema::dto::response::{CinemaDetailResponse, CinemaResponse};
use crate::modules::cinema::entity::CinemaRecord;
use crate::modules::cinema::enums::CinemaStatus;
use crate::modules::cinema::repository::cinema_repository::{self, CinemaSearchFilter};
use crate::modules::cinema::repository::{facility_repository, hall_usage_repository, is_integrity_violation};
use crate::modules::cinema::service::cinema_access_guard::{cinema_not_found, require_accessible_cinema};
use crate::modules::cinema::service::cinema_assembler::{assemble_cinema_detail, assemble_cinema_responses};
use crate::modules::cinema::service::public_cinema_cache::PUBLIC_CINEMA_CACHE;
use crate::state::AppState;

const MAX_PAGE_SIZE: i64 = 100;
const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_SLUG_ATTEMPTS: i64 = 50;
const FALLBACK_SLUG: &str = "cinema";

pub struct CinemaSearchQuery {
   pub keyword: Option<String>,
   pub city: Option<String>,
   pub status: Option<CinemaStatus>,
   pub deleted: bool,
   pub page: i64,
   pub size: i64,
}

fn escape_like(value: &str) -> String {
   value.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn blank_to_none(value: Option<String>) -> Option<String> {
   value.map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

fn cinema_conflict() -> AppError {
   AppError::conflict("CINEMA_CONFLICT", "Cụm rạp đã tồn tại hoặc dữ liệu không hợp lệ")
}

pub async fn search_cinemas(
   app_state: &AppState,
   actor_user_id: i64,
   cinema_search_query: CinemaSearchQuery,
) -> Result<PageResponse<CinemaResponse>, AppError> {
   let cinema_ids: Option<Vec<i64>> = match resolve_branch_scope(app_state, actor_user_id).await? {
      BranchScope::All => None,
      BranchScope::Only(accessible_cinema_ids) => Some(accessible_cinema_ids.into_iter().collect()),
   };
   let page: i64 = cinema_search_query.page.max(0);
   let size: i64 =
      if cinema_search_query.size <= 0 { DEFAULT_PAGE_SIZE } else { cinema_search_query.size.min(MAX_PAGE_SIZE) };
   let cinema_search_filter: CinemaSearchFilter = CinemaSearchFilter {
      keyword_pattern: blank_to_none(cinema_search_query.keyword)
         .map(|keyword| format!("%{}%", escape_like(&keyword.to_lowercase()))),
      city: blank_to_none(cinema_search_query.city),
      status: cinema_search_query.status.map(|status| status.as_str().to_string()),
      cinema_ids,
      deleted: cinema_search_query.deleted,
   };
   let cinema_records: Vec<CinemaRecord> =
      cinema_repository::search(&app_state.pg_pool, &cinema_search_filter, size, page.saturating_mul(size)).await?;
   let total_elements: i64 = cinema_repository::count_search(&app_state.pg_pool, &cinema_search_filter).await?;
   let cinema_responses: Vec<CinemaResponse> =
      assemble_cinema_responses(&app_state.pg_pool, cinema_records, false).await?;
   Ok(PageResponse::of(cinema_responses, page, size, total_elements))
}

pub async fn get_cinema(
   app_state: &AppState,
   actor_user_id: i64,
   cinema_id: i64,
) -> Result<CinemaDetailResponse, AppError> {
   let cinema_record: CinemaRecord = require_accessible_cinema(app_state, actor_user_id, cinema_id).await?;
   Ok(assemble_cinema_detail(&app_state.pg_pool, cinema_record, false).await?)
}

pub async fn create_cinema(
   app_state: &AppState,
   cinema_command: CinemaCommand,
) -> Result<CinemaDetailResponse, AppError> {
   cinema_command.ensure_coordinates_given_together()?;
   let facility_ids: Vec<i64> = resolve_facility_ids(app_state, &cinema_command.facility_ids).await?;
   let slug: String = generate_unique_slug(app_state, &cinema_command.name).await?;
   let mut transaction = app_state.pg_pool.begin().await?;
   let cinema_record: CinemaRecord = match cinema_repository::insert(&mut *transaction, &cinema_command, &slug).await {
      Ok(cinema_record) => cinema_record,
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(cinema_conflict()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   save_facilities(&mut transaction, cinema_record.id, &facility_ids).await?;
   transaction.commit().await?;
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(assemble_cinema_detail(&app_state.pg_pool, cinema_record, false).await?)
}

pub async fn update_cinema(
   app_state: &AppState,
   actor_user_id: i64,
   cinema_id: i64,
   cinema_command: CinemaCommand,
) -> Result<CinemaDetailResponse, AppError> {
   let existing_cinema_record: CinemaRecord = require_accessible_cinema(app_state, actor_user_id, cinema_id).await?;
   cinema_command.ensure_coordinates_given_together()?;
   if cinema_command.status != existing_cinema_record.status
      && !resolve_branch_scope(app_state, actor_user_id).await?.has_access_to_all()
   {
      return Err(AppError::forbidden("ADMIN_ONLY", "Chỉ quản trị viên mới được đổi trạng thái cụm rạp"));
   }
   let facility_ids: Vec<i64> = resolve_facility_ids(app_state, &cinema_command.facility_ids).await?;
   let mut transaction = app_state.pg_pool.begin().await?;
   let cinema_record: CinemaRecord =
      match cinema_repository::update(&mut *transaction, cinema_id, &cinema_command).await {
         Ok(cinema_record) => cinema_record,
         Err(sqlx_error) if is_integrity_violation(&sqlx_error) => return Err(cinema_conflict()),
         Err(sqlx_error) => return Err(sqlx_error.into()),
      };
   save_facilities(&mut transaction, cinema_id, &facility_ids).await?;
   transaction.commit().await?;
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(assemble_cinema_detail(&app_state.pg_pool, cinema_record, false).await?)
}

pub async fn delete_cinema(app_state: &AppState, cinema_id: i64) -> Result<(), AppError> {
   cinema_repository::find_by_id(&app_state.pg_pool, cinema_id, false).await?.ok_or_else(cinema_not_found)?;
   let now = Utc::now();
   if hall_usage_repository::count_upcoming_showtimes_by_cinema(&app_state.pg_pool, cinema_id, now).await? > 0 {
      return Err(AppError::conflict(
         "CINEMA_HAS_UPCOMING_SHOWTIMES",
         "Cụm rạp còn suất chiếu chưa diễn ra, hãy hủy suất chiếu hoặc chuyển rạp sang tạm ngưng",
      ));
   }
   cinema_repository::mark_deleted(&app_state.pg_pool, cinema_id, now).await?;
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(())
}

pub async fn restore_cinema(app_state: &AppState, cinema_id: i64) -> Result<CinemaDetailResponse, AppError> {
   let deleted_cinema_record: CinemaRecord = cinema_repository::find_by_id(&app_state.pg_pool, cinema_id, true)
      .await?
      .ok_or_else(|| AppError::not_found("CINEMA_NOT_FOUND", "Không tìm thấy cụm rạp đã xóa"))?;
   let slug: String = if cinema_repository::exists_active_slug(&app_state.pg_pool, &deleted_cinema_record.slug).await? {
      generate_unique_slug(app_state, &deleted_cinema_record.name).await?
   } else {
      deleted_cinema_record.slug.clone()
   };
   if let Err(sqlx_error) = cinema_repository::restore(&app_state.pg_pool, cinema_id, &slug).await {
      return Err(if is_integrity_violation(&sqlx_error) { cinema_conflict() } else { sqlx_error.into() });
   }
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   let cinema_record: CinemaRecord =
      cinema_repository::find_by_id(&app_state.pg_pool, cinema_id, false).await?.ok_or_else(cinema_not_found)?;
   Ok(assemble_cinema_detail(&app_state.pg_pool, cinema_record, false).await?)
}

async fn resolve_facility_ids(app_state: &AppState, facility_ids: &[i64]) -> Result<Vec<i64>, AppError> {
   let mut distinct_facility_ids: Vec<i64> = facility_ids.to_vec();
   distinct_facility_ids.sort_unstable();
   distinct_facility_ids.dedup();
   if distinct_facility_ids.is_empty() {
      return Ok(distinct_facility_ids);
   }
   let active_facility_ids: Vec<i64> =
      facility_repository::find_active_ids(&app_state.pg_pool, &distinct_facility_ids).await?;
   if active_facility_ids.len() != distinct_facility_ids.len() {
      return Err(AppError::bad_request("INVALID_FACILITY", "Có tiện ích không tồn tại"));
   }
   Ok(distinct_facility_ids)
}

async fn save_facilities(
   transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
   cinema_id: i64,
   facility_ids: &[i64],
) -> Result<(), AppError> {
   match cinema_repository::replace_facilities(&mut *transaction, cinema_id, facility_ids).await {
      Ok(()) => Ok(()),
      Err(sqlx_error) if is_integrity_violation(&sqlx_error) => {
         Err(AppError::bad_request("INVALID_FACILITY", "Có tiện ích không tồn tại"))
      }
      Err(sqlx_error) => Err(sqlx_error.into()),
   }
}

async fn generate_unique_slug(app_state: &AppState, name: &str) -> Result<String, AppError> {
   let base_slug: String = to_slug(name, FALLBACK_SLUG);
   let mut slug: String = base_slug.clone();
   let mut attempt: i64 = 2;
   while cinema_repository::exists_active_slug(&app_state.pg_pool, &slug).await? {
      if attempt > MAX_SLUG_ATTEMPTS {
         return Err(AppError::conflict("SLUG_EXHAUSTED", "Không tạo được đường dẫn cho cụm rạp"));
      }
      slug = format!("{base_slug}-{attempt}");
      attempt += 1;
   }
   Ok(slug)
}

#[cfg(test)]
mod tests {
   use super::escape_like;

   #[test]
   fn like_wildcards_are_escaped() {
      assert_eq!(escape_like("100%_a\\b"), "100\\%\\_a\\\\b");
   }
}
