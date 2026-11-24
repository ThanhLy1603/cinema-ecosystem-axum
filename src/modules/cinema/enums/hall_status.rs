use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HallStatus {
   Active,
   Maintenance,
}

impl HallStatus {
   pub fn as_str(self) -> &'static str {
      match self {
         HallStatus::Active => "ACTIVE",
         HallStatus::Maintenance => "MAINTENANCE",
      }
   }
}

impl TryFrom<String> for HallStatus {
   type Error = String;

   fn try_from(status: String) -> Result<HallStatus, String> {
      match status.as_str() {
         "ACTIVE" => Ok(HallStatus::Active),
         "MAINTENANCE" => Ok(HallStatus::Maintenance),
         other => Err(format!("trạng thái phòng chiếu không hợp lệ: {other}")),
      }
   }
}
