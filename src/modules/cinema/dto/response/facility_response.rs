use serde::{Deserialize, Serialize};

use crate::modules::cinema::entity::{CinemaFacilityRecord, FacilityRecord};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FacilityResponse {
   pub id: i64,
   pub name: String,
   pub icon_url: Option<String>,
   pub description: Option<String>,
   pub deleted: bool,
}

impl From<FacilityRecord> for FacilityResponse {
   fn from(facility_record: FacilityRecord) -> FacilityResponse {
      FacilityResponse {
         id: facility_record.id,
         name: facility_record.name,
         icon_url: facility_record.icon_url,
         description: facility_record.description,
         deleted: facility_record.is_deleted,
      }
   }
}

impl From<CinemaFacilityRecord> for FacilityResponse {
   fn from(cinema_facility_record: CinemaFacilityRecord) -> FacilityResponse {
      FacilityResponse {
         id: cinema_facility_record.id,
         name: cinema_facility_record.name,
         icon_url: cinema_facility_record.icon_url,
         description: cinema_facility_record.description,
         deleted: cinema_facility_record.is_deleted,
      }
   }
}
