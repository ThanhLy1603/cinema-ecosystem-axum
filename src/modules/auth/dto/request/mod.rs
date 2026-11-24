mod change_password_request;
mod forgot_password_request;
mod google_login_request;
mod login_request;
mod register_request;
mod reset_password_request;
mod update_profile_request;
mod validation_rules;
mod verify_email_request;

pub use change_password_request::{ChangePasswordCommand, ChangePasswordRequest};
pub use forgot_password_request::{ForgotPasswordCommand, ForgotPasswordRequest};
pub use google_login_request::{GoogleLoginCommand, GoogleLoginRequest};
pub use login_request::{LoginCommand, LoginRequest};
pub use register_request::{RegisterCommand, RegisterRequest};
pub use reset_password_request::{ResetPasswordCommand, ResetPasswordRequest};
pub use update_profile_request::{UpdateProfileCommand, UpdateProfileRequest};
pub use verify_email_request::{VerifyEmailCommand, VerifyEmailRequest};

#[cfg(test)]
pub fn field_errors_of<T>(result: Result<T, crate::common::error::AppError>) -> serde_json::Value {
   match result {
      Ok(_) => panic!("dữ liệu hợp lệ nhưng test mong đợi lỗi"),
      Err(app_error) => {
         assert_eq!(app_error.error_code, "VALIDATION_ERROR");
         app_error.data.unwrap()
      }
   }
}
