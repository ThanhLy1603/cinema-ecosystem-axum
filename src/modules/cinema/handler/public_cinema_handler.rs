use axum::extract::State;
use axum::response::Json;

use crate::common::error::AppError;
use crate::common::extract::{ApiPath, QueryParams};
use crate::common::response::ApiResponse;
use crate::modules::cinema::dto::response::{
   CinemaDetailResponse, CinemaResponse, SeatLayoutResponse, SeatTypeResponse,
};
use crate::modules::cinema::service::cinema_public_service;
use crate::state::AppState;

pub async fn list_public_cinemas(
   State(app_state): State<AppState>,
   query_params: QueryParams,
) -> Result<Json<ApiResponse<Vec<CinemaResponse>>>, AppError> {
   let cinema_responses: Vec<CinemaResponse> =
      cinema_public_service::find_public_cinemas(&app_state, query_params.text("city")).await?;
   Ok(Json(ApiResponse::success(cinema_responses)))
}

pub async fn list_public_cities(State(app_state): State<AppState>) -> Result<Json<ApiResponse<Vec<String>>>, AppError> {
   let cities: Vec<String> = cinema_public_service::find_public_cities(&app_state).await?;
   Ok(Json(ApiResponse::success(cities)))
}

pub async fn get_public_cinema_by_slug(
   State(app_state): State<AppState>,
   ApiPath(slug): ApiPath<String>,
) -> Result<Json<ApiResponse<CinemaDetailResponse>>, AppError> {
   let cinema_detail_response: CinemaDetailResponse =
      cinema_public_service::find_public_cinema_by_slug(&app_state, &slug).await?;
   Ok(Json(ApiResponse::success(cinema_detail_response)))
}

pub async fn get_seat_layout(
   State(app_state): State<AppState>,
   ApiPath(hall_id): ApiPath<i64>,
) -> Result<Json<ApiResponse<SeatLayoutResponse>>, AppError> {
   let seat_layout_response: SeatLayoutResponse =
      cinema_public_service::find_public_seat_layout(&app_state, hall_id).await?;
   Ok(Json(ApiResponse::success(seat_layout_response)))
}

pub async fn list_seat_types(
   State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<SeatTypeResponse>>>, AppError> {
   let seat_type_responses: Vec<SeatTypeResponse> = cinema_public_service::find_seat_types(&app_state).await?;
   Ok(Json(ApiResponse::success(seat_type_responses)))
}
