use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;

use crate::common::error::AppError;
use crate::common::extract::{ApiJson, ApiPath};
use crate::common::response::ApiResponse;
use crate::modules::auth::AuthenticatedUser;
use crate::modules::cinema::dto::request::{GenerateSeatsRequest, SeatMapRequest};
use crate::modules::cinema::dto::response::{SeatLayoutResponse, SeatMapLockResponse};
use crate::modules::cinema::service::seat_map_service;
use crate::state::AppState;

const ADMIN_OR_MANAGER: &[&str] = &["ROLE_ADMIN", "ROLE_MANAGER"];

pub async fn get_seat_map(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<SeatLayoutResponse>>, AppError> {
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let seat_layout_response: SeatLayoutResponse =
      seat_map_service::get_seat_map(&app_state, authenticated_user.user_id, hall_id).await?;
   Ok(Json(ApiResponse::success(seat_layout_response)))
}

pub async fn get_seat_map_lock(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<SeatMapLockResponse>>, AppError> {
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let seat_map_lock_response: SeatMapLockResponse =
      seat_map_service::get_seat_map_lock(&app_state, authenticated_user.user_id, hall_id).await?;
   Ok(Json(ApiResponse::success(seat_map_lock_response)))
}

pub async fn save_seat_map(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
   ApiJson(seat_map_request): ApiJson<SeatMapRequest>,
) -> Result<Json<ApiResponse<SeatLayoutResponse>>, AppError> {
   let seat_map_command = seat_map_request.validate()?;
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let seat_layout_response: SeatLayoutResponse =
      seat_map_service::save_seat_map(&app_state, authenticated_user.user_id, hall_id, seat_map_command).await?;
   Ok(Json(ApiResponse::success_with_message("Lưu sơ đồ ghế thành công", seat_layout_response)))
}

pub async fn generate_seats(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiPath(hall_id): ApiPath<i64>,
   ApiJson(generate_seats_request): ApiJson<GenerateSeatsRequest>,
) -> Result<(StatusCode, Json<ApiResponse<SeatLayoutResponse>>), AppError> {
   let generate_seats_command = generate_seats_request.validate()?;
   authenticated_user.require_any_role(ADMIN_OR_MANAGER)?;
   let seat_layout_response: SeatLayoutResponse =
      seat_map_service::generate_seats(&app_state, authenticated_user.user_id, hall_id, generate_seats_command).await?;
   Ok((StatusCode::CREATED, Json(ApiResponse::success_with_message("Sinh sơ đồ ghế thành công", seat_layout_response))))
}
