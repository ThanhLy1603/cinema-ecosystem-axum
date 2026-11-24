use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeatStatus {
   Active,
   Maintenance,
}

impl SeatStatus {
   pub fn as_str(self) -> &'static str {
      match self {
         SeatStatus::Active => "ACTIVE",
         SeatStatus::Maintenance => "MAINTENANCE",
      }
   }
}

impl TryFrom<String> for SeatStatus {
   type Error = String;

   fn try_from(status: String) -> Result<SeatStatus, String> {
      match status.as_str() {
         "ACTIVE" => Ok(SeatStatus::Active),
         "MAINTENANCE" => Ok(SeatStatus::Maintenance),
         other => Err(format!("trạng thái ghế không hợp lệ: {other}")),
      }
   }
}
