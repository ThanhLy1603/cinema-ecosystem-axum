use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageResponse<T> {
   pub content: Vec<T>,
   pub page: i64,
   pub size: i64,
   pub total_elements: i64,
   pub total_pages: i64,
}

impl<T> PageResponse<T> {
   pub fn of(content: Vec<T>, page: i64, size: i64, total_elements: i64) -> PageResponse<T> {
      let total_pages: i64 = if size <= 0 { 0 } else { (total_elements + size - 1) / size };
      PageResponse { content, page, size, total_elements, total_pages }
   }
}

#[cfg(test)]
mod tests {
   use super::{ApiResponse, PageResponse};

   #[test]
   fn page_response_computes_total_pages_and_uses_camel_case() {
      let page_response: PageResponse<i32> = PageResponse::of(vec![1, 2], 0, 2, 5);
      assert_eq!(page_response.total_pages, 3);
      let value: serde_json::Value = serde_json::to_value(&page_response).unwrap();
      assert_eq!(value["totalElements"], 5);
      assert_eq!(value["totalPages"], 3);
      assert_eq!(PageResponse::<i32>::of(vec![], 0, 20, 0).total_pages, 0);
   }

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
