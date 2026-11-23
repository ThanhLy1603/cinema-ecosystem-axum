mod dto;
mod handler;
mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
   Router::new().route("/health", get(handler::health))
}
