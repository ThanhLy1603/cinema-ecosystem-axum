use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
   pub success: bool,
   pub message: String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub data: Option<T>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub error_code: Option<String>,
   pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
   pub fn success(data: T) -> ApiResponse<T> {
      ApiResponse::success_with_message("Thành công", data)
   }

   pub fn success_with_message(message: impl Into<String>, data: T) -> ApiResponse<T> {
      ApiResponse { success: true, message: message.into(), data: Some(data), error_code: None, timestamp: Utc::now() }
   }

   pub fn error(message: impl Into<String>, error_code: impl Into<String>) -> ApiResponse<T> {
      ApiResponse {
         success: false,
         message: message.into(),
         data: None,
         error_code: Some(error_code.into()),
         timestamp: Utc::now(),
      }
   }

   pub fn error_with_data(message: impl Into<String>, error_code: impl Into<String>, data: T) -> ApiResponse<T> {
      ApiResponse {
         success: false,
         message: message.into(),
         data: Some(data),
         error_code: Some(error_code.into()),
         timestamp: Utc::now(),
      }
   }
}

impl ApiResponse<()> {
   pub fn message_only(message: impl Into<String>) -> ApiResponse<()> {
      ApiResponse { success: true, message: message.into(), data: None, error_code: None, timestamp: Utc::now() }
   }
}

#[cfg(test)]
mod tests {
   use super::ApiResponse;

   #[test]
   fn success_serializes_camel_case_and_omits_error_code() {
      let value: serde_json::Value = serde_json::to_value(ApiResponse::success(42)).unwrap();
      assert_eq!(value["success"], true);
      assert_eq!(value["data"], 42);
      assert!(value.get("errorCode").is_none());
      assert!(value["timestamp"].is_string());
   }

   #[test]
   fn error_serializes_error_code_and_omits_data() {
      let value: serde_json::Value = serde_json::to_value(ApiResponse::<()>::error("Lỗi", "SOME_CODE")).unwrap();
      assert_eq!(value["success"], false);
      assert_eq!(value["errorCode"], "SOME_CODE");
      assert!(value.get("data").is_none());
   }

   #[test]
   fn error_with_data_keeps_both_error_code_and_data() {
      let value: serde_json::Value = serde_json::to_value(ApiResponse::error_with_data(
         "Lỗi",
         "VALIDATION_ERROR",
         serde_json::json!({ "email": "sai" }),
      ))
      .unwrap();
      assert_eq!(value["success"], false);
      assert_eq!(value["errorCode"], "VALIDATION_ERROR");
      assert_eq!(value["data"]["email"], "sai");
   }

   #[test]
   fn message_only_has_success_true_and_no_data() {
      let value: serde_json::Value = serde_json::to_value(ApiResponse::message_only("Xong")).unwrap();
      assert_eq!(value["success"], true);
      assert_eq!(value["message"], "Xong");
      assert!(value.get("data").is_none());
   }
}
