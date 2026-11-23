use axum::extract::{Query, State};
use axum::response::Json;

use super::dto::{CinemaResponse, CityQuery};
use super::service;
use crate::common::error::AppError;
use crate::common::response::ApiResponse;
use crate::state::AppState;

pub async fn list_public_cinemas(
   State(app_state): State<AppState>,
   Query(city_query): Query<CityQuery>,
) -> Result<Json<ApiResponse<Vec<CinemaResponse>>>, AppError> {
   let cinema_responses: Vec<CinemaResponse> =
      service::find_public_cinemas(&app_state.pg_pool, city_query.city).await?;
   Ok(Json(ApiResponse::success(cinema_responses)))
}

pub async fn list_public_cities(State(app_state): State<AppState>) -> Result<Json<ApiResponse<Vec<String>>>, AppError> {
   let cities: Vec<String> = service::find_public_cities(&app_state.pg_pool).await?;
   Ok(Json(ApiResponse::success(cities)))
}
