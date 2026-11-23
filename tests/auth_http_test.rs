mod common;

use axum::http::{Method, StatusCode};
use chrono::{Duration, Utc};
use common::{send, test_app, JsonResponse, RequestOptions, TestApp, UNREACHABLE_DATABASE_URL, UNREACHABLE_REDIS_URL};
use serde_json::json;

fn app() -> TestApp {
   test_app(UNREACHABLE_DATABASE_URL, UNREACHABLE_REDIS_URL)
}

fn access_token_for(app: &TestApp, roles: &[&str]) -> String {
   let role_codes: Vec<String> = roles.iter().map(|role| role.to_string()).collect();
   app.app_state.jwt_service.generate_access_token(42, "a@example.com", &role_codes, Utc::now(), 900).unwrap()
}

#[tokio::test]
async fn login_with_empty_body_returns_validation_error_with_field_map_in_data() {
   let test_app: TestApp = app();
   let response: JsonResponse =
      send(test_app.router.clone(), RequestOptions::new(Method::POST, "/api/v1/auth/login").json(json!({}))).await;
   assert_eq!(response.status, StatusCode::BAD_REQUEST);
   assert_eq!(response.body["success"], false);
   assert_eq!(response.body["errorCode"], "VALIDATION_ERROR");
   assert_eq!(response.body["message"], "Dữ liệu đầu vào không hợp lệ");
   assert_eq!(response.body["data"]["email"], "Email không được để trống");
   assert_eq!(response.body["data"]["password"], "Mật khẩu không được để trống");
}

#[tokio::test]
async fn malformed_json_returns_invalid_request_body() {
   let test_app: TestApp = app();
   let response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/login").raw_body("{khong-phai-json"),
   )
   .await;
   assert_eq!(response.status, StatusCode::BAD_REQUEST);
   assert_eq!(response.body["errorCode"], "INVALID_REQUEST_BODY");
   assert_eq!(response.body["message"], "Nội dung yêu cầu không hợp lệ");
}

#[tokio::test]
async fn register_reports_all_invalid_fields_at_once() {
   let test_app: TestApp = app();
   let response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/register").json(json!({
         "email": "sai-email",
         "password": "123",
         "fullName": "",
         "phone": "abc"
      })),
   )
   .await;
   assert_eq!(response.status, StatusCode::BAD_REQUEST);
   assert_eq!(response.body["data"]["email"], "Email không hợp lệ");
   assert_eq!(response.body["data"]["password"], "Mật khẩu phải từ 8 đến 72 ký tự");
   assert_eq!(response.body["data"]["fullName"], "Họ tên không được để trống");
   assert_eq!(response.body["data"]["phone"], "Số điện thoại không hợp lệ");
}

#[tokio::test]
async fn valid_login_with_unreachable_database_is_mapped_to_internal_server_error() {
   let test_app: TestApp = app();
   let response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/login")
         .json(json!({ "email": "a@example.com", "password": "matkhau123" })),
   )
   .await;
   assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
   assert_eq!(response.body["errorCode"], "INTERNAL_SERVER_ERROR");
   assert!(response.set_cookies.is_empty());
}

#[tokio::test]
async fn forgot_password_and_verify_email_validate_input_before_touching_the_database() {
   let test_app: TestApp = app();
   let forgot_response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/forgot-password").json(json!({ "email": "sai" })),
   )
   .await;
   assert_eq!(forgot_response.status, StatusCode::BAD_REQUEST);
   assert_eq!(forgot_response.body["data"]["email"], "Email không hợp lệ");

   let verify_response: JsonResponse =
      send(test_app.router.clone(), RequestOptions::new(Method::POST, "/api/v1/auth/verify-email").json(json!({})))
         .await;
   assert_eq!(verify_response.status, StatusCode::BAD_REQUEST);
   assert_eq!(verify_response.body["data"]["token"], "Token không được để trống");

   let reset_response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/reset-password")
         .json(json!({ "token": "abc", "newPassword": "ngan" })),
   )
   .await;
   assert_eq!(reset_response.body["data"]["newPassword"], "Mật khẩu phải từ 8 đến 72 ký tự");
}

#[tokio::test]
async fn refresh_without_cookie_is_rejected_and_clears_both_session_cookies() {
   let test_app: TestApp = app();
   let response: JsonResponse =
      send(test_app.router.clone(), RequestOptions::new(Method::POST, "/api/v1/auth/refresh-token").csrf_guard()).await;
   assert_eq!(response.status, StatusCode::UNAUTHORIZED);
   assert_eq!(response.body["errorCode"], "INVALID_REFRESH_TOKEN");
   assert_eq!(response.set_cookies.len(), 2);
   assert!(response.set_cookies.iter().all(|set_cookie| set_cookie.contains("Max-Age=0")));
}

#[tokio::test]
async fn protected_endpoints_require_authentication() {
   let test_app: TestApp = app();
   let protected: [(Method, &str); 6] = [
      (Method::GET, "/api/v1/users/me"),
      (Method::PUT, "/api/v1/users/me"),
      (Method::PUT, "/api/v1/users/me/password"),
      (Method::POST, "/api/v1/auth/logout"),
      (Method::POST, "/api/v1/auth/logout-all"),
      (Method::POST, "/api/v1/auth/resend-verification"),
   ];
   for (method, uri) in protected {
      let response: JsonResponse = send(test_app.router.clone(), RequestOptions::new(method.clone(), uri)).await;
      assert_eq!(response.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
      assert_eq!(response.body["errorCode"], "UNAUTHORIZED", "{method} {uri}");
      assert_eq!(response.body["message"], "Phiên đăng nhập không hợp lệ hoặc đã hết hạn");
   }
}

#[tokio::test]
async fn expired_or_forged_access_tokens_are_rejected() {
   let test_app: TestApp = app();
   let expired_token: String = test_app
      .app_state
      .jwt_service
      .generate_access_token(
         42,
         "a@example.com",
         &["ROLE_CUSTOMER".to_string()],
         Utc::now() - Duration::seconds(1000),
         900,
      )
      .unwrap();
   for access_token in [expired_token.as_str(), "khong.phai.jwt"] {
      let response: JsonResponse =
         send(test_app.router.clone(), RequestOptions::new(Method::GET, "/api/v1/users/me").bearer(access_token)).await;
      assert_eq!(response.status, StatusCode::UNAUTHORIZED);
   }
}

#[tokio::test]
async fn valid_bearer_token_passes_authentication_and_reaches_validation() {
   let test_app: TestApp = app();
   let access_token: String = access_token_for(&test_app, &["ROLE_CUSTOMER"]);
   let response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::PUT, "/api/v1/users/me").bearer(&access_token).json(json!({ "phone": "123" })),
   )
   .await;
   assert_eq!(response.status, StatusCode::BAD_REQUEST);
   assert_eq!(response.body["errorCode"], "VALIDATION_ERROR");
   assert_eq!(response.body["data"]["fullName"], "Họ tên không được để trống");
   assert_eq!(response.body["data"]["phone"], "Số điện thoại không hợp lệ");
}

#[tokio::test]
async fn access_cookie_on_state_changing_request_needs_csrf_guard_header() {
   let test_app: TestApp = app();
   let access_token: String = access_token_for(&test_app, &["ROLE_CUSTOMER"]);
   let cookie: String = format!("access_token={access_token}");

   let without_guard: JsonResponse =
      send(test_app.router.clone(), RequestOptions::new(Method::POST, "/api/v1/auth/logout").cookie(&cookie)).await;
   assert_eq!(without_guard.status, StatusCode::UNAUTHORIZED);

   let with_guard: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/logout").cookie(&cookie).csrf_guard(),
   )
   .await;
   assert_eq!(with_guard.status, StatusCode::OK);
   assert_eq!(with_guard.body["message"], "Đăng xuất thành công");
   assert_eq!(with_guard.set_cookies.len(), 2);
   assert!(with_guard.set_cookies.iter().all(|set_cookie| set_cookie.contains("Max-Age=0")));
}

#[tokio::test]
async fn unknown_gender_in_profile_update_is_an_invalid_request_body() {
   let test_app: TestApp = app();
   let access_token: String = access_token_for(&test_app, &["ROLE_CUSTOMER"]);
   let response: JsonResponse = send(
      test_app.router.clone(),
      RequestOptions::new(Method::PUT, "/api/v1/users/me")
         .bearer(&access_token)
         .json(json!({ "fullName": "An", "gender": "KHAC" })),
   )
   .await;
   assert_eq!(response.status, StatusCode::BAD_REQUEST);
   assert_eq!(response.body["errorCode"], "INVALID_REQUEST_BODY");
}

#[tokio::test]
async fn wrong_http_method_returns_method_not_allowed_in_api_response_shape() {
   let test_app: TestApp = app();
   let response: JsonResponse =
      send(test_app.router.clone(), RequestOptions::new(Method::GET, "/api/v1/auth/login")).await;
   assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED);
   assert_eq!(response.body["success"], false);
   assert_eq!(response.body["errorCode"], "METHOD_NOT_ALLOWED");
   assert_eq!(response.body["message"], "Phương thức HTTP không được hỗ trợ");
}
