use serde::Deserialize;

use super::validation_rules::{check_new_password, PHONE_MESSAGE};
use crate::common::error::AppError;
use crate::common::validation::{is_valid_email, is_valid_phone, FieldErrors};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RegisterRequest {
   pub email: Option<String>,
   pub password: Option<String>,
   pub full_name: Option<String>,
   pub phone: Option<String>,
}

pub struct RegisterCommand {
   pub email: String,
   pub password: String,
   pub full_name: String,
   pub phone: Option<String>,
}

impl RegisterRequest {
   pub fn validate(self) -> Result<RegisterCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let email: String = field_errors.required_text("email", &self.email, "Email không được để trống");
      field_errors.max_chars("email", &email, 150, "Email tối đa 150 ký tự");
      if !email.is_empty() && !is_valid_email(email.trim()) {
         field_errors.add("email", "Email không hợp lệ");
      }
      let password: String = field_errors.required_text("password", &self.password, "Mật khẩu không được để trống");
      if !password.is_empty() {
         check_new_password(&mut field_errors, "password", &password);
      }
      let full_name: String = field_errors.required_text("fullName", &self.full_name, "Họ tên không được để trống");
      field_errors.max_chars("fullName", &full_name, 150, "Họ tên tối đa 150 ký tự");
      if let Some(phone) = self.phone.as_deref() {
         if !is_valid_phone(phone) {
            field_errors.add("phone", PHONE_MESSAGE);
         }
      }
      field_errors.finish()?;
      Ok(RegisterCommand {
         email: email.trim().to_lowercase(),
         password,
         full_name: full_name.trim().to_string(),
         phone: self.phone,
      })
   }
}
#[cfg(test)]
mod tests {
   use super::RegisterRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   fn register_request() -> RegisterRequest {
      RegisterRequest {
         email: Some("  Khach.Hang@Example.COM ".to_string()),
         password: Some("matkhau123".to_string()),
         full_name: Some("  Nguyễn Văn An ".to_string()),
         phone: Some("0901234567".to_string()),
      }
   }

   #[test]
   fn register_normalizes_email_name_and_phone() {
      let register_command = register_request().validate().unwrap();
      assert_eq!(register_command.email, "khach.hang@example.com");
      assert_eq!(register_command.full_name, "Nguyễn Văn An");
      assert_eq!(register_command.phone.as_deref(), Some("0901234567"));
   }

   #[test]
   fn register_rejects_blank_phone_like_spring_but_allows_missing_phone() {
      let mut request: RegisterRequest = register_request();
      request.phone = Some("   ".to_string());
      assert_eq!(field_errors_of(request.validate())["phone"], "Số điện thoại không hợp lệ");

      let mut request: RegisterRequest = register_request();
      request.phone = Some(String::new());
      assert_eq!(field_errors_of(request.validate())["phone"], "Số điện thoại không hợp lệ");

      let mut request: RegisterRequest = register_request();
      request.phone = None;
      assert!(request.validate().unwrap().phone.is_none());
   }

   #[test]
   fn register_reports_every_invalid_field_with_spring_messages() {
      let request: RegisterRequest = RegisterRequest {
         email: Some("khong-hop-le".to_string()),
         password: Some("ngan".to_string()),
         full_name: None,
         phone: Some("abc".to_string()),
      };
      let data = field_errors_of(request.validate());
      assert_eq!(data["email"], "Email không hợp lệ");
      assert_eq!(data["password"], "Mật khẩu phải từ 8 đến 72 ký tự");
      assert_eq!(data["fullName"], "Họ tên không được để trống");
      assert_eq!(data["phone"], "Số điện thoại không hợp lệ");
   }

   #[test]
   fn register_missing_body_fields_are_reported_as_blank() {
      let data = field_errors_of(RegisterRequest::default().validate());
      assert_eq!(data["email"], "Email không được để trống");
      assert_eq!(data["password"], "Mật khẩu không được để trống");
      assert_eq!(data["fullName"], "Họ tên không được để trống");
      assert!(data.get("phone").is_none());
   }

   #[test]
   fn register_password_needs_letters_and_digits() {
      let mut request: RegisterRequest = register_request();
      request.password = Some("chicochucai".to_string());
      assert_eq!(field_errors_of(request.validate())["password"], "Mật khẩu phải gồm cả chữ và số");
   }

   #[test]
   fn register_password_longer_than_72_chars_is_rejected() {
      let mut request: RegisterRequest = register_request();
      request.password = Some(format!("a1{}", "b".repeat(71)));
      assert_eq!(field_errors_of(request.validate())["password"], "Mật khẩu phải từ 8 đến 72 ký tự");
   }
}
