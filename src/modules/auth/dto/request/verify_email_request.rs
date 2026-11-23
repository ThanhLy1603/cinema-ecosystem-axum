use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct VerifyEmailRequest {
   pub token: Option<String>,
}

pub struct VerifyEmailCommand {
   pub token: String,
}

impl VerifyEmailRequest {
   pub fn validate(self) -> Result<VerifyEmailCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let token: String = field_errors.required_text("token", &self.token, "Token không được để trống");
      field_errors.max_chars("token", &token, 128, "Token không hợp lệ");
      field_errors.finish()?;
      Ok(VerifyEmailCommand { token })
   }
}
#[cfg(test)]
mod tests {
   use super::VerifyEmailRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn verify_email_requires_token() {
      assert_eq!(field_errors_of(VerifyEmailRequest { token: None }.validate())["token"], "Token không được để trống");
      assert!(VerifyEmailRequest { token: Some("abc".to_string()) }.validate().is_ok());
   }
}
