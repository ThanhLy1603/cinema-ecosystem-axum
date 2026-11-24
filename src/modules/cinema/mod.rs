mod dto;
mod entity;
mod enums;
mod handler;
mod repository;
mod service;

use axum::routing::{get, post, put};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
   Router::new()
      .route("/cinemas/public", get(handler::public_cinema_handler::list_public_cinemas))
      .route("/cinemas/public/cities", get(handler::public_cinema_handler::list_public_cities))
      .route("/cinemas/public/{slug}", get(handler::public_cinema_handler::get_public_cinema_by_slug))
      .route("/halls/{hallId}/seat-layout", get(handler::public_cinema_handler::get_seat_layout))
      .route("/seat-types", get(handler::public_cinema_handler::list_seat_types))
      .route("/admin/cinema-options", get(handler::cinema_option_handler::list_cinema_options))
      .route(
         "/admin/cinemas",
         get(handler::admin_cinema_handler::search_cinemas).post(handler::admin_cinema_handler::create_cinema),
      )
      .route(
         "/admin/cinemas/{cinemaId}",
         get(handler::admin_cinema_handler::get_cinema)
            .put(handler::admin_cinema_handler::update_cinema)
            .delete(handler::admin_cinema_handler::delete_cinema),
      )
      .route("/admin/cinemas/{cinemaId}/restore", post(handler::admin_cinema_handler::restore_cinema))
      .route(
         "/admin/cinemas/{cinemaId}/halls",
         get(handler::admin_hall_handler::list_halls).post(handler::admin_hall_handler::create_hall),
      )
      .route(
         "/admin/cinema-facilities",
         get(handler::admin_facility_handler::list_facilities).post(handler::admin_facility_handler::create_facility),
      )
      .route(
         "/admin/cinema-facilities/{facilityId}",
         put(handler::admin_facility_handler::update_facility).delete(handler::admin_facility_handler::delete_facility),
      )
      .route("/admin/cinema-facilities/{facilityId}/restore", post(handler::admin_facility_handler::restore_facility))
      .route(
         "/admin/halls/{hallId}",
         put(handler::admin_hall_handler::update_hall).delete(handler::admin_hall_handler::delete_hall),
      )
      .route("/admin/halls/{hallId}/restore", post(handler::admin_hall_handler::restore_hall))
      .route(
         "/admin/halls/{hallId}/seat-map",
         get(handler::admin_seat_map_handler::get_seat_map).put(handler::admin_seat_map_handler::save_seat_map),
      )
      .route("/admin/halls/{hallId}/seat-map/lock", get(handler::admin_seat_map_handler::get_seat_map_lock))
      .route("/admin/halls/{hallId}/generate-seats", post(handler::admin_seat_map_handler::generate_seats))
}
