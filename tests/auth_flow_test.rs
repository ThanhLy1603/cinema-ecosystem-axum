mod common;

use axum::http::{Method, StatusCode};
use cinema_ecosystem_axum::modules::auth::delete_expired_tokens;
use common::{send, test_app, JsonResponse, RequestOptions, TestApp};
use serde_json::json;

const PASSWORD: &str = "matkhau123";
const IGNORE_REASON: &str = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL";
const TOKEN_HASH_SQL: &str = "encode(sha256(convert_to($1, 'UTF8')), 'hex')";

fn database_url() -> String {
   std::env::var("DATABASE_URL").expect("DATABASE_URL")
}

fn redis_url() -> String {
   std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

fn app() -> TestApp {
   test_app(&database_url(), &redis_url())
}

fn unique_email(prefix: &str) -> String {
   format!("{prefix}-{}@example.com", uuid::Uuid::new_v4().simple())
}

fn unique_phone() -> String {
   format!("09{:08}", uuid::Uuid::new_v4().as_u128() % 100_000_000)
}

fn cookie_value(response: &JsonResponse, name: &str) -> String {
   let prefix: String = format!("{name}=");
   response
      .set_cookies
      .iter()
      .find_map(|set_cookie| set_cookie.strip_prefix(&prefix).map(|rest| rest.split(';').next().unwrap().to_string()))
      .unwrap_or_else(|| panic!("không có cookie {name}"))
}

fn session_cookies(response: &JsonResponse) -> String {
   format!(
      "access_token={}; refresh_token={}",
      cookie_value(response, "access_token"),
      cookie_value(response, "refresh_token")
   )
}

fn refresh_cookie(response: &JsonResponse) -> String {
   format!("refresh_token={}", cookie_value(response, "refresh_token"))
}

fn assert_cleared_cookies(response: &JsonResponse) {
   assert_eq!(response.set_cookies.len(), 2, "{:?}", response.set_cookies);
   assert!(response.set_cookies.iter().all(|set_cookie| set_cookie.contains("Max-Age=0")));
}

async fn post_json(app: &TestApp, uri: &str, body: serde_json::Value) -> JsonResponse {
   send(app.router.clone(), RequestOptions::new(Method::POST, uri).json(body).csrf_guard()).await
}

async fn register(app: &TestApp, email: &str, phone: Option<&str>) -> JsonResponse {
   let mut body: serde_json::Value = json!({ "email": email, "password": PASSWORD, "fullName": "Nguyễn Văn An" });
   if let Some(phone) = phone {
      body["phone"] = json!(phone);
   }
   post_json(app, "/api/v1/auth/register", body).await
}

async fn login(app: &TestApp, email: &str, password: &str) -> JsonResponse {
   post_json(app, "/api/v1/auth/login", json!({ "email": email, "password": password })).await
}

async fn registered_and_logged_in(app: &TestApp) -> (String, JsonResponse) {
   let email: String = unique_email("flow");
   assert_eq!(register(app, &email, None).await.status, StatusCode::CREATED);
   let login_response: JsonResponse = login(app, &email, PASSWORD).await;
   assert_eq!(login_response.status, StatusCode::OK);
   (email, login_response)
}

async fn refresh_with(app: &TestApp, cookie: &str) -> JsonResponse {
   send(app.router.clone(), RequestOptions::new(Method::POST, "/api/v1/auth/refresh-token").cookie(cookie).csrf_guard())
      .await
}

async fn get_profile(app: &TestApp, cookie: &str) -> JsonResponse {
   send(app.router.clone(), RequestOptions::new(Method::GET, "/api/v1/users/me").cookie(cookie)).await
}

fn latest_mail_token(app: &TestApp, email: &str, path: &str) -> String {
   let outbox = app.outbox.lock().unwrap();
   let mail = outbox
      .iter()
      .rev()
      .find(|mail| mail.to == email && mail.body.contains(path))
      .unwrap_or_else(|| panic!("không có email {path} gửi tới {email}"));
   let after_token: &str = mail.body.split("token=").nth(1).expect("link có token");
   after_token.split_whitespace().next().unwrap().to_string()
}

fn mail_count(app: &TestApp, email: &str) -> usize {
   app.outbox.lock().unwrap().iter().filter(|mail| mail.to == email).count()
}

async fn set_user_column(app: &TestApp, email: &str, set_clause: &str) {
   sqlx::query(&format!("UPDATE users SET {set_clause} WHERE email = $1"))
      .bind(email)
      .execute(&app.app_state.pg_pool)
      .await
      .unwrap();
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn register_creates_customer_with_lowercase_email_and_sends_verification_mail() {
   let _ = IGNORE_REASON;
   let app: TestApp = app();
   let email: String = unique_email("register");
   let phone: String = unique_phone();
   let mixed_case_email: String = email.to_uppercase();

   let response: JsonResponse = register(&app, &mixed_case_email, Some(&phone)).await;

   assert_eq!(response.status, StatusCode::CREATED);
   assert_eq!(response.body["success"], true);
   assert_eq!(response.body["message"], "Đăng ký thành công");
   let user = &response.body["data"];
   assert_eq!(user["email"], email.to_lowercase());
   assert_eq!(user["fullName"], "Nguyễn Văn An");
   assert_eq!(user["phone"], phone);
   assert_eq!(user["roles"], json!(["ROLE_CUSTOMER"]));
   assert_eq!(user["loyaltyPoints"], 0);
   assert_eq!(user["tier"], "STAR");
   assert_eq!(user["emailVerified"], false);
   assert!(user.get("password").is_none() && user.get("passwordHash").is_none());
   assert!(response.set_cookies.is_empty());

   let stored_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE email = $1")
      .bind(email.to_lowercase())
      .fetch_one(&app.app_state.pg_pool)
      .await
      .unwrap();
   assert!(stored_hash.starts_with("$2") && stored_hash != PASSWORD);

   let token: String = latest_mail_token(&app, &email.to_lowercase(), "/verify-email");
   assert_eq!(token.len(), 43);
   assert_eq!(mail_count(&app, &email.to_lowercase()), 1);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn register_rejects_duplicate_email_and_phone() {
   let app: TestApp = app();
   let email: String = unique_email("dup");
   let phone: String = unique_phone();
   assert_eq!(register(&app, &email, Some(&phone)).await.status, StatusCode::CREATED);

   let same_email: JsonResponse = register(&app, &email.to_uppercase(), None).await;
   assert_eq!(same_email.status, StatusCode::CONFLICT);
   assert_eq!(same_email.body["errorCode"], "EMAIL_ALREADY_EXISTS");
   assert_eq!(same_email.body["message"], "Email đã được sử dụng");

   let same_phone: JsonResponse = register(&app, &unique_email("dup2"), Some(&phone)).await;
   assert_eq!(same_phone.status, StatusCode::CONFLICT);
   assert_eq!(same_phone.body["errorCode"], "PHONE_ALREADY_EXISTS");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn login_sets_httponly_cookies_and_never_returns_tokens_in_body() {
   let app: TestApp = app();
   let (email, login_response) = registered_and_logged_in(&app).await;

   assert_eq!(login_response.body["message"], "Đăng nhập thành công");
   assert_eq!(login_response.body["data"]["user"]["email"], email);
   let body_text: String = login_response.body.to_string();
   assert!(!body_text.contains("accessToken") && !body_text.contains("refreshToken"));
   assert_eq!(login_response.body["data"].as_object().unwrap().len(), 1);

   assert_eq!(login_response.set_cookies.len(), 2);
   let refresh_set_cookie: &String =
      login_response.set_cookies.iter().find(|c| c.starts_with("refresh_token=")).unwrap();
   assert!(refresh_set_cookie.contains("HttpOnly") && refresh_set_cookie.contains("SameSite=Strict"));
   assert!(refresh_set_cookie.contains("Path=/api/v1/auth") && refresh_set_cookie.contains("Max-Age=604800"));
   let access_set_cookie: &String = login_response.set_cookies.iter().find(|c| c.starts_with("access_token=")).unwrap();
   assert!(access_set_cookie.contains("HttpOnly") && access_set_cookie.contains("Max-Age=900"));

   let last_login_set: bool = sqlx::query_scalar("SELECT last_login_at IS NOT NULL FROM users WHERE email = $1")
      .bind(&email)
      .fetch_one(&app.app_state.pg_pool)
      .await
      .unwrap();
   assert!(last_login_set);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn login_is_case_insensitive_on_email_and_hides_which_part_was_wrong() {
   let app: TestApp = app();
   let (email, _) = registered_and_logged_in(&app).await;

   assert_eq!(login(&app, &email.to_uppercase(), PASSWORD).await.status, StatusCode::OK);

   let wrong_password: JsonResponse = login(&app, &email, "sai-mat-khau-1").await;
   let unknown_email: JsonResponse = login(&app, &unique_email("ghost"), PASSWORD).await;
   for response in [&wrong_password, &unknown_email] {
      assert_eq!(response.status, StatusCode::UNAUTHORIZED);
      assert_eq!(response.body["errorCode"], "BAD_CREDENTIALS");
      assert_eq!(response.body["message"], "Email hoặc mật khẩu không chính xác");
      assert!(response.set_cookies.is_empty());
   }
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn profile_is_readable_with_cookie_or_bearer_token() {
   let app: TestApp = app();
   let (email, login_response) = registered_and_logged_in(&app).await;

   let by_cookie: JsonResponse = get_profile(&app, &session_cookies(&login_response)).await;
   assert_eq!(by_cookie.status, StatusCode::OK);
   let profile = &by_cookie.body["data"];
   assert_eq!(profile["email"], email);
   assert_eq!(profile["hasPassword"], true);
   assert_eq!(profile["linkedProviders"], json!([]));
   assert_eq!(profile["branches"], json!([]));
   assert_eq!(profile["roles"], json!(["ROLE_CUSTOMER"]));
   assert!(profile.get("dateOfBirth").is_none() && profile.get("gender").is_none());

   let access_token: String = cookie_value(&login_response, "access_token");
   let by_bearer: JsonResponse =
      send(app.router.clone(), RequestOptions::new(Method::GET, "/api/v1/users/me").bearer(&access_token)).await;
   assert_eq!(by_bearer.status, StatusCode::OK);
   assert_eq!(by_bearer.body["data"]["email"], email);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn fifth_wrong_password_locks_account_and_lock_expires_after_deadline() {
   let app: TestApp = app();
   let (email, _) = registered_and_logged_in(&app).await;

   for attempt in 1..=5 {
      let response: JsonResponse = login(&app, &email, "sai-mat-khau-1").await;
      assert_eq!(response.status, StatusCode::UNAUTHORIZED, "lần {attempt}");
      assert_eq!(response.body["errorCode"], "BAD_CREDENTIALS");
   }

   let locked: JsonResponse = login(&app, &email, PASSWORD).await;
   assert_eq!(locked.status, StatusCode::LOCKED);
   assert_eq!(locked.body["errorCode"], "ACCOUNT_LOCKED");

   set_user_column(&app, &email, "locked_until = now() - interval '1 second'").await;
   let unlocked: JsonResponse = login(&app, &email, PASSWORD).await;
   assert_eq!(unlocked.status, StatusCode::OK);

   let (status, failed_attempts): (String, i32) =
      sqlx::query_as("SELECT status, failed_login_attempts FROM users WHERE email = $1")
         .bind(&email)
         .fetch_one(&app.app_state.pg_pool)
         .await
         .unwrap();
   assert_eq!((status.as_str(), failed_attempts), ("ACTIVE", 0));
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn suspended_account_cannot_login_refresh_or_read_profile() {
   let app: TestApp = app();
   let (email, login_response) = registered_and_logged_in(&app).await;
   set_user_column(&app, &email, "status = 'SUSPENDED'").await;

   let login_attempt: JsonResponse = login(&app, &email, PASSWORD).await;
   assert_eq!(login_attempt.status, StatusCode::FORBIDDEN);
   assert_eq!(login_attempt.body["errorCode"], "ACCOUNT_DISABLED");

   let profile: JsonResponse = get_profile(&app, &session_cookies(&login_response)).await;
   assert_eq!(profile.status, StatusCode::FORBIDDEN);
   assert_eq!(profile.body["errorCode"], "ACCOUNT_DISABLED");

   let refresh: JsonResponse = refresh_with(&app, &refresh_cookie(&login_response)).await;
   assert_eq!(refresh.status, StatusCode::UNAUTHORIZED);
   assert_eq!(refresh.body["errorCode"], "ACCOUNT_DISABLED");
   assert_cleared_cookies(&refresh);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn deleted_account_looks_like_unknown_credentials() {
   let app: TestApp = app();
   let (email, _) = registered_and_logged_in(&app).await;
   set_user_column(&app, &email, "status = 'DELETED'").await;

   let response: JsonResponse = login(&app, &email, PASSWORD).await;
   assert_eq!(response.status, StatusCode::UNAUTHORIZED);
   assert_eq!(response.body["errorCode"], "BAD_CREDENTIALS");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn refresh_rotates_tokens_and_revokes_family_when_old_token_is_replayed() {
   let app: TestApp = app();
   let (_, login_response) = registered_and_logged_in(&app).await;
   let old_cookie: String = refresh_cookie(&login_response);

   let rotated: JsonResponse = refresh_with(&app, &old_cookie).await;
   assert_eq!(rotated.status, StatusCode::OK);
   assert_eq!(rotated.body["message"], "Thành công");
   assert!(rotated.body["data"]["user"]["email"].is_string());
   assert_ne!(cookie_value(&rotated, "refresh_token"), cookie_value(&login_response, "refresh_token"));
   let new_cookie: String = refresh_cookie(&rotated);

   let replay_in_grace_period: JsonResponse = refresh_with(&app, &old_cookie).await;
   assert_eq!(replay_in_grace_period.status, StatusCode::UNAUTHORIZED);
   assert_eq!(replay_in_grace_period.body["errorCode"], "REFRESH_IN_PROGRESS");
   assert!(replay_in_grace_period.set_cookies.is_empty());

   sqlx::query(&format!(
      "UPDATE refresh_tokens SET revoked_at = revoked_at - interval '1 minute' WHERE token_hash = {TOKEN_HASH_SQL}"
   ))
   .bind(cookie_value(&login_response, "refresh_token"))
   .execute(&app.app_state.pg_pool)
   .await
   .unwrap();

   let reuse: JsonResponse = refresh_with(&app, &old_cookie).await;
   assert_eq!(reuse.status, StatusCode::UNAUTHORIZED);
   assert_eq!(reuse.body["errorCode"], "REFRESH_TOKEN_REUSED");
   assert_cleared_cookies(&reuse);

   let descendant: JsonResponse = refresh_with(&app, &new_cookie).await;
   assert_eq!(descendant.status, StatusCode::UNAUTHORIZED);
   assert_eq!(descendant.body["errorCode"], "REFRESH_TOKEN_REUSED");

   let active_tokens_left: i64 = sqlx::query_scalar(&format!(
      "SELECT count(*) FROM refresh_tokens WHERE revoked = FALSE AND family_id = \
       (SELECT family_id FROM refresh_tokens WHERE token_hash = {TOKEN_HASH_SQL})"
   ))
   .bind(cookie_value(&login_response, "refresh_token"))
   .fetch_one(&app.app_state.pg_pool)
   .await
   .unwrap();
   assert_eq!(active_tokens_left, 0);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn concurrent_refresh_with_the_same_cookie_succeeds_exactly_once() {
   let app: TestApp = app();
   let (_, login_response) = registered_and_logged_in(&app).await;
   let cookie: String = refresh_cookie(&login_response);

   let (first, second) = tokio::join!(refresh_with(&app, &cookie), refresh_with(&app, &cookie));

   let mut outcomes: Vec<(StatusCode, String)> = vec![
      (first.status, first.body["errorCode"].as_str().unwrap_or("").to_string()),
      (second.status, second.body["errorCode"].as_str().unwrap_or("").to_string()),
   ];
   outcomes.sort();
   assert_eq!(outcomes[0], (StatusCode::OK, String::new()));
   assert_eq!(outcomes[1], (StatusCode::UNAUTHORIZED, "REFRESH_IN_PROGRESS".to_string()));
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn expired_and_unknown_refresh_tokens_are_invalid() {
   let app: TestApp = app();
   let (_, login_response) = registered_and_logged_in(&app).await;
   sqlx::query(&format!(
      "UPDATE refresh_tokens SET expires_at = now() - interval '1 hour' WHERE token_hash = {TOKEN_HASH_SQL}"
   ))
   .bind(cookie_value(&login_response, "refresh_token"))
   .execute(&app.app_state.pg_pool)
   .await
   .unwrap();

   let expired: JsonResponse = refresh_with(&app, &refresh_cookie(&login_response)).await;
   assert_eq!(expired.status, StatusCode::UNAUTHORIZED);
   assert_eq!(expired.body["errorCode"], "INVALID_REFRESH_TOKEN");

   let unknown: JsonResponse = refresh_with(&app, "refresh_token=khong-ton-tai").await;
   assert_eq!(unknown.body["errorCode"], "INVALID_REFRESH_TOKEN");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn logout_revokes_only_current_device_and_logout_all_revokes_every_device() {
   let app: TestApp = app();
   let (email, first_device) = registered_and_logged_in(&app).await;
   let second_device: JsonResponse = login(&app, &email, PASSWORD).await;

   let logout: JsonResponse = send(
      app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/logout").cookie(&session_cookies(&first_device)).csrf_guard(),
   )
   .await;
   assert_eq!(logout.status, StatusCode::OK);
   assert_eq!(logout.body["message"], "Đăng xuất thành công");
   assert_cleared_cookies(&logout);

   assert_eq!(refresh_with(&app, &refresh_cookie(&first_device)).await.status, StatusCode::UNAUTHORIZED);
   let second_refresh: JsonResponse = refresh_with(&app, &refresh_cookie(&second_device)).await;
   assert_eq!(second_refresh.status, StatusCode::OK);

   let logout_all: JsonResponse = send(
      app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/logout-all")
         .cookie(&session_cookies(&second_refresh))
         .csrf_guard(),
   )
   .await;
   assert_eq!(logout_all.status, StatusCode::OK);
   assert_eq!(logout_all.body["message"], "Đã đăng xuất khỏi tất cả thiết bị");
   assert_cleared_cookies(&logout_all);
   assert_eq!(refresh_with(&app, &refresh_cookie(&second_refresh)).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn email_verification_token_is_single_use_and_resend_invalidates_the_previous_one() {
   let app: TestApp = app();
   let email: String = unique_email("verify");
   assert_eq!(register(&app, &email, None).await.status, StatusCode::CREATED);
   let first_token: String = latest_mail_token(&app, &email, "/verify-email");
   let login_response: JsonResponse = login(&app, &email, PASSWORD).await;

   let resend: JsonResponse = send(
      app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/resend-verification")
         .cookie(&session_cookies(&login_response))
         .csrf_guard(),
   )
   .await;
   assert_eq!(resend.status, StatusCode::OK);
   assert_eq!(resend.body["message"], "Đã gửi lại email xác thực");
   let second_token: String = latest_mail_token(&app, &email, "/verify-email");
   assert_ne!(first_token, second_token);

   let stale: JsonResponse = post_json(&app, "/api/v1/auth/verify-email", json!({ "token": first_token })).await;
   assert_eq!(stale.status, StatusCode::BAD_REQUEST);
   assert_eq!(stale.body["errorCode"], "INVALID_OR_EXPIRED_TOKEN");

   let verified: JsonResponse = post_json(&app, "/api/v1/auth/verify-email", json!({ "token": second_token })).await;
   assert_eq!(verified.status, StatusCode::OK);
   assert_eq!(verified.body["message"], "Xác thực email thành công");
   assert_eq!(get_profile(&app, &session_cookies(&login_response)).await.body["data"]["emailVerified"], true);

   let reused: JsonResponse = post_json(&app, "/api/v1/auth/verify-email", json!({ "token": second_token })).await;
   assert_eq!(reused.body["errorCode"], "INVALID_OR_EXPIRED_TOKEN");

   let resend_when_verified: JsonResponse = send(
      app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/resend-verification")
         .cookie(&session_cookies(&login_response))
         .csrf_guard(),
   )
   .await;
   assert_eq!(resend_when_verified.status, StatusCode::CONFLICT);
   assert_eq!(resend_when_verified.body["errorCode"], "EMAIL_ALREADY_VERIFIED");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn resend_verification_is_limited_to_three_per_hour() {
   let app: TestApp = app();
   let (_, login_response) = registered_and_logged_in(&app).await;
   let cookie: String = session_cookies(&login_response);
   let resend = |app: &TestApp| {
      send(
         app.router.clone(),
         RequestOptions::new(Method::POST, "/api/v1/auth/resend-verification").cookie(&cookie).csrf_guard(),
      )
   };

   for _ in 0..3 {
      assert_eq!(resend(&app).await.status, StatusCode::OK);
   }
   let limited: JsonResponse = resend(&app).await;
   assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
   assert_eq!(limited.body["errorCode"], "TOO_MANY_REQUESTS");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn forgot_password_answers_the_same_for_unknown_email_and_sends_mail_only_to_real_accounts() {
   let app: TestApp = app();
   let (email, _) = registered_and_logged_in(&app).await;
   let mails_before: usize = mail_count(&app, &email);

   let known: JsonResponse = post_json(&app, "/api/v1/auth/forgot-password", json!({ "email": email })).await;
   let ghost_email: String = unique_email("ghost");
   let unknown: JsonResponse = post_json(&app, "/api/v1/auth/forgot-password", json!({ "email": ghost_email })).await;

   for response in [&known, &unknown] {
      assert_eq!(response.status, StatusCode::OK);
      assert_eq!(
         response.body["message"],
         "Nếu email tồn tại trong hệ thống, chúng tôi đã gửi hướng dẫn đặt lại mật khẩu"
      );
   }
   assert_eq!(mail_count(&app, &email), mails_before + 1);
   assert_eq!(mail_count(&app, &ghost_email), 0);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn password_reset_changes_password_unlocks_account_and_revokes_all_sessions() {
   let app: TestApp = app();
   let (email, login_response) = registered_and_logged_in(&app).await;
   set_user_column(
      &app,
      &email,
      "status = 'LOCKED', locked_until = now() + interval '10 minutes', failed_login_attempts = 5",
   )
   .await;
   post_json(&app, "/api/v1/auth/forgot-password", json!({ "email": email })).await;
   let token: String = latest_mail_token(&app, &email, "/reset-password");

   let weak: JsonResponse =
      post_json(&app, "/api/v1/auth/reset-password", json!({ "token": token, "newPassword": "chicochu" })).await;
   assert_eq!(weak.status, StatusCode::BAD_REQUEST);
   assert_eq!(weak.body["data"]["newPassword"], "Mật khẩu phải gồm cả chữ và số");

   let reset: JsonResponse =
      post_json(&app, "/api/v1/auth/reset-password", json!({ "token": token, "newPassword": "matkhaumoi456" })).await;
   assert_eq!(reset.status, StatusCode::OK);
   assert_eq!(reset.body["message"], "Đặt lại mật khẩu thành công, vui lòng đăng nhập lại");

   assert_eq!(login(&app, &email, PASSWORD).await.status, StatusCode::UNAUTHORIZED);
   assert_eq!(login(&app, &email, "matkhaumoi456").await.status, StatusCode::OK);
   assert_eq!(refresh_with(&app, &refresh_cookie(&login_response)).await.status, StatusCode::UNAUTHORIZED);

   let reused: JsonResponse =
      post_json(&app, "/api/v1/auth/reset-password", json!({ "token": token, "newPassword": "matkhaukhac789" })).await;
   assert_eq!(reused.status, StatusCode::BAD_REQUEST);
   assert_eq!(reused.body["errorCode"], "INVALID_OR_EXPIRED_TOKEN");

   let email_verified: bool = sqlx::query_scalar("SELECT email_verified FROM users WHERE email = $1")
      .bind(&email)
      .fetch_one(&app.app_state.pg_pool)
      .await
      .unwrap();
   assert!(email_verified);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn expired_password_reset_token_is_rejected() {
   let app: TestApp = app();
   let (email, _) = registered_and_logged_in(&app).await;
   post_json(&app, "/api/v1/auth/forgot-password", json!({ "email": email })).await;
   let token: String = latest_mail_token(&app, &email, "/reset-password");
   sqlx::query(&format!(
      "UPDATE user_tokens SET expires_at = now() - interval '1 minute' WHERE token_hash = {TOKEN_HASH_SQL}"
   ))
   .bind(&token)
   .execute(&app.app_state.pg_pool)
   .await
   .unwrap();

   let response: JsonResponse =
      post_json(&app, "/api/v1/auth/reset-password", json!({ "token": token, "newPassword": "matkhaumoi456" })).await;
   assert_eq!(response.status, StatusCode::BAD_REQUEST);
   assert_eq!(response.body["errorCode"], "INVALID_OR_EXPIRED_TOKEN");
   assert_eq!(login(&app, &email, PASSWORD).await.status, StatusCode::OK);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn forgot_password_sends_at_most_three_mails_per_hour_per_account() {
   let app: TestApp = app();
   let (email, _) = registered_and_logged_in(&app).await;
   let mails_before: usize = mail_count(&app, &email);

   for _ in 0..5 {
      let response: JsonResponse = post_json(&app, "/api/v1/auth/forgot-password", json!({ "email": email })).await;
      assert_eq!(response.status, StatusCode::OK);
   }

   assert_eq!(mail_count(&app, &email), mails_before + 3);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn change_password_validates_current_password_and_rotates_every_session() {
   let app: TestApp = app();
   let (email, login_response) = registered_and_logged_in(&app).await;
   let cookie: String = session_cookies(&login_response);
   let change = |body: serde_json::Value| {
      send(
         app.router.clone(),
         RequestOptions::new(Method::PUT, "/api/v1/users/me/password").cookie(&cookie).csrf_guard().json(body),
      )
   };

   let missing_current: JsonResponse = change(json!({ "newPassword": "matkhaumoi456" })).await;
   assert_eq!(missing_current.status, StatusCode::BAD_REQUEST);
   assert_eq!(missing_current.body["errorCode"], "CURRENT_PASSWORD_REQUIRED");

   let wrong_current: JsonResponse =
      change(json!({ "currentPassword": "sai-mat-khau-1", "newPassword": "matkhaumoi456" })).await;
   assert_eq!(wrong_current.body["errorCode"], "INVALID_CURRENT_PASSWORD");

   let same_password: JsonResponse = change(json!({ "currentPassword": PASSWORD, "newPassword": PASSWORD })).await;
   assert_eq!(same_password.body["errorCode"], "NEW_PASSWORD_SAME_AS_CURRENT");

   let changed: JsonResponse = change(json!({ "currentPassword": PASSWORD, "newPassword": "matkhaumoi456" })).await;
   assert_eq!(changed.status, StatusCode::OK);
   assert_eq!(changed.body["message"], "Đổi mật khẩu thành công");
   assert_eq!(changed.body["data"]["user"]["email"], email);
   assert_eq!(changed.set_cookies.len(), 2);

   assert_eq!(refresh_with(&app, &refresh_cookie(&login_response)).await.status, StatusCode::UNAUTHORIZED);
   assert_eq!(refresh_with(&app, &refresh_cookie(&changed)).await.status, StatusCode::OK);
   assert_eq!(login(&app, &email, PASSWORD).await.status, StatusCode::UNAUTHORIZED);
   assert_eq!(login(&app, &email, "matkhaumoi456").await.status, StatusCode::OK);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn repeated_wrong_current_password_locks_the_account() {
   let app: TestApp = app();
   let (email, login_response) = registered_and_logged_in(&app).await;
   let cookie: String = session_cookies(&login_response);

   for _ in 0..5 {
      let response: JsonResponse = send(
         app.router.clone(),
         RequestOptions::new(Method::PUT, "/api/v1/users/me/password")
            .cookie(&cookie)
            .csrf_guard()
            .json(json!({ "currentPassword": "sai-mat-khau-1", "newPassword": "matkhaumoi456" })),
      )
      .await;
      assert_eq!(response.body["errorCode"], "INVALID_CURRENT_PASSWORD");
   }

   let locked: JsonResponse = login(&app, &email, PASSWORD).await;
   assert_eq!(locked.status, StatusCode::LOCKED);
   assert_eq!(locked.body["errorCode"], "ACCOUNT_LOCKED");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn update_profile_persists_fields_and_rejects_phone_owned_by_someone_else() {
   let app: TestApp = app();
   let other_phone: String = unique_phone();
   assert_eq!(register(&app, &unique_email("other"), Some(&other_phone)).await.status, StatusCode::CREATED);
   let (_, login_response) = registered_and_logged_in(&app).await;
   let cookie: String = session_cookies(&login_response);
   let update = |body: serde_json::Value| {
      send(
         app.router.clone(),
         RequestOptions::new(Method::PUT, "/api/v1/users/me").cookie(&cookie).csrf_guard().json(body),
      )
   };
   let my_phone: String = unique_phone();

   let updated: JsonResponse = update(json!({
      "fullName": "  Trần Thị Bình ",
      "phone": my_phone,
      "dateOfBirth": "1999-12-31",
      "gender": "FEMALE",
      "avatarUrl": "https://cdn.example.com/a.png"
   }))
   .await;
   assert_eq!(updated.status, StatusCode::OK);
   assert_eq!(updated.body["message"], "Cập nhật hồ sơ thành công");
   let profile = &updated.body["data"];
   assert_eq!(profile["fullName"], "Trần Thị Bình");
   assert_eq!(profile["phone"], my_phone);
   assert_eq!(profile["dateOfBirth"], "1999-12-31");
   assert_eq!(profile["gender"], "FEMALE");
   assert_eq!(profile["avatarUrl"], "https://cdn.example.com/a.png");

   let stolen_phone: JsonResponse = update(json!({ "fullName": "An", "phone": other_phone })).await;
   assert_eq!(stolen_phone.status, StatusCode::CONFLICT);
   assert_eq!(stolen_phone.body["errorCode"], "PHONE_ALREADY_EXISTS");

   let own_phone_again: JsonResponse = update(json!({ "fullName": "An", "phone": my_phone })).await;
   assert_eq!(own_phone_again.status, StatusCode::OK);

   let cleared: JsonResponse = update(json!({ "fullName": "An" })).await;
   assert_eq!(cleared.status, StatusCode::OK);
   assert!(cleared.body["data"].get("phone").is_none());
   assert!(cleared.body["data"].get("gender").is_none());
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn cleanup_removes_tokens_expired_for_more_than_a_day_only() {
   let app: TestApp = app();
   let (_, first_login) = registered_and_logged_in(&app).await;
   let (_, second_login) = registered_and_logged_in(&app).await;
   let ancient_token: String = cookie_value(&first_login, "refresh_token");
   let recent_token: String = cookie_value(&second_login, "refresh_token");
   for (token, interval) in [(&ancient_token, "2 days"), (&recent_token, "1 hour")] {
      sqlx::query(&format!(
         "UPDATE refresh_tokens SET expires_at = now() - interval '{interval}' WHERE token_hash = {TOKEN_HASH_SQL}"
      ))
      .bind(token)
      .execute(&app.app_state.pg_pool)
      .await
      .unwrap();
   }

   let (deleted_refresh_tokens, _) = delete_expired_tokens(&app.app_state.pg_pool).await.unwrap();

   assert!(deleted_refresh_tokens >= 1);
   let remaining = |token: String| {
      let pg_pool = app.app_state.pg_pool.clone();
      async move {
         sqlx::query_scalar::<_, i64>(&format!(
            "SELECT count(*) FROM refresh_tokens WHERE token_hash = {TOKEN_HASH_SQL}"
         ))
         .bind(token)
         .fetch_one(&pg_pool)
         .await
         .unwrap()
      }
   };
   assert_eq!(remaining(ancient_token).await, 0);
   assert_eq!(remaining(recent_token).await, 1);
}
