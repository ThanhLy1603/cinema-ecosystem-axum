mod dto;
mod entity;
mod enums;
mod handler;
mod model;
mod repository;
mod security;
mod service;

use axum::routing::{get, post, put};
use axum::Router;

use crate::state::AppState;

pub use security::{AuthenticatedUser, JwtService, PasswordService};
pub use service::{delete_expired_tokens, spawn_token_cleanup};

pub fn router() -> Router<AppState> {
   Router::new()
      .route("/auth/register", post(handler::auth_handler::register))
      .route("/auth/login", post(handler::auth_handler::login))
      .route("/auth/refresh-token", post(handler::auth_handler::refresh_token))
      .route("/auth/forgot-password", post(handler::auth_handler::forgot_password))
      .route("/auth/reset-password", post(handler::auth_handler::reset_password))
      .route("/auth/verify-email", post(handler::auth_handler::verify_email))
      .route("/auth/resend-verification", post(handler::auth_handler::resend_verification))
      .route("/auth/logout", post(handler::auth_handler::logout))
      .route("/auth/logout-all", post(handler::auth_handler::logout_all))
      .route("/users/me", get(handler::profile_handler::get_profile).put(handler::profile_handler::update_profile))
      .route("/users/me/password", put(handler::profile_handler::change_password))
}
