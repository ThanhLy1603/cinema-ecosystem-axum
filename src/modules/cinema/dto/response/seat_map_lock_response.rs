use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeatMapLockResponse {
   pub locked: bool,
   pub upcoming_showtime_count: i64,
   pub booked_showtime_count: i64,
}
