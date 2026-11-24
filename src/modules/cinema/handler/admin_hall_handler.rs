use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;

use crate::common::error::AppError;
use crate::common::extract::{ApiJson, ApiPath, QueryParams};
use crate::common::response::ApiResponse;
use crate::modules::auth::AuthenticatedUser;
use crate::modules::cinema::dto::request::HallRequest;
use crate::modules::cinema::dto::response::HallResponse;
use crate::modules::cinema::service::hall_service;
use crate::state::AppState;

const ADMIN_OR_MANAGER: &[&str] = &["ROLE_ADMIN", "ROLE_MANAGER"];

pub async fn list_halls(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(cinema_id): ApiPath<i64>,
   query_params: QueryParams,
) -> Result<Json<ApiResponse<Vec<HallResponse>>>, AppError> {
   let deleted: bool = query_params.boolean("deleted", false)?;
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let hall_responses: Vec<HallResponse> =
      hall_service::list_halls(&app_state, authenticated_user.user_id, cinema_id, deleted).await?;
   Ok(Json(ApiResponse::success(hall_responses)))
}

pub async fn create_hall(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(cinema_id): ApiPath<i64>,
   ApiJson(hall_request): ApiJson<HallRequest>,
) -> Result<(StatusCode, Json<ApiResponse<HallResponse>>), AppError> {
   let hall_command = hall_request.validate()?;
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let hall_response: HallResponse =
      hall_service::create_hall(&app_state, authenticated_user.user_id, cinema_id, hall_command).await?;
   Ok((StatusCode::CREATED, Json(ApiResponse::success_with_message("Thêm phòng chiếu thành công", hall_response))))
}

pub async fn update_hall(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
   ApiJson(hall_request): ApiJson<HallRequest>,
) -> Result<Json<ApiResponse<HallResponse>>, AppError> {
   let hall_command = hall_request.validate()?;
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let hall_response: HallResponse =
      hall_service::update_hall(&app_state, authenticated_user.user_id, hall_id, hall_command).await?;
   Ok(Json(ApiResponse::success_with_message("Cập nhật phòng chiếu thành công", hall_response)))
}

pub async fn delete_hall(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   hall_service::delete_hall(&app_state, authenticated_user.user_id, hall_id).await?;
   Ok(Json(ApiResponse::message_only("Xóa phòng chiếu thành công")))
}

pub async fn restore_hall(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<HallResponse>>, AppError> {
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let hall_response: HallResponse =
      hall_service::restore_hall(&app_state, authenticated_user.user_id, hall_id).await?;
   Ok(Json(ApiResponse::success_with_message("Khôi phục phòng chiếu thành công", hall_response)))
}
