use std::collections::{HashMap, HashSet};

use crate::common::error::AppError;
use crate::modules::cinema::dto::request::SeatCellCommand;
use crate::modules::cinema::entity::SeatTypeRecord;

pub const MAX_COLUMNS: i32 = 40;
const INVALID_SEAT_MAP: &str = "INVALID_SEAT_MAP";
pub const INVALID_SEAT_TYPE: &str = "INVALID_SEAT_TYPE";

pub fn label(row: &str, number: i32) -> String {
   format!("{row}{number}")
}

pub fn invalid_seat_type(seat_type_code: &str) -> AppError {
   AppError::bad_request(INVALID_SEAT_TYPE, &format!("Loại ghế '{seat_type_code}' không tồn tại"))
}

fn invalid(message: &str) -> AppError {
   AppError::bad_request(INVALID_SEAT_MAP, message)
}

pub fn validate(
   seat_cell_commands: &[SeatCellCommand],
   seat_types_by_code: &HashMap<String, SeatTypeRecord>,
) -> Result<(), AppError> {
   let mut labels: HashSet<String> = HashSet::new();
   let mut occupied_cells: HashMap<String, String> = HashMap::new();
   for seat_cell_command in seat_cell_commands {
      let seat_label: String = label(&seat_cell_command.row, seat_cell_command.number);
      if !labels.insert(seat_label.clone()) {
         return Err(invalid(&format!("Ghế {seat_label} bị khai báo trùng")));
      }
      let seat_type_record: &SeatTypeRecord = seat_types_by_code
         .get(&seat_cell_command.seat_type_code)
         .ok_or_else(|| invalid_seat_type(&seat_cell_command.seat_type_code))?;
      let last_column: i32 = seat_cell_command.number + seat_type_record.seat_span - 1;
      if last_column > MAX_COLUMNS {
         return Err(invalid(&format!("Ghế {seat_label} vượt quá cột cuối cùng của sơ đồ (tối đa {MAX_COLUMNS} cột)")));
      }
      for column in seat_cell_command.number..=last_column {
         let cell_key: String = format!("{}:{column}", seat_cell_command.row);
         if let Some(occupying_seat_label) = occupied_cells.get(&cell_key) {
            return Err(invalid(&format!("Ghế {seat_label} chồng lên ghế {occupying_seat_label}")));
         }
         occupied_cells.insert(cell_key, seat_label.clone());
      }
   }
   Ok(())
}

#[cfg(test)]
mod tests {
   use std::collections::HashMap;

   use super::validate;
   use crate::modules::cinema::dto::request::SeatCellCommand;
   use crate::modules::cinema::entity::SeatTypeRecord;
   use crate::modules::cinema::enums::SeatStatus;

   fn seat_types() -> HashMap<String, SeatTypeRecord> {
      let mut seat_types_by_code: HashMap<String, SeatTypeRecord> = HashMap::new();
      for (id, code, span) in [(1, "STANDARD", 1), (3, "SWEETBOX", 2)] {
         seat_types_by_code.insert(
            code.to_string(),
            SeatTypeRecord {
               id,
               code: code.to_string(),
               name: code.to_string(),
               default_surcharge: 0.0,
               seat_span: span,
            },
         );
      }
      seat_types_by_code
   }

   fn cell(row: &str, number: i32, seat_type_code: &str) -> SeatCellCommand {
      SeatCellCommand {
         row: row.to_string(),
         number,
         seat_type_code: seat_type_code.to_string(),
         status: SeatStatus::Active,
      }
   }

   #[test]
   fn accepts_a_clean_layout_with_double_seats() {
      let seat_cell_commands = vec![cell("A", 1, "STANDARD"), cell("A", 2, "SWEETBOX"), cell("A", 4, "STANDARD")];
      assert!(validate(&seat_cell_commands, &seat_types()).is_ok());
   }

   #[test]
   fn rejects_duplicates_overlaps_overflow_and_unknown_types() {
      let seat_types_by_code = seat_types();
      let duplicate = validate(&[cell("A", 1, "STANDARD"), cell("A", 1, "STANDARD")], &seat_types_by_code);
      assert_eq!(duplicate.unwrap_err().message, "Ghế A1 bị khai báo trùng");
      let overlap = validate(&[cell("B", 2, "SWEETBOX"), cell("B", 3, "STANDARD")], &seat_types_by_code);
      assert_eq!(overlap.unwrap_err().message, "Ghế B3 chồng lên ghế B2");
      let overflow = validate(&[cell("C", 40, "SWEETBOX")], &seat_types_by_code);
      assert_eq!(overflow.unwrap_err().error_code, "INVALID_SEAT_MAP");
      let unknown = validate(&[cell("D", 1, "GOLD")], &seat_types_by_code);
      assert_eq!(unknown.unwrap_err().error_code, "INVALID_SEAT_TYPE");
   }
}
