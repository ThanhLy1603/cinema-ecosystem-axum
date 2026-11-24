use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;

use crate::common::error::AppError;
use crate::common::extract::{ApiJson, ApiPath, QueryParams};
use crate::common::response::{ApiResponse, PageResponse};
use crate::modules::auth::AuthenticatedUser;
use crate::modules::cinema::dto::request::CinemaRequest;
use crate::modules::cinema::dto::response::{CinemaDetailResponse, CinemaResponse};
use crate::modules::cinema::service::cinema_admin_service::{self, CinemaSearchQuery};
use crate::state::AppState;

const ADMIN_OR_MANAGER: &[&str] = &["ROLE_ADMIN", "ROLE_MANAGER"];
const ADMIN_ONLY: &[&str] = &["ROLE_ADMIN"];

pub async fn search_cinemas(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   query_params: QueryParams,
) -> Result<Json<ApiResponse<PageResponse<CinemaResponse>>>, AppError> {
   let cinema_search_query: CinemaSearchQuery = CinemaSearchQuery {
      keyword: query_params.text("search"),
      city: query_params.text("city"),
      status: query_params.enumeration("status")?,
      deleted: query_params.boolean("deleted", false)?,
      page: query_params.integer("page", 0)?,
      size: query_params.integer("size", 20)?,
   };
   authenticated_user.require_any_role(if cinema_search_query.deleted { ADMIN_ONLY } else { ADMIN_OR_MANAGER })?;
   let page_response: PageResponse<CinemaResponse> =
      cinema_admin_service::search_cinemas(&app_state, authenticated_user.user_id, cinema_search_query).await?;
   Ok(Json(ApiResponse::success(page_response)))
}

pub async fn get_cinema(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(cinema_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<CinemaDetailResponse>>, AppError> {
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let cinema_detail_response: CinemaDetailResponse =
      cinema_admin_service::get_cinema(&app_state, authenticated_user.user_id, cinema_id).await?;
   Ok(Json(ApiResponse::success(cinema_detail_response)))
}

pub async fn create_cinema(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiJson(cinema_request): ApiJson<CinemaRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CinemaDetailResponse>>), AppError> {
   let cinema_command = cinema_request.validate()?;
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   let cinema_detail_response: CinemaDetailResponse =
      cinema_admin_service::create_cinema(&app_state, cinema_command).await?;
   Ok((StatusCode::CREATED, Json(ApiResponse::success_with_message("Tạo cụm rạp thành công", cinema_detail_response))))
}

pub async fn update_cinema(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(cinema_id): ApiPath<i64>,
   ApiJson(cinema_request): ApiJson<CinemaRequest>,
) -> Result<Json<ApiResponse<CinemaDetailResponse>>, AppError> {
   let cinema_command = cinema_request.validate()?;
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let cinema_detail_response: CinemaDetailResponse =
      cinema_admin_service::update_cinema(&app_state, authenticated_user.user_id, cinema_id, cinema_command).await?;
   Ok(Json(ApiResponse::success_with_message("Cập nhật cụm rạp thành công", cinema_detail_response)))
}

pub async fn delete_cinema(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(cinema_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   cinema_admin_service::delete_cinema(&app_state, cinema_id).await?;
   Ok(Json(ApiResponse::message_only("Xóa cụm rạp thành công")))
}

pub async fn restore_cinema(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(cinema_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<CinemaDetailResponse>>, AppError> {
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   let cinema_detail_response: CinemaDetailResponse =
      cinema_admin_service::restore_cinema(&app_state, cinema_id).await?;
   Ok(Json(ApiResponse::success_with_message("Khôi phục cụm rạp thành công", cinema_detail_response)))
}
