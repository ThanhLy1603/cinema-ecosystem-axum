use std::collections::HashMap;

use sqlx::PgExecutor;

use crate::modules::cinema::dto::response::{CinemaDetailResponse, CinemaResponse, FacilityResponse, HallResponse};
use crate::modules::cinema::entity::{CinemaFacilityRecord, CinemaRecord, HallRecord};
use crate::modules::cinema::repository::{cinema_repository, hall_repository};

pub async fn assemble_cinema_responses(
   executor: impl PgExecutor<'_> + Copy,
   cinema_records: Vec<CinemaRecord>,
   active_halls_only: bool,
) -> Result<Vec<CinemaResponse>, sqlx::Error> {
   if cinema_records.is_empty() {
      return Ok(Vec::new());
   }
   let cinema_ids: Vec<i64> = cinema_records.iter().map(|cinema_record| cinema_record.id).collect();
   let mut facilities_by_cinema_id: HashMap<i64, Vec<FacilityResponse>> = HashMap::new();
   for cinema_facility_record in cinema_repository::find_facilities(executor, &cinema_ids).await? {
      facilities_by_cinema_id
         .entry(cinema_facility_record.cinema_id)
         .or_default()
         .push(FacilityResponse::from(cinema_facility_record));
   }
   let hall_counts: HashMap<i64, i64> =
      cinema_repository::count_halls(executor, &cinema_ids, active_halls_only).await?.into_iter().collect();
   Ok(cinema_records
      .into_iter()
      .map(|cinema_record| {
         let facilities: Vec<FacilityResponse> = facilities_by_cinema_id.remove(&cinema_record.id).unwrap_or_default();
         let hall_count: i32 = hall_counts.get(&cinema_record.id).copied().unwrap_or(0) as i32;
         CinemaResponse::of(cinema_record, facilities, hall_count)
      })
      .collect())
}

pub async fn assemble_cinema_detail(
   executor: impl PgExecutor<'_> + Copy,
   cinema_record: CinemaRecord,
   active_halls_only: bool,
) -> Result<CinemaDetailResponse, sqlx::Error> {
   let facility_records: Vec<CinemaFacilityRecord> =
      cinema_repository::find_facilities(executor, &[cinema_record.id]).await?;
   let hall_records: Vec<HallRecord> = if active_halls_only {
      hall_repository::find_active_by_cinema(executor, cinema_record.id).await?
   } else {
      hall_repository::find_by_cinema(executor, cinema_record.id, false).await?
   };
   Ok(CinemaDetailResponse::of(
      cinema_record,
      facility_records.into_iter().map(FacilityResponse::from).collect(),
      hall_records.into_iter().map(HallResponse::from).collect(),
   ))
}
