use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::{is_https_url_without_spaces, FieldErrors};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FacilityRequest {
   pub name: Option<String>,
   pub icon_url: Option<String>,
   pub description: Option<String>,
}

pub struct FacilityCommand {
   pub name: String,
   pub icon_url: Option<String>,
   pub description: Option<String>,
}

fn blank_to_none(value: Option<String>) -> Option<String> {
   value.map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

impl FacilityRequest {
   pub fn validate(self) -> Result<FacilityCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let name: String = field_errors.required_text("name", &self.name, "Vui lòng nhập tên tiện ích");
      field_errors.max_chars("name", &name, 100, "Tên tiện ích tối đa 100 ký tự");
      if let Some(icon_url) = self.icon_url.as_deref() {
         field_errors.max_chars("iconUrl", icon_url, 500, "Đường dẫn biểu tượng tối đa 500 ký tự");
         if !icon_url.is_empty() && !is_https_url_without_spaces(icon_url) {
            field_errors.add("iconUrl", "Biểu tượng phải là đường dẫn bắt đầu bằng https://");
         }
      }
      if let Some(description) = self.description.as_deref() {
         field_errors.max_chars("description", description, 500, "Mô tả tối đa 500 ký tự");
      }
      field_errors.finish()?;
      Ok(FacilityCommand {
         name: name.trim().to_string(),
         icon_url: blank_to_none(self.icon_url),
         description: blank_to_none(self.description),
      })
   }
}

#[cfg(test)]
mod tests {
   use super::FacilityRequest;
   use crate::modules::cinema::dto::request::field_errors_of;

   #[test]
   fn icon_must_be_https_and_name_is_required() {
      let request: FacilityRequest =
         serde_json::from_value(serde_json::json!({ "iconUrl": "http://a.com/x.svg" })).unwrap();
      let data = field_errors_of(request.validate());
      assert_eq!(data["name"], "Vui lòng nhập tên tiện ích");
      assert_eq!(data["iconUrl"], "Biểu tượng phải là đường dẫn bắt đầu bằng https://");
   }

   #[test]
   fn blank_optional_fields_become_none() {
      let request: FacilityRequest =
         serde_json::from_value(serde_json::json!({ "name": " Wifi ", "iconUrl": "", "description": "  " })).unwrap();
      let facility_command = request.validate().unwrap();
      assert_eq!(facility_command.name, "Wifi");
      assert!(facility_command.icon_url.is_none() && facility_command.description.is_none());
   }
}
