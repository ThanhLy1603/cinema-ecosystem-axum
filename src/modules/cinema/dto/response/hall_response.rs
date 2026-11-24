use serde::{Deserialize, Serialize};

use crate::modules::cinema::entity::HallRecord;
use crate::modules::cinema::enums::{HallStatus, ScreenType};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HallResponse {
   pub id: i64,
   pub cinema_id: i64,
   pub name: String,
   pub screen_type: ScreenType,
   pub total_seats: i32,
   pub status: HallStatus,
   pub deleted: bool,
}

impl From<HallRecord> for HallResponse {
   fn from(hall_record: HallRecord) -> HallResponse {
      HallResponse {
         id: hall_record.id,
         cinema_id: hall_record.cinema_id,
         name: hall_record.name,
         screen_type: hall_record.screen_type,
         total_seats: hall_record.total_seats,
         status: hall_record.status,
         deleted: hall_record.is_deleted,
      }
   }
}
