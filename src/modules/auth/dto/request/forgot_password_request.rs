use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::{is_valid_email, FieldErrors};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ForgotPasswordRequest {
   pub email: Option<String>,
}

pub struct ForgotPasswordCommand {
   pub email: String,
}

impl ForgotPasswordRequest {
   pub fn validate(self) -> Result<ForgotPasswordCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let email: String = field_errors.required_text("email", &self.email, "Email không được để trống");
      field_errors.max_chars("email", &email, 150, "Email tối đa 150 ký tự");
      if !email.is_empty() && !is_valid_email(email.trim()) {
         field_errors.add("email", "Email không hợp lệ");
      }
      field_errors.finish()?;
      Ok(ForgotPasswordCommand { email: email.trim().to_lowercase() })
   }
}
#[cfg(test)]
mod tests {
   use super::ForgotPasswordRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn forgot_password_needs_valid_email() {
      let data = field_errors_of(ForgotPasswordRequest { email: Some("sai".to_string()) }.validate());
      assert_eq!(data["email"], "Email không hợp lệ");
      assert!(ForgotPasswordRequest { email: Some("a@example.com".to_string()) }.validate().is_ok());
   }
}
