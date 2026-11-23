mod dto;
mod handler;
mod repository;
mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
   Router::new()
      .route("/cinemas/public", get(handler::list_public_cinemas))
      .route("/cinemas/public/cities", get(handler::list_public_cities))
}
