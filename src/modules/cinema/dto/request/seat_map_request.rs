use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;
use crate::modules::cinema::enums::SeatStatus;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SeatCellRequest {
   pub row: Option<String>,
   pub number: Option<i64>,
   pub seat_type_code: Option<String>,
   pub status: Option<SeatStatus>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SeatMapRequest {
   pub version: Option<i64>,
   pub seats: Option<Vec<SeatCellRequest>>,
}

#[derive(Clone, Debug)]
pub struct SeatCellCommand {
   pub row: String,
   pub number: i32,
   pub seat_type_code: String,
   pub status: SeatStatus,
}

pub struct SeatMapCommand {
   pub version: i64,
   pub seats: Vec<SeatCellCommand>,
}

const MAX_SEATS_PER_HALL: usize = 800;

fn is_single_capital_letter(row: &str) -> bool {
   let mut characters = row.chars();
   matches!((characters.next(), characters.next()), (Some('A'..='Z'), None))
}

impl SeatMapRequest {
   pub fn validate(self) -> Result<SeatMapCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      if self.version.is_none() {
         field_errors.add("version", "Thiếu phiên bản sơ đồ ghế");
      }
      let mut seats: Vec<SeatCellCommand> = Vec::new();
      match self.seats {
         None => field_errors.add("seats", "Thiếu danh sách ghế"),
         Some(seat_cell_requests) => {
            if seat_cell_requests.len() > MAX_SEATS_PER_HALL {
               field_errors.add("seats", "Một phòng chiếu tối đa 800 ghế");
            }
            for (index, seat_cell_request) in seat_cell_requests.into_iter().enumerate() {
               if let Some(seat_cell_command) = validate_seat_cell(&mut field_errors, index, seat_cell_request) {
                  seats.push(seat_cell_command);
               }
            }
         }
      }
      field_errors.finish()?;
      Ok(SeatMapCommand { version: self.version.unwrap_or_default(), seats })
   }
}

fn validate_seat_cell(
   field_errors: &mut FieldErrors,
   index: usize,
   seat_cell_request: SeatCellRequest,
) -> Option<SeatCellCommand> {
   let key = |field: &str| format!("seats[{index}].{field}");
   let mut is_valid: bool = true;
   let row: String = match seat_cell_request.row {
      Some(row) if row.trim().is_empty() => {
         field_errors.add(&key("row"), "Hàng ghế không được để trống");
         is_valid = false;
         String::new()
      }
      Some(row) if !is_single_capital_letter(&row) => {
         field_errors.add(&key("row"), "Hàng ghế phải là một chữ cái từ A đến Z");
         is_valid = false;
         String::new()
      }
      Some(row) => row,
      None => {
         field_errors.add(&key("row"), "Hàng ghế không được để trống");
         is_valid = false;
         String::new()
      }
   };
   let number: i64 = seat_cell_request.number.unwrap_or(0);
   if number < 1 {
      field_errors.add(&key("number"), "Số ghế tối thiểu là 1");
      is_valid = false;
   } else if number > 40 {
      field_errors.add(&key("number"), "Số ghế tối đa là 40");
      is_valid = false;
   }
   let seat_type_code: String = match seat_cell_request.seat_type_code {
      Some(code) if !code.trim().is_empty() => code,
      _ => {
         field_errors.add(&key("seatTypeCode"), "Loại ghế không được để trống");
         is_valid = false;
         String::new()
      }
   };
   let status: SeatStatus = match seat_cell_request.status {
      Some(status) => status,
      None => {
         field_errors.add(&key("status"), "Trạng thái ghế không được để trống");
         is_valid = false;
         SeatStatus::Active
      }
   };
   is_valid.then_some(SeatCellCommand { row, number: number as i32, seat_type_code, status })
}

#[cfg(test)]
mod tests {
   use super::SeatMapRequest;
   use crate::modules::cinema::dto::request::field_errors_of;

   #[test]
   fn missing_version_and_seats_are_reported() {
      let data = field_errors_of(SeatMapRequest::default().validate());
      assert_eq!(data["version"], "Thiếu phiên bản sơ đồ ghế");
      assert_eq!(data["seats"], "Thiếu danh sách ghế");
   }

   #[test]
   fn invalid_seat_cells_are_reported_with_indexed_keys() {
      let request: SeatMapRequest = serde_json::from_value(serde_json::json!({
         "version": 0,
         "seats": [
            { "row": "A", "number": 1, "seatTypeCode": "STANDARD", "status": "ACTIVE" },
            { "row": "AB", "number": 41, "seatTypeCode": " " }
         ]
      }))
      .unwrap();
      let data = field_errors_of(request.validate());
      assert!(data.get("seats[0].row").is_none());
      assert_eq!(data["seats[1].row"], "Hàng ghế phải là một chữ cái từ A đến Z");
      assert_eq!(data["seats[1].number"], "Số ghế tối đa là 40");
      assert_eq!(data["seats[1].seatTypeCode"], "Loại ghế không được để trống");
      assert_eq!(data["seats[1].status"], "Trạng thái ghế không được để trống");
   }

   #[test]
   fn more_than_800_seats_are_rejected() {
      let seats: Vec<serde_json::Value> = (0..801)
         .map(|_| serde_json::json!({ "row": "A", "number": 1, "seatTypeCode": "STANDARD", "status": "ACTIVE" }))
         .collect();
      let request: SeatMapRequest =
         serde_json::from_value(serde_json::json!({ "version": 1, "seats": seats })).unwrap();
      assert_eq!(field_errors_of(request.validate())["seats"], "Một phòng chiếu tối đa 800 ghế");
   }
}
