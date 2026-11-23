use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use crate::common::response::ApiResponse;

#[derive(Debug)]
pub struct AppError {
   pub status: StatusCode,
   pub error_code: String,
   pub message: String,
}

impl AppError {
   pub fn new(status: StatusCode, error_code: &str, message: &str) -> AppError {
      AppError { status, error_code: error_code.to_string(), message: message.to_string() }
   }

   pub fn not_found(error_code: &str, message: &str) -> AppError {
      AppError::new(StatusCode::NOT_FOUND, error_code, message)
   }
}

impl From<sqlx::Error> for AppError {
   fn from(sqlx_error: sqlx::Error) -> AppError {
      tracing::error!(error = %sqlx_error, "database error");
      AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Lỗi hệ thống, vui lòng thử lại sau")
   }
}

impl IntoResponse for AppError {
   fn into_response(self) -> Response {
      let body: ApiResponse<()> = ApiResponse::error(self.message, self.error_code);
      (self.status, Json(body)).into_response()
   }
}
