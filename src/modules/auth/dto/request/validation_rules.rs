use crate::common::validation::{has_letter_and_digit, FieldErrors};

const PASSWORD_LENGTH_MESSAGE: &str = "Mật khẩu phải từ 8 đến 72 ký tự";
const PASSWORD_PATTERN_MESSAGE: &str = "Mật khẩu phải gồm cả chữ và số";
pub const PHONE_MESSAGE: &str = "Số điện thoại không hợp lệ";

pub fn check_new_password(field_errors: &mut FieldErrors, field: &str, password: &str) {
   field_errors.min_chars(field, password, 8, PASSWORD_LENGTH_MESSAGE);
   field_errors.max_chars(field, password, 72, PASSWORD_LENGTH_MESSAGE);
   if !has_letter_and_digit(password) {
      field_errors.add(field, PASSWORD_PATTERN_MESSAGE);
   }
}
