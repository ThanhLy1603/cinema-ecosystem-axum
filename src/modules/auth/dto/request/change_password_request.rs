use serde::Deserialize;

use super::validation_rules::check_new_password;
use crate::common::error::AppError;
use crate::common::validation::FieldErrors;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ChangePasswordRequest {
   pub current_password: Option<String>,
   pub new_password: Option<String>,
}

pub struct ChangePasswordCommand {
   pub current_password: Option<String>,
   pub new_password: String,
}

impl ChangePasswordRequest {
   pub fn validate(self) -> Result<ChangePasswordCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      if let Some(current_password) = self.current_password.as_deref() {
         field_errors.max_chars("currentPassword", current_password, 72, "Mật khẩu tối đa 72 ký tự");
      }
      let new_password: String =
         field_errors.required_text("newPassword", &self.new_password, "Mật khẩu mới không được để trống");
      if !new_password.is_empty() {
         check_new_password(&mut field_errors, "newPassword", &new_password);
      }
      field_errors.finish()?;
      Ok(ChangePasswordCommand { current_password: self.current_password, new_password })
   }
}
#[cfg(test)]
mod tests {
   use super::ChangePasswordRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn change_password_current_password_is_optional_but_new_one_is_required() {
      let change_password_command =
         ChangePasswordRequest { current_password: None, new_password: Some("matkhau123".to_string()) }
            .validate()
            .unwrap();
      assert!(change_password_command.current_password.is_none());
      let data = field_errors_of(ChangePasswordRequest::default().validate());
      assert_eq!(data["newPassword"], "Mật khẩu mới không được để trống");
   }
}
