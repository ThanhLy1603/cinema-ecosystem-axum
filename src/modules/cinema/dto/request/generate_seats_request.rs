use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GenerateSeatsRequest {
   pub row_count: Option<i64>,
   pub column_count: Option<i64>,
   pub seat_type_code: Option<String>,
}

pub struct GenerateSeatsCommand {
   pub row_count: i64,
   pub column_count: i64,
   pub seat_type_code: String,
}

impl GenerateSeatsRequest {
   pub fn validate(self) -> Result<GenerateSeatsCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let row_count: i64 = self.row_count.unwrap_or(0);
      if row_count < 1 {
         field_errors.add("rowCount", "Số hàng tối thiểu là 1");
      } else if row_count > 26 {
         field_errors.add("rowCount", "Số hàng tối đa là 26 (A đến Z)");
      }
      let column_count: i64 = self.column_count.unwrap_or(0);
      if column_count < 1 {
         field_errors.add("columnCount", "Số cột tối thiểu là 1");
      } else if column_count > 40 {
         field_errors.add("columnCount", "Số cột tối đa là 40");
      }
      let seat_type_code: String =
         field_errors.required_text("seatTypeCode", &self.seat_type_code, "Vui lòng chọn loại ghế mặc định");
      field_errors.finish()?;
      Ok(GenerateSeatsCommand { row_count, column_count, seat_type_code })
   }
}

#[cfg(test)]
mod tests {
   use super::GenerateSeatsRequest;
   use crate::modules::cinema::dto::request::field_errors_of;

   #[test]
   fn grid_size_is_bounded() {
      let low = field_errors_of(GenerateSeatsRequest::default().validate());
      assert_eq!(low["rowCount"], "Số hàng tối thiểu là 1");
      assert_eq!(low["columnCount"], "Số cột tối thiểu là 1");
      assert_eq!(low["seatTypeCode"], "Vui lòng chọn loại ghế mặc định");
      let high: GenerateSeatsRequest =
         serde_json::from_value(serde_json::json!({ "rowCount": 27, "columnCount": 41, "seatTypeCode": "VIP" }))
            .unwrap();
      let data = field_errors_of(high.validate());
      assert_eq!(data["rowCount"], "Số hàng tối đa là 26 (A đến Z)");
      assert_eq!(data["columnCount"], "Số cột tối đa là 40");
   }
}
