use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use super::dto::HealthResponse;
use super::service;
use crate::common::response::ApiResponse;
use crate::state::AppState;

pub async fn health(State(app_state): State<AppState>) -> Response {
   let health_response: HealthResponse = service::check(&app_state.pg_pool, &app_state.redis_client).await;
   let status_code: StatusCode =
      if health_response.database == "UP" { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
   (status_code, Json(ApiResponse::success(health_response))).into_response()
}
