use serde::{Deserialize, Serialize};

use crate::modules::cinema::entity::SeatRecord;
use crate::modules::cinema::enums::SeatStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeatLayoutSeatResponse {
   pub id: i64,
   pub row: String,
   pub number: i32,
   #[serde(rename = "type")]
   pub seat_type_code: String,
   pub surcharge: f64,
   pub span: i32,
   pub status: SeatStatus,
}

impl From<SeatRecord> for SeatLayoutSeatResponse {
   fn from(seat_record: SeatRecord) -> SeatLayoutSeatResponse {
      SeatLayoutSeatResponse {
         id: seat_record.id,
         row: seat_record.seat_row,
         number: seat_record.seat_number,
         seat_type_code: seat_record.seat_type_code,
         surcharge: seat_record.surcharge,
         span: seat_record.seat_span,
         status: seat_record.status,
      }
   }
}
