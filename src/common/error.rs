use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use crate::common::response::ApiResponse;

#[derive(Debug)]
pub struct AppError {
   pub status: StatusCode,
   pub error_code: String,
   pub message: String,
   pub data: Option<serde_json::Value>,
}

impl AppError {
   pub fn new(status: StatusCode, error_code: &str, message: &str) -> AppError {
      AppError { status, error_code: error_code.to_string(), message: message.to_string(), data: None }
   }

   pub fn not_found(error_code: &str, message: &str) -> AppError {
      AppError::new(StatusCode::NOT_FOUND, error_code, message)
   }

   pub fn bad_request(error_code: &str, message: &str) -> AppError {
      AppError::new(StatusCode::BAD_REQUEST, error_code, message)
   }

   pub fn unauthorized(error_code: &str, message: &str) -> AppError {
      AppError::new(StatusCode::UNAUTHORIZED, error_code, message)
   }

   pub fn forbidden(error_code: &str, message: &str) -> AppError {
      AppError::new(StatusCode::FORBIDDEN, error_code, message)
   }

   pub fn conflict(error_code: &str, message: &str) -> AppError {
      AppError::new(StatusCode::CONFLICT, error_code, message)
   }

   pub fn internal() -> AppError {
      AppError::new(
         StatusCode::INTERNAL_SERVER_ERROR,
         "INTERNAL_SERVER_ERROR",
         "Đã xảy ra lỗi hệ thống, vui lòng thử lại sau",
      )
   }

   pub fn validation(field_errors: BTreeMap<String, String>) -> AppError {
      AppError {
         status: StatusCode::BAD_REQUEST,
         error_code: "VALIDATION_ERROR".to_string(),
         message: "Dữ liệu đầu vào không hợp lệ".to_string(),
         data: serde_json::to_value(field_errors).ok(),
      }
   }
}

impl From<sqlx::Error> for AppError {
   fn from(sqlx_error: sqlx::Error) -> AppError {
      tracing::error!(error = %sqlx_error, "database error");
      AppError::internal()
   }
}

impl IntoResponse for AppError {
   fn into_response(self) -> Response {
      let body: ApiResponse<serde_json::Value> = match self.data {
         Some(data) => ApiResponse::error_with_data(self.message, self.error_code, data),
         None => ApiResponse::error(self.message, self.error_code),
      };
      (self.status, Json(body)).into_response()
   }
}

#[cfg(test)]
mod tests {
   use std::collections::BTreeMap;

   use axum::http::StatusCode;
   use axum::response::IntoResponse;
   use http_body_util::BodyExt;

   use super::AppError;

   #[tokio::test]
   async fn validation_error_puts_field_map_into_data() {
      let mut field_errors: BTreeMap<String, String> = BTreeMap::new();
      field_errors.insert("email".to_string(), "Email không hợp lệ".to_string());
      let response = AppError::validation(field_errors).into_response();
      assert_eq!(response.status(), StatusCode::BAD_REQUEST);
      let bytes = response.into_body().collect().await.unwrap().to_bytes();
      let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
      assert_eq!(body["errorCode"], "VALIDATION_ERROR");
      assert_eq!(body["message"], "Dữ liệu đầu vào không hợp lệ");
      assert_eq!(body["data"]["email"], "Email không hợp lệ");
   }

   #[tokio::test]
   async fn plain_error_has_no_data() {
      let response = AppError::unauthorized("UNAUTHORIZED", "Chưa đăng nhập").into_response();
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
      let bytes = response.into_body().collect().await.unwrap().to_bytes();
      let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
      assert_eq!(body["errorCode"], "UNAUTHORIZED");
      assert!(body.get("data").is_none());
   }
}
