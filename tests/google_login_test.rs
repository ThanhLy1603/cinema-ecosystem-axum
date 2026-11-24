mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::{Method, StatusCode};
use cinema_ecosystem_axum::modules::auth::GoogleIdTokenVerifier;
use common::{
   send, test_app_with_google_keys, JsonResponse, RequestOptions, TestApp, TEST_GOOGLE_CLIENT_ID,
   UNREACHABLE_DATABASE_URL, UNREACHABLE_REDIS_URL,
};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};

const TEST_PRIVATE_KEY: &str = include_str!("fixtures/google_test_rsa_private.pem");
const OTHER_PRIVATE_KEY: &str = include_str!("fixtures/google_other_rsa_private.pem");
const TEST_JWKS: &str = include_str!("fixtures/google_test_jwks.json");
const KEY_ID: &str = "test-key-1";

fn jwk_set() -> JwkSet {
   serde_json::from_str(TEST_JWKS).unwrap()
}

fn now_seconds() -> i64 {
   SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn claims(subject: &str, email: &str) -> Value {
   json!({
      "iss": "https://accounts.google.com",
      "aud": TEST_GOOGLE_CLIENT_ID,
      "sub": subject,
      "email": email,
      "email_verified": true,
      "name": "Nguyễn Thị Google",
      "picture": "https://lh3.googleusercontent.com/a/avatar",
      "iat": now_seconds(),
      "exp": now_seconds() + 3600
   })
}

fn sign_with(private_key: &str, key_id: Option<&str>, claims: &Value) -> String {
   let mut header: Header = Header::new(Algorithm::RS256);
   header.kid = key_id.map(str::to_string);
   encode(&header, claims, &EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap()).unwrap()
}

fn sign(claims: &Value) -> String {
   sign_with(TEST_PRIVATE_KEY, Some(KEY_ID), claims)
}

fn verifier() -> GoogleIdTokenVerifier {
   GoogleIdTokenVerifier::with_static_keys(TEST_GOOGLE_CLIENT_ID, jwk_set())
}

async fn error_code_of(id_token: &str) -> String {
   verifier().verify(id_token).await.unwrap_err().error_code
}

#[tokio::test]
async fn valid_token_yields_the_google_profile() {
   let google_profile = verifier().verify(&sign(&claims("sub-1", "An@Example.com"))).await.unwrap();
   assert_eq!(google_profile.subject, "sub-1");
   assert_eq!(google_profile.email, "An@Example.com");
   assert_eq!(google_profile.full_name.as_deref(), Some("Nguyễn Thị Google"));
   assert_eq!(google_profile.avatar_url.as_deref(), Some("https://lh3.googleusercontent.com/a/avatar"));
}

#[tokio::test]
async fn issuer_without_scheme_and_string_email_verified_are_accepted() {
   let mut token_claims: Value = claims("sub-2", "b@example.com");
   token_claims["iss"] = json!("accounts.google.com");
   token_claims["email_verified"] = json!("true");
   token_claims["aud"] = json!([TEST_GOOGLE_CLIENT_ID, "another-audience"]);
   assert!(verifier().verify(&sign(&token_claims)).await.is_ok());
}

#[tokio::test]
async fn forged_expired_or_misdirected_tokens_are_rejected() {
   assert_eq!(error_code_of("khong-phai-jwt").await, "INVALID_GOOGLE_TOKEN");

   let forged: String = sign_with(OTHER_PRIVATE_KEY, Some(KEY_ID), &claims("sub", "a@example.com"));
   assert_eq!(error_code_of(&forged).await, "INVALID_GOOGLE_TOKEN");

   let unknown_key: String = sign_with(TEST_PRIVATE_KEY, Some("khong-co"), &claims("sub", "a@example.com"));
   assert_eq!(error_code_of(&unknown_key).await, "INVALID_GOOGLE_TOKEN");

   let without_key_id: String = sign_with(TEST_PRIVATE_KEY, None, &claims("sub", "a@example.com"));
   assert_eq!(error_code_of(&without_key_id).await, "INVALID_GOOGLE_TOKEN");

   let mut expired: Value = claims("sub", "a@example.com");
   expired["exp"] = json!(now_seconds() - 600);
   assert_eq!(error_code_of(&sign(&expired)).await, "INVALID_GOOGLE_TOKEN");

   let mut wrong_audience: Value = claims("sub", "a@example.com");
   wrong_audience["aud"] = json!("client-khac");
   assert_eq!(error_code_of(&sign(&wrong_audience)).await, "INVALID_GOOGLE_TOKEN");

   let mut wrong_issuer: Value = claims("sub", "a@example.com");
   wrong_issuer["iss"] = json!("https://evil.example.com");
   assert_eq!(error_code_of(&sign(&wrong_issuer)).await, "INVALID_GOOGLE_TOKEN");

   let mut missing_email: Value = claims("sub", "a@example.com");
   missing_email.as_object_mut().unwrap().remove("email");
   assert_eq!(error_code_of(&sign(&missing_email)).await, "INVALID_GOOGLE_TOKEN");
}

#[tokio::test]
async fn unverified_google_email_is_forbidden() {
   let mut unverified: Value = claims("sub", "a@example.com");
   unverified["email_verified"] = json!(false);
   let app_error = verifier().verify(&sign(&unverified)).await.unwrap_err();
   assert_eq!(app_error.error_code, "GOOGLE_EMAIL_NOT_VERIFIED");
   assert_eq!(app_error.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unconfigured_client_id_disables_google_login() {
   let disabled: GoogleIdTokenVerifier = GoogleIdTokenVerifier::with_static_keys("  ", jwk_set());
   let app_error = disabled.verify(&sign(&claims("sub", "a@example.com"))).await.unwrap_err();
   assert_eq!(app_error.error_code, "GOOGLE_LOGIN_DISABLED");
   assert_eq!(app_error.status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn endpoint_validates_the_body_and_rejects_bad_tokens_before_touching_the_database() {
   let app: TestApp = test_app_with_google_keys(UNREACHABLE_DATABASE_URL, UNREACHABLE_REDIS_URL, jwk_set());
   let empty: JsonResponse =
      send(app.router.clone(), RequestOptions::new(Method::POST, "/api/v1/auth/google").json(json!({})).csrf_guard())
         .await;
   assert_eq!(empty.status, StatusCode::BAD_REQUEST);
   assert_eq!(empty.body["data"]["idToken"], "Google ID Token không được để trống");
   let invalid: JsonResponse = send(
      app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/google").json(json!({ "idToken": "rac" })).csrf_guard(),
   )
   .await;
   assert_eq!(invalid.status, StatusCode::UNAUTHORIZED);
   assert_eq!(invalid.body["errorCode"], "INVALID_GOOGLE_TOKEN");
   assert!(invalid.set_cookies.is_empty());
}

fn database_url() -> String {
   std::env::var("DATABASE_URL").expect("DATABASE_URL")
}

fn redis_url() -> String {
   std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

fn unique_suffix() -> String {
   uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

async fn google_login(app: &TestApp, id_token: &str) -> JsonResponse {
   send(
      app.router.clone(),
      RequestOptions::new(Method::POST, "/api/v1/auth/google").json(json!({ "idToken": id_token })).csrf_guard(),
   )
   .await
}

fn db_app() -> TestApp {
   test_app_with_google_keys(&database_url(), &redis_url(), jwk_set())
}

async fn insert_user(app: &TestApp, email: &str, email_verified: bool, status: &str) -> i64 {
   let user_id: i64 = sqlx::query_scalar::<_, i64>(
      "INSERT INTO users (email, full_name, email_verified, status) VALUES ($1, 'Có Sẵn', $2, $3) RETURNING id",
   )
   .bind(email)
   .bind(email_verified)
   .bind(status)
   .fetch_one(&app.app_state.pg_pool)
   .await
   .unwrap();
   sqlx::query("INSERT INTO user_roles (user_id, role_id) SELECT $1, id FROM roles WHERE code = 'ROLE_CUSTOMER'")
      .bind(user_id)
      .execute(&app.app_state.pg_pool)
      .await
      .unwrap();
   user_id
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn first_google_login_registers_a_verified_customer_and_later_logins_reuse_it() {
   let app: TestApp = db_app();
   let suffix: String = unique_suffix();
   let email: String = format!("Google.{suffix}@Example.com");
   let subject: String = format!("sub-{suffix}");

   let first: JsonResponse = google_login(&app, &sign(&claims(&subject, &email))).await;
   assert_eq!(first.status, StatusCode::OK, "{}", first.body);
   assert_eq!(first.body["message"], "Đăng nhập thành công");
   assert_eq!(first.body["data"]["user"]["email"], email.to_lowercase());
   assert_eq!(first.body["data"]["user"]["fullName"], "Nguyễn Thị Google");
   assert_eq!(first.body["data"]["user"]["roles"], json!(["ROLE_CUSTOMER"]));
   assert_eq!(first.body["data"]["user"]["emailVerified"], true);
   assert!(first.body["data"].get("accessToken").is_none());
   assert!(first.set_cookies.iter().any(|cookie| cookie.starts_with("access_token=") && cookie.contains("HttpOnly")));
   assert!(first.set_cookies.iter().any(|cookie| cookie.starts_with("refresh_token=") && cookie.contains("HttpOnly")));

   let user_id: i64 = first.body["data"]["user"]["id"].as_i64().unwrap();
   let second: JsonResponse = google_login(&app, &sign(&claims(&subject, &email))).await;
   assert_eq!(second.status, StatusCode::OK);
   assert_eq!(second.body["data"]["user"]["id"], user_id);

   let access_cookie: String = first
      .set_cookies
      .iter()
      .find(|cookie| cookie.starts_with("access_token="))
      .map(|cookie| cookie.split(';').next().unwrap().to_string())
      .unwrap();
   let profile: JsonResponse =
      send(app.router.clone(), RequestOptions::new(Method::GET, "/api/v1/users/me").cookie(&access_cookie)).await;
   assert_eq!(profile.body["data"]["linkedProviders"], json!(["GOOGLE"]));
   assert_eq!(profile.body["data"]["avatarUrl"], "https://lh3.googleusercontent.com/a/avatar");

   let identity_count: i64 = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM user_identities WHERE user_id = $1")
      .bind(user_id)
      .fetch_one(&app.app_state.pg_pool)
      .await
      .unwrap();
   assert_eq!(identity_count, 1);
   let has_last_login: bool =
      sqlx::query_scalar::<_, bool>("SELECT last_login_at IS NOT NULL FROM users WHERE id = $1")
         .bind(user_id)
         .fetch_one(&app.app_state.pg_pool)
         .await
         .unwrap();
   assert!(has_last_login);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn google_login_links_a_verified_password_account_once() {
   let app: TestApp = db_app();
   let suffix: String = unique_suffix();
   let email: String = format!("link.{suffix}@example.com");
   let user_id: i64 = insert_user(&app, &email, true, "ACTIVE").await;

   let linked: JsonResponse = google_login(&app, &sign(&claims(&format!("sub-a-{suffix}"), &email))).await;
   assert_eq!(linked.status, StatusCode::OK, "{}", linked.body);
   assert_eq!(linked.body["data"]["user"]["id"], user_id);
   assert_eq!(linked.body["data"]["user"]["fullName"], "Có Sẵn");

   let other_google_account: JsonResponse =
      google_login(&app, &sign(&claims(&format!("sub-b-{suffix}"), &email))).await;
   assert_eq!(other_google_account.status, StatusCode::CONFLICT);
   assert_eq!(other_google_account.body["errorCode"], "GOOGLE_ACCOUNT_MISMATCH");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn google_login_refuses_unverified_deleted_suspended_and_locked_accounts() {
   let app: TestApp = db_app();
   let suffix: String = unique_suffix();
   let cases: [(&str, bool, &str, StatusCode, &str); 4] = [
      ("unverified", false, "ACTIVE", StatusCode::CONFLICT, "ACCOUNT_EXISTS_UNVERIFIED"),
      ("deleted", true, "DELETED", StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"),
      ("suspended", true, "SUSPENDED", StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"),
      ("locked", true, "LOCKED", StatusCode::LOCKED, "ACCOUNT_LOCKED"),
   ];
   for (label, email_verified, status, expected_status, expected_code) in cases {
      let email: String = format!("{label}.{suffix}@example.com");
      let user_id: i64 = insert_user(&app, &email, email_verified, status).await;
      if status == "LOCKED" {
         sqlx::query("UPDATE users SET locked_until = now() + interval '1 hour' WHERE id = $1")
            .bind(user_id)
            .execute(&app.app_state.pg_pool)
            .await
            .unwrap();
      }
      let response: JsonResponse = google_login(&app, &sign(&claims(&format!("sub-{label}-{suffix}"), &email))).await;
      assert_eq!(response.status, expected_status, "{label}: {}", response.body);
      assert_eq!(response.body["errorCode"], expected_code, "{label}");
      assert!(response.set_cookies.is_empty(), "{label}");
   }
}
