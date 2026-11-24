use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CinemaStatus {
   Active,
   Inactive,
}

impl CinemaStatus {
   pub fn as_str(self) -> &'static str {
      match self {
         CinemaStatus::Active => "ACTIVE",
         CinemaStatus::Inactive => "INACTIVE",
      }
   }
}

impl TryFrom<String> for CinemaStatus {
   type Error = String;

   fn try_from(status: String) -> Result<CinemaStatus, String> {
      match status.as_str() {
         "ACTIVE" => Ok(CinemaStatus::Active),
         "INACTIVE" => Ok(CinemaStatus::Inactive),
         other => Err(format!("trạng thái cụm rạp không hợp lệ: {other}")),
      }
   }
}

#[cfg(test)]
mod tests {
   use super::CinemaStatus;

   #[test]
   fn status_round_trips_through_text_and_json() {
      for status in [CinemaStatus::Active, CinemaStatus::Inactive] {
         assert_eq!(CinemaStatus::try_from(status.as_str().to_string()).unwrap(), status);
         assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
      }
      assert!(CinemaStatus::try_from("DELETED".to_string()).is_err());
   }
}
