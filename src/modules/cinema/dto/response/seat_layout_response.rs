use serde::{Deserialize, Serialize};

use super::SeatLayoutSeatResponse;
use crate::modules::cinema::entity::{HallRecord, SeatRecord};
use crate::modules::cinema::enums::{HallStatus, ScreenType};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeatLayoutResponse {
   pub hall_id: i64,
   pub cinema_id: i64,
   pub hall_name: String,
   pub screen_type: ScreenType,
   pub hall_status: HallStatus,
   pub version: i64,
   pub rows: Vec<String>,
   pub columns: i32,
   pub seats: Vec<SeatLayoutSeatResponse>,
}

impl SeatLayoutResponse {
   pub fn of(hall_record: HallRecord, mut seat_records: Vec<SeatRecord>) -> SeatLayoutResponse {
      seat_records.sort_by(|left, right| {
         left.seat_row.cmp(&right.seat_row).then_with(|| left.seat_number.cmp(&right.seat_number))
      });
      let mut rows: Vec<String> = Vec::new();
      for seat_record in &seat_records {
         if rows.last() != Some(&seat_record.seat_row) {
            rows.push(seat_record.seat_row.clone());
         }
      }
      let columns: i32 =
         seat_records.iter().map(|seat_record| seat_record.seat_number + seat_record.seat_span - 1).max().unwrap_or(0);
      SeatLayoutResponse {
         hall_id: hall_record.id,
         cinema_id: hall_record.cinema_id,
         hall_name: hall_record.name,
         screen_type: hall_record.screen_type,
         hall_status: hall_record.status,
         version: hall_record.version,
         rows,
         columns,
         seats: seat_records.into_iter().map(SeatLayoutSeatResponse::from).collect(),
      }
   }
}
