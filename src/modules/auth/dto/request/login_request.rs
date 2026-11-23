use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LoginRequest {
   pub email: Option<String>,
   pub password: Option<String>,
}

pub struct LoginCommand {
   pub email: String,
   pub password: String,
}

impl LoginRequest {
   pub fn validate(self) -> Result<LoginCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let email: String = field_errors.required_text("email", &self.email, "Email không được để trống");
      field_errors.max_chars("email", &email, 150, "Email tối đa 150 ký tự");
      let password: String = field_errors.required_text("password", &self.password, "Mật khẩu không được để trống");
      field_errors.max_chars("password", &password, 72, "Mật khẩu tối đa 72 ký tự");
      field_errors.finish()?;
      Ok(LoginCommand { email: email.trim().to_lowercase(), password })
   }
}
#[cfg(test)]
mod tests {
   use super::LoginRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn login_lowercases_email_and_keeps_password_untouched() {
      let login_command =
         LoginRequest { email: Some(" A@Example.com ".to_string()), password: Some("  giu nguyen  ".to_string()) }
            .validate()
            .unwrap();
      assert_eq!(login_command.email, "a@example.com");
      assert_eq!(login_command.password, "  giu nguyen  ");
   }

   #[test]
   fn login_rejects_blank_fields() {
      let data = field_errors_of(LoginRequest::default().validate());
      assert_eq!(data["email"], "Email không được để trống");
      assert_eq!(data["password"], "Mật khẩu không được để trống");
   }
}
