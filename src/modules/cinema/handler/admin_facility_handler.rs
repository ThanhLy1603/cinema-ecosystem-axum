use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;

use crate::common::error::AppError;
use crate::common::extract::{ApiJson, ApiPath, QueryParams};
use crate::common::response::ApiResponse;
use crate::modules::auth::AuthenticatedUser;
use crate::modules::cinema::dto::request::FacilityRequest;
use crate::modules::cinema::dto::response::FacilityResponse;
use crate::modules::cinema::service::facility_service;
use crate::state::AppState;

const ADMIN_OR_MANAGER: &[&str] = &["ROLE_ADMIN", "ROLE_MANAGER"];
const ADMIN_ONLY: &[&str] = &["ROLE_ADMIN"];

pub async fn list_facilities(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   query_params: QueryParams,
) -> Result<Json<ApiResponse<Vec<FacilityResponse>>>, AppError> {
   let deleted: bool = query_params.boolean("deleted", false)?;
   authenticated_user.require_any_role(if deleted { ADMIN_ONLY } else { ADMIN_OR_MANAGER })?;
   let facility_responses: Vec<FacilityResponse> = facility_service::list_facilities(&app_state, deleted).await?;
   Ok(Json(ApiResponse::success(facility_responses)))
}

pub async fn create_facility(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiJson(facility_request): ApiJson<FacilityRequest>,
) -> Result<(StatusCode, Json<ApiResponse<FacilityResponse>>), AppError> {
   let facility_command = facility_request.validate()?;
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   let facility_response: FacilityResponse = facility_service::create_facility(&app_state, facility_command).await?;
   Ok((StatusCode::CREATED, Json(ApiResponse::success_with_message("Thêm tiện ích thành công", facility_response))))
}

pub async fn update_facility(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(facility_id): ApiPath<i64>,
   ApiJson(facility_request): ApiJson<FacilityRequest>,
) -> Result<Json<ApiResponse<FacilityResponse>>, AppError> {
   let facility_command = facility_request.validate()?;
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   let facility_response: FacilityResponse =
      facility_service::update_facility(&app_state, facility_id, facility_command).await?;
   Ok(Json(ApiResponse::success_with_message("Cập nhật tiện ích thành công", facility_response)))
}

pub async fn delete_facility(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(facility_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   facility_service::delete_facility(&app_state, facility_id).await?;
   Ok(Json(ApiResponse::message_only("Xóa tiện ích thành công")))
}

pub async fn restore_facility(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(facility_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<FacilityResponse>>, AppError> {
   authenticated_user.require_any_role(ADMIN_ONLY)?;
   let facility_response: FacilityResponse = facility_service::restore_facility(&app_state, facility_id).await?;
   Ok(Json(ApiResponse::success_with_message("Khôi phục tiện ích thành công", facility_response)))
}
