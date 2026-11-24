#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use cinema_ecosystem_axum::app::build_router;
use cinema_ecosystem_axum::config::{AppConfig, JwtConfig, MailConfig};
use cinema_ecosystem_axum::infrastructure::mailer::{Mailer, OutgoingMail};
use cinema_ecosystem_axum::infrastructure::{database, redis};
use cinema_ecosystem_axum::modules::auth::GoogleIdTokenVerifier;
use cinema_ecosystem_axum::state::AppState;
use http_body_util::BodyExt;
use tower::ServiceExt;

pub const UNREACHABLE_DATABASE_URL: &str = "postgres://nobody@127.0.0.1:1/none";
pub const TEST_JWT_SECRET: &str = "test-secret-test-secret-test-secret-1234";
pub const TEST_GOOGLE_CLIENT_ID: &str = "test-client.apps.googleusercontent.com";
pub const UNREACHABLE_REDIS_URL: &str = "redis://127.0.0.1:1";

pub fn test_config(database_url: &str, redis_url: &str) -> AppConfig {
   AppConfig {
      server_port: 0,
      database_url: database_url.to_string(),
      redis_url: redis_url.to_string(),
      cors_allowed_origins: vec!["http://localhost:3000".to_string()],
      jwt: JwtConfig {
         secret: TEST_JWT_SECRET.to_string(),
         access_token_ttl_seconds: 900,
         refresh_token_ttl_seconds: 604800,
      },
      cookie_secure: false,
      bcrypt_cost: 4,
      frontend_base_url: "http://localhost:5173".to_string(),
      google_client_id: TEST_GOOGLE_CLIENT_ID.to_string(),
      mail: MailConfig {
         host: "localhost".to_string(),
         port: 1025,
         username: String::new(),
         password: String::new(),
         starttls: false,
         from: "CinemaSphere <noreply@cinemasphere.local>".to_string(),
      },
   }
}

pub struct TestApp {
   pub router: Router,
   pub app_state: AppState,
   pub outbox: Arc<Mutex<Vec<OutgoingMail>>>,
}

pub fn test_app(database_url: &str, redis_url: &str) -> TestApp {
   let app_config: AppConfig = test_config(database_url, redis_url);
   let pg_pool = database::create_lazy_pg_pool(database_url).expect("database url");
   let redis_client = redis::create_redis_client(redis_url).expect("redis url");
   let (mailer, outbox) = Mailer::recording();
   let app_state: AppState = AppState::new(app_config, pg_pool, redis_client, mailer).expect("app state");
   let router: Router = build_router(app_state.clone(), &["http://localhost:3000".to_string()]);
   TestApp { router, app_state, outbox }
}

pub fn test_app_with_google_keys(database_url: &str, redis_url: &str, jwk_set: jsonwebtoken::jwk::JwkSet) -> TestApp {
   let app_config: AppConfig = test_config(database_url, redis_url);
   let pg_pool = database::create_lazy_pg_pool(database_url).expect("database url");
   let redis_client = redis::create_redis_client(redis_url).expect("redis url");
   let (mailer, outbox) = Mailer::recording();
   let mut app_state: AppState = AppState::new(app_config, pg_pool, redis_client, mailer).expect("app state");
   app_state.google_id_token_verifier =
      Arc::new(GoogleIdTokenVerifier::with_static_keys(TEST_GOOGLE_CLIENT_ID, jwk_set));
   let router: Router = build_router(app_state.clone(), &["http://localhost:3000".to_string()]);
   TestApp { router, app_state, outbox }
}

pub fn router_for(database_url: &str) -> Router {
   test_app(database_url, UNREACHABLE_REDIS_URL).router
}

pub struct JsonResponse {
   pub status: StatusCode,
   pub body: serde_json::Value,
   pub set_cookies: Vec<String>,
}

pub struct RequestOptions<'a> {
   pub method: Method,
   pub uri: &'a str,
   pub json_body: Option<serde_json::Value>,
   pub raw_body: Option<&'a str>,
   pub cookie: Option<&'a str>,
   pub bearer_token: Option<&'a str>,
   pub csrf_guard: bool,
}

impl<'a> RequestOptions<'a> {
   pub fn new(method: Method, uri: &'a str) -> RequestOptions<'a> {
      RequestOptions {
         method,
         uri,
         json_body: None,
         raw_body: None,
         cookie: None,
         bearer_token: None,
         csrf_guard: false,
      }
   }

   pub fn json(mut self, json_body: serde_json::Value) -> RequestOptions<'a> {
      self.json_body = Some(json_body);
      self
   }

   pub fn raw_body(mut self, raw_body: &'a str) -> RequestOptions<'a> {
      self.raw_body = Some(raw_body);
      self
   }

   pub fn cookie(mut self, cookie: &'a str) -> RequestOptions<'a> {
      self.cookie = Some(cookie);
      self
   }

   pub fn bearer(mut self, bearer_token: &'a str) -> RequestOptions<'a> {
      self.bearer_token = Some(bearer_token);
      self
   }

   pub fn csrf_guard(mut self) -> RequestOptions<'a> {
      self.csrf_guard = true;
      self
   }
}

pub async fn send(router: Router, request_options: RequestOptions<'_>) -> JsonResponse {
   let mut request_builder = Request::builder().method(request_options.method).uri(request_options.uri);
   if let Some(cookie) = request_options.cookie {
      request_builder = request_builder.header(header::COOKIE, cookie);
   }
   if let Some(bearer_token) = request_options.bearer_token {
      request_builder = request_builder.header(header::AUTHORIZATION, format!("Bearer {bearer_token}"));
   }
   if request_options.csrf_guard {
      request_builder = request_builder.header("X-Requested-With", "XMLHttpRequest");
   }
   let request: Request<Body> = match (request_options.json_body, request_options.raw_body) {
      (Some(json_body), _) => request_builder
         .header(header::CONTENT_TYPE, "application/json")
         .body(Body::from(json_body.to_string()))
         .unwrap(),
      (None, Some(raw_body)) => request_builder
         .header(header::CONTENT_TYPE, "application/json")
         .body(Body::from(raw_body.to_string()))
         .unwrap(),
      (None, None) => request_builder.body(Body::empty()).unwrap(),
   };
   let response = router.oneshot(request).await.unwrap();
   let status: StatusCode = response.status();
   let set_cookies: Vec<String> = response
      .headers()
      .get_all(header::SET_COOKIE)
      .iter()
      .map(|header_value| header_value.to_str().unwrap().to_string())
      .collect();
   let bytes = response.into_body().collect().await.unwrap().to_bytes();
   let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
   JsonResponse { status, body, set_cookies }
}

pub async fn get_json(router: Router, uri: &str) -> (StatusCode, serde_json::Value) {
   let json_response: JsonResponse = send(router, RequestOptions::new(Method::GET, uri)).await;
   (json_response.status, json_response.body)
}
