#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserTokenType {
   PasswordReset,
   EmailVerification,
}

impl UserTokenType {
   pub fn as_str(self) -> &'static str {
      match self {
         UserTokenType::PasswordReset => "PASSWORD_RESET",
         UserTokenType::EmailVerification => "EMAIL_VERIFICATION",
      }
   }
}
