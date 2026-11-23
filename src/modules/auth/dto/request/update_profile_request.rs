use chrono::{NaiveDate, Utc};
use serde::Deserialize;

use super::validation_rules::PHONE_MESSAGE;
use crate::common::error::AppError;
use crate::common::validation::{is_https_url_without_spaces, is_valid_phone, FieldErrors};
use crate::modules::auth::enums::Gender;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateProfileRequest {
   pub full_name: Option<String>,
   pub phone: Option<String>,
   pub date_of_birth: Option<NaiveDate>,
   pub gender: Option<Gender>,
   pub avatar_url: Option<String>,
}

pub struct UpdateProfileCommand {
   pub full_name: String,
   pub phone: Option<String>,
   pub date_of_birth: Option<NaiveDate>,
   pub gender: Option<Gender>,
   pub avatar_url: Option<String>,
}

impl UpdateProfileRequest {
   pub fn validate(self) -> Result<UpdateProfileCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let full_name: String = field_errors.required_text("fullName", &self.full_name, "Họ tên không được để trống");
      field_errors.max_chars("fullName", &full_name, 150, "Họ tên tối đa 150 ký tự");
      if let Some(phone) = self.phone.as_deref() {
         if !is_valid_phone(phone) {
            field_errors.add("phone", PHONE_MESSAGE);
         }
      }
      if let Some(date_of_birth) = self.date_of_birth {
         if date_of_birth >= Utc::now().date_naive() {
            field_errors.add("dateOfBirth", "Ngày sinh phải ở quá khứ");
         }
      }
      if let Some(avatar_url) = self.avatar_url.as_deref() {
         field_errors.max_chars("avatarUrl", avatar_url, 500, "Đường dẫn ảnh đại diện tối đa 500 ký tự");
         if !is_https_url_without_spaces(avatar_url) {
            field_errors.add("avatarUrl", "Ảnh đại diện phải là đường dẫn https");
         }
      }
      field_errors.finish()?;
      Ok(UpdateProfileCommand {
         full_name: full_name.trim().to_string(),
         phone: self.phone,
         date_of_birth: self.date_of_birth,
         gender: self.gender,
         avatar_url: self.avatar_url,
      })
   }
}
#[cfg(test)]
mod tests {
   use super::UpdateProfileRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn update_profile_validates_phone_birthday_and_avatar() {
      let request: UpdateProfileRequest = serde_json::from_value(serde_json::json!({
         "fullName": "An",
         "phone": "12",
         "dateOfBirth": "2999-01-01",
         "gender": "MALE",
         "avatarUrl": "http://khong-https.example.com/a.png"
      }))
      .unwrap();
      let data = field_errors_of(request.validate());
      assert_eq!(data["phone"], "Số điện thoại không hợp lệ");
      assert_eq!(data["dateOfBirth"], "Ngày sinh phải ở quá khứ");
      assert_eq!(data["avatarUrl"], "Ảnh đại diện phải là đường dẫn https");
      assert!(data.get("fullName").is_none());
   }

   #[test]
   fn update_profile_accepts_minimal_and_full_payloads() {
      let minimal: UpdateProfileRequest = serde_json::from_value(serde_json::json!({ "fullName": " An " })).unwrap();
      let update_profile_command = minimal.validate().unwrap();
      assert_eq!(update_profile_command.full_name, "An");
      assert!(update_profile_command.phone.is_none() && update_profile_command.gender.is_none());

      let full: UpdateProfileRequest = serde_json::from_value(serde_json::json!({
         "fullName": "An",
         "phone": "+84901234567",
         "dateOfBirth": "1999-12-31",
         "gender": "FEMALE",
         "avatarUrl": "https://cdn.example.com/a.png"
      }))
      .unwrap();
      assert!(full.validate().is_ok());
   }

   #[test]
   fn update_profile_rejects_unknown_gender_at_deserialization() {
      let result =
         serde_json::from_value::<UpdateProfileRequest>(serde_json::json!({ "fullName": "An", "gender": "KHAC" }));
      assert!(result.is_err());
   }
}
