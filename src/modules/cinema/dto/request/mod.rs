mod cinema_request;
mod facility_request;
mod generate_seats_request;
mod hall_request;
mod seat_map_request;

pub use cinema_request::{CinemaCommand, CinemaRequest};
pub use facility_request::{FacilityCommand, FacilityRequest};
pub use generate_seats_request::{GenerateSeatsCommand, GenerateSeatsRequest};
pub use hall_request::{HallCommand, HallRequest};
pub use seat_map_request::{SeatCellCommand, SeatMapCommand, SeatMapRequest};

#[cfg(test)]
pub fn field_errors_of<T>(result: Result<T, crate::common::error::AppError>) -> serde_json::Value {
   match result {
      Ok(_) => panic!("dữ liệu hợp lệ nhưng test mong đợi lỗi"),
      Err(app_error) => {
         assert_eq!(app_error.error_code, "VALIDATION_ERROR");
         app_error.data.unwrap()
      }
   }
}
