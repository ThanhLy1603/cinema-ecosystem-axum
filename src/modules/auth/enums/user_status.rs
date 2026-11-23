#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserStatus {
   Active,
   Locked,
   Suspended,
   Deleted,
}

impl UserStatus {
   pub fn as_str(self) -> &'static str {
      match self {
         UserStatus::Active => "ACTIVE",
         UserStatus::Locked => "LOCKED",
         UserStatus::Suspended => "SUSPENDED",
         UserStatus::Deleted => "DELETED",
      }
   }
}

impl TryFrom<String> for UserStatus {
   type Error = String;

   fn try_from(status: String) -> Result<UserStatus, String> {
      match status.as_str() {
         "ACTIVE" => Ok(UserStatus::Active),
         "LOCKED" => Ok(UserStatus::Locked),
         "SUSPENDED" => Ok(UserStatus::Suspended),
         "DELETED" => Ok(UserStatus::Deleted),
         other => Err(format!("trạng thái người dùng không hợp lệ: {other}")),
      }
   }
}
#[cfg(test)]
mod tests {
   use super::UserStatus;

   #[test]
   fn status_round_trips_through_text() {
      for status in [UserStatus::Active, UserStatus::Locked, UserStatus::Suspended, UserStatus::Deleted] {
         assert_eq!(UserStatus::try_from(status.as_str().to_string()).unwrap(), status);
      }
      assert!(UserStatus::try_from("KHAC".to_string()).is_err());
   }
}
