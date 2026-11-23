use serde::Deserialize;

use super::validation_rules::check_new_password;
use crate::common::error::AppError;
use crate::common::validation::FieldErrors;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ResetPasswordRequest {
   pub token: Option<String>,
   pub new_password: Option<String>,
}

pub struct ResetPasswordCommand {
   pub token: String,
   pub new_password: String,
}

impl ResetPasswordRequest {
   pub fn validate(self) -> Result<ResetPasswordCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let token: String = field_errors.required_text("token", &self.token, "Token không được để trống");
      field_errors.max_chars("token", &token, 128, "Token không hợp lệ");
      let new_password: String =
         field_errors.required_text("newPassword", &self.new_password, "Mật khẩu mới không được để trống");
      if !new_password.is_empty() {
         check_new_password(&mut field_errors, "newPassword", &new_password);
      }
      field_errors.finish()?;
      Ok(ResetPasswordCommand { token, new_password })
   }
}
#[cfg(test)]
mod tests {
   use super::ResetPasswordRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn reset_password_validates_token_and_new_password() {
      let data =
         field_errors_of(ResetPasswordRequest { token: None, new_password: Some("1234".to_string()) }.validate());
      assert_eq!(data["token"], "Token không được để trống");
      assert_eq!(data["newPassword"], "Mật khẩu phải từ 8 đến 72 ký tự");
      let long_token: String = "t".repeat(129);
      let data = field_errors_of(
         ResetPasswordRequest { token: Some(long_token), new_password: Some("matkhau123".to_string()) }.validate(),
      );
      assert_eq!(data["token"], "Token không hợp lệ");
   }
}
