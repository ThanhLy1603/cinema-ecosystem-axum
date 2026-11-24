use crate::common::error::AppError;
use crate::modules::cinema::dto::response::{
   CinemaDetailResponse, CinemaResponse, SeatLayoutResponse, SeatTypeResponse,
};
use crate::modules::cinema::entity::{CinemaRecord, HallRecord, SeatRecord};
use crate::modules::cinema::enums::{CinemaStatus, HallStatus};
use crate::modules::cinema::repository::{cinema_repository, hall_repository, seat_repository, seat_type_repository};
use crate::modules::cinema::service::cinema_access_guard::{cinema_not_found, hall_not_found};
use crate::modules::cinema::service::cinema_assembler::{assemble_cinema_detail, assemble_cinema_responses};
use crate::modules::cinema::service::public_cinema_cache::PUBLIC_CINEMA_CACHE;
use crate::state::AppState;

const ALL_CITIES_CACHE_SUFFIX: &str = "*";

pub async fn find_public_cities(app_state: &AppState) -> Result<Vec<String>, AppError> {
   PUBLIC_CINEMA_CACHE
      .get_or_load(&app_state.redis_client, "cities", || async {
         Ok::<Vec<String>, AppError>(cinema_repository::find_active_cities(&app_state.pg_pool).await?)
      })
      .await
}

pub async fn find_public_cinemas(app_state: &AppState, city: Option<String>) -> Result<Vec<CinemaResponse>, AppError> {
   let normalized_city: Option<String> = city.map(|city| city.trim().to_string()).filter(|city| !city.is_empty());
   if let Some(city) = normalized_city.as_deref() {
      if !find_public_cities(app_state).await?.iter().any(|public_city| public_city == city) {
         return Ok(Vec::new());
      }
   }
   let cache_name: String = format!("list:{}", normalized_city.as_deref().unwrap_or(ALL_CITIES_CACHE_SUFFIX));
   PUBLIC_CINEMA_CACHE
      .get_or_load(&app_state.redis_client, &cache_name, || async {
         let cinema_records: Vec<CinemaRecord> =
            cinema_repository::find_all_active(&app_state.pg_pool, normalized_city.as_deref()).await?;
         Ok::<Vec<CinemaResponse>, AppError>(assemble_cinema_responses(&app_state.pg_pool, cinema_records, true).await?)
      })
      .await
}

pub async fn find_public_cinema_by_slug(app_state: &AppState, slug: &str) -> Result<CinemaDetailResponse, AppError> {
   PUBLIC_CINEMA_CACHE
      .get_or_load(&app_state.redis_client, &format!("detail:{slug}"), || async {
         let cinema_record: CinemaRecord =
            cinema_repository::find_active_by_slug(&app_state.pg_pool, slug).await?.ok_or_else(cinema_not_found)?;
         Ok::<CinemaDetailResponse, AppError>(assemble_cinema_detail(&app_state.pg_pool, cinema_record, true).await?)
      })
      .await
}

pub async fn find_seat_types(app_state: &AppState) -> Result<Vec<SeatTypeResponse>, AppError> {
   Ok(seat_type_repository::find_all(&app_state.pg_pool).await?.into_iter().map(SeatTypeResponse::from).collect())
}

pub async fn find_public_seat_layout(app_state: &AppState, hall_id: i64) -> Result<SeatLayoutResponse, AppError> {
   PUBLIC_CINEMA_CACHE
      .get_or_load(&app_state.redis_client, &format!("seat-layout:{hall_id}"), || async {
         let hall_record: HallRecord = hall_repository::find_by_id(&app_state.pg_pool, hall_id, false)
            .await?
            .filter(|hall_record| hall_record.status == HallStatus::Active)
            .ok_or_else(hall_not_found)?;
         let cinema_is_active: bool = cinema_repository::find_by_id(&app_state.pg_pool, hall_record.cinema_id, false)
            .await?
            .is_some_and(|cinema_record| cinema_record.status == CinemaStatus::Active);
         if !cinema_is_active {
            return Err(hall_not_found());
         }
         let seat_records: Vec<SeatRecord> = seat_repository::find_active_by_hall(&app_state.pg_pool, hall_id).await?;
         Ok::<SeatLayoutResponse, AppError>(SeatLayoutResponse::of(hall_record, seat_records))
      })
      .await
}
