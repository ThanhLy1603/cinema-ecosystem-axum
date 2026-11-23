use std::collections::BTreeMap;

use crate::common::error::AppError;

#[derive(Default)]
pub struct FieldErrors {
   errors: BTreeMap<String, String>,
}

impl FieldErrors {
   pub fn new() -> FieldErrors {
      FieldErrors::default()
   }

   pub fn add(&mut self, field: &str, message: &str) {
      self.errors.entry(field.to_string()).or_insert_with(|| message.to_string());
   }

   pub fn finish(self) -> Result<(), AppError> {
      if self.errors.is_empty() {
         Ok(())
      } else {
         Err(AppError::validation(self.errors))
      }
   }

   pub fn required_text(&mut self, field: &str, value: &Option<String>, message: &str) -> String {
      match value {
         Some(text) if !text.trim().is_empty() => text.clone(),
         _ => {
            self.add(field, message);
            String::new()
         }
      }
   }

   pub fn max_chars(&mut self, field: &str, value: &str, max: usize, message: &str) {
      if value.chars().count() > max {
         self.add(field, message);
      }
   }

   pub fn min_chars(&mut self, field: &str, value: &str, min: usize, message: &str) {
      if value.chars().count() < min {
         self.add(field, message);
      }
   }
}

pub fn is_valid_email(email: &str) -> bool {
   let Some((local_part, domain)) = email.rsplit_once('@') else {
      return false;
   };
   if local_part.is_empty() || local_part.len() > 64 || domain.is_empty() || domain.len() > 255 {
      return false;
   }
   if local_part.starts_with('.') || local_part.ends_with('.') || local_part.contains("..") {
      return false;
   }
   let local_part_is_valid: bool = local_part
      .chars()
      .all(|character| character.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(character));
   let domain_is_valid: bool = domain.split('.').all(|label| {
      !label.is_empty()
         && !label.starts_with('-')
         && !label.ends_with('-')
         && label.chars().all(|character| character.is_ascii_alphanumeric() || character == '-')
   });
   local_part_is_valid && domain_is_valid
}

pub fn is_valid_phone(phone: &str) -> bool {
   let digits: &str = phone.strip_prefix('+').unwrap_or(phone);
   (9..=15).contains(&digits.len()) && digits.chars().all(|character| character.is_ascii_digit())
}

pub fn has_letter_and_digit(password: &str) -> bool {
   password.chars().any(|character| character.is_alphabetic())
      && password.chars().any(|character| character.is_ascii_digit())
}

pub fn is_https_url_without_spaces(url: &str) -> bool {
   url.starts_with("https://")
      && url.len() > "https://".len()
      && !url.chars().any(|character| character.is_whitespace())
}

#[cfg(test)]
mod tests {
   use super::{has_letter_and_digit, is_https_url_without_spaces, is_valid_email, is_valid_phone, FieldErrors};

   #[test]
   fn email_accepts_common_addresses_and_rejects_malformed_ones() {
      assert!(is_valid_email("khach.hang+vip@example.com"));
      assert!(is_valid_email("a@localhost"));
      assert!(!is_valid_email("khongcoa.example.com"));
      assert!(!is_valid_email("@example.com"));
      assert!(!is_valid_email("a@"));
      assert!(!is_valid_email("a b@example.com"));
      assert!(!is_valid_email(".a@example.com"));
      assert!(!is_valid_email("a..b@example.com"));
      assert!(!is_valid_email("a@-example.com"));
      assert!(!is_valid_email("a@example..com"));
   }

   #[test]
   fn phone_accepts_9_to_15_digits_with_optional_plus() {
      assert!(is_valid_phone("0901234567"));
      assert!(is_valid_phone("+84901234567"));
      assert!(!is_valid_phone("12345678"));
      assert!(!is_valid_phone("1234567890123456"));
      assert!(!is_valid_phone("09012a4567"));
      assert!(!is_valid_phone("++84901234567"));
   }

   #[test]
   fn password_needs_letter_and_digit_including_vietnamese_letters() {
      assert!(has_letter_and_digit("matkhau1"));
      assert!(has_letter_and_digit("MậtKhẩu9"));
      assert!(!has_letter_and_digit("chicochu"));
      assert!(!has_letter_and_digit("12345678"));
   }

   #[test]
   fn avatar_url_must_be_https_without_whitespace() {
      assert!(is_https_url_without_spaces("https://cdn.example.com/a.png"));
      assert!(!is_https_url_without_spaces("http://cdn.example.com/a.png"));
      assert!(!is_https_url_without_spaces("https://"));
      assert!(!is_https_url_without_spaces("https://cdn.example.com/a b.png"));
   }

   #[test]
   fn first_message_wins_and_finish_reports_validation_error() {
      let mut field_errors: FieldErrors = FieldErrors::new();
      field_errors.add("email", "Email không được để trống");
      field_errors.add("email", "Email không hợp lệ");
      let app_error = field_errors.finish().unwrap_err();
      assert_eq!(app_error.error_code, "VALIDATION_ERROR");
      assert_eq!(app_error.data.unwrap()["email"], "Email không được để trống");
   }

   #[test]
   fn finish_is_ok_without_errors() {
      assert!(FieldErrors::new().finish().is_ok());
   }

   #[test]
   fn required_text_rejects_missing_and_blank_values() {
      let mut field_errors: FieldErrors = FieldErrors::new();
      assert_eq!(field_errors.required_text("a", &Some("x".to_string()), "thiếu a"), "x");
      field_errors.required_text("b", &None, "thiếu b");
      field_errors.required_text("c", &Some("   ".to_string()), "thiếu c");
      let data = field_errors.finish().unwrap_err().data.unwrap();
      assert!(data.get("a").is_none());
      assert_eq!(data["b"], "thiếu b");
      assert_eq!(data["c"], "thiếu c");
   }
}
