use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GoogleLoginRequest {
   pub id_token: Option<String>,
}

pub struct GoogleLoginCommand {
   pub id_token: String,
}

impl GoogleLoginRequest {
   pub fn validate(self) -> Result<GoogleLoginCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let id_token: String =
         field_errors.required_text("idToken", &self.id_token, "Google ID Token không được để trống");
      field_errors.max_chars("idToken", &id_token, 4096, "Google ID Token không hợp lệ");
      field_errors.finish()?;
      Ok(GoogleLoginCommand { id_token })
   }
}

#[cfg(test)]
mod tests {
   use super::GoogleLoginRequest;
   use crate::modules::auth::dto::request::field_errors_of;

   #[test]
   fn blank_and_oversized_tokens_are_rejected() {
      let blank = field_errors_of(GoogleLoginRequest { id_token: Some("  ".to_string()) }.validate());
      assert_eq!(blank["idToken"], "Google ID Token không được để trống");
      let oversized = field_errors_of(GoogleLoginRequest { id_token: Some("a".repeat(4097)) }.validate());
      assert_eq!(oversized["idToken"], "Google ID Token không hợp lệ");
      assert_eq!(GoogleLoginRequest { id_token: Some("abc".to_string()) }.validate().unwrap().id_token, "abc");
   }
}
