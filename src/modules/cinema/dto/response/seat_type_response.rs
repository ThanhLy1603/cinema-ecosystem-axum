use serde::{Deserialize, Serialize};

use crate::modules::cinema::entity::SeatTypeRecord;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeatTypeResponse {
   pub id: i64,
   pub code: String,
   pub name: String,
   pub default_surcharge: f64,
   pub span: i32,
}

impl From<SeatTypeRecord> for SeatTypeResponse {
   fn from(seat_type_record: SeatTypeRecord) -> SeatTypeResponse {
      SeatTypeResponse {
         id: seat_type_record.id,
         code: seat_type_record.code,
         name: seat_type_record.name,
         default_surcharge: seat_type_record.default_surcharge,
         span: seat_type_record.seat_span,
      }
   }
}
