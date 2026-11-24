use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;
use crate::modules::cinema::enums::{HallStatus, ScreenType};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HallRequest {
   pub name: Option<String>,
   pub screen_type: Option<ScreenType>,
   pub status: Option<HallStatus>,
}

pub struct HallCommand {
   pub name: String,
   pub screen_type: ScreenType,
   pub status: HallStatus,
}

impl HallRequest {
   pub fn validate(self) -> Result<HallCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let name: String = field_errors.required_text("name", &self.name, "Vui lòng nhập tên phòng chiếu");
      field_errors.max_chars("name", &name, 100, "Tên phòng chiếu tối đa 100 ký tự");
      if self.screen_type.is_none() {
         field_errors.add("screenType", "Vui lòng chọn loại màn hình");
      }
      if self.status.is_none() {
         field_errors.add("status", "Vui lòng chọn trạng thái");
      }
      field_errors.finish()?;
      Ok(HallCommand {
         name: name.trim().to_string(),
         screen_type: self.screen_type.unwrap_or(ScreenType::Screen2d),
         status: self.status.unwrap_or(HallStatus::Active),
      })
   }
}

#[cfg(test)]
mod tests {
   use super::HallRequest;
   use crate::modules::cinema::dto::request::field_errors_of;

   #[test]
   fn missing_fields_are_reported_with_vietnamese_messages() {
      let data = field_errors_of(HallRequest::default().validate());
      assert_eq!(data["name"], "Vui lòng nhập tên phòng chiếu");
      assert_eq!(data["screenType"], "Vui lòng chọn loại màn hình");
      assert_eq!(data["status"], "Vui lòng chọn trạng thái");
   }
}
