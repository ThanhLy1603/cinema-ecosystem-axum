use axum::http::{header, HeaderName, HeaderValue, Method, StatusCode};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::common::error::AppError;
use crate::modules::{auth, cinema, health};
use crate::state::AppState;

pub fn build_router(app_state: AppState, cors_allowed_origins: &[String]) -> Router {
   let api_router: Router<AppState> =
      Router::new().merge(health::router()).merge(cinema::router()).merge(auth::router());

   Router::new()
      .nest("/api/v1", api_router)
      .fallback(fallback)
      .method_not_allowed_fallback(method_not_allowed)
      .layer(TraceLayer::new_for_http())
      .layer(cors_layer(cors_allowed_origins))
      .with_state(app_state)
}

fn cors_layer(cors_allowed_origins: &[String]) -> CorsLayer {
   let allowed_origins: Vec<HeaderValue> =
      cors_allowed_origins.iter().filter_map(|origin| HeaderValue::from_str(origin).ok()).collect();
   CorsLayer::new()
      .allow_origin(allowed_origins)
      .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
      .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, HeaderName::from_static("x-requested-with")])
      .allow_credentials(true)
}

async fn fallback() -> AppError {
   AppError::not_found("NOT_FOUND", "Không tìm thấy tài nguyên")
}

async fn method_not_allowed() -> AppError {
   AppError::new(StatusCode::METHOD_NOT_ALLOWED, "METHOD_NOT_ALLOWED", "Phương thức HTTP không được hỗ trợ")
}
