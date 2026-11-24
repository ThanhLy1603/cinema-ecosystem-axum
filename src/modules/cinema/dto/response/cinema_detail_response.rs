use serde::{Deserialize, Serialize};

use super::{FacilityResponse, HallResponse};
use crate::modules::cinema::entity::CinemaRecord;
use crate::modules::cinema::enums::CinemaStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CinemaDetailResponse {
   pub id: i64,
   pub name: String,
   pub slug: String,
   pub city: String,
   pub address: String,
   pub phone: Option<String>,
   pub latitude: Option<f64>,
   pub longitude: Option<f64>,
   pub status: CinemaStatus,
   pub facilities: Vec<FacilityResponse>,
   pub halls: Vec<HallResponse>,
   pub deleted: bool,
}

impl CinemaDetailResponse {
   pub fn of(
      cinema_record: CinemaRecord,
      facilities: Vec<FacilityResponse>,
      halls: Vec<HallResponse>,
   ) -> CinemaDetailResponse {
      CinemaDetailResponse {
         id: cinema_record.id,
         name: cinema_record.name,
         slug: cinema_record.slug,
         city: cinema_record.city,
         address: cinema_record.address,
         phone: cinema_record.phone,
         latitude: cinema_record.latitude,
         longitude: cinema_record.longitude,
         status: cinema_record.status,
         facilities,
         halls,
         deleted: cinema_record.is_deleted,
      }
   }
}
