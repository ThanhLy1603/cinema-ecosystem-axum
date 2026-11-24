use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::common::error::AppError;
use crate::modules::cinema::dto::request::{GenerateSeatsCommand, SeatCellCommand, SeatMapCommand};
use crate::modules::cinema::dto::response::{SeatLayoutResponse, SeatMapLockResponse};
use crate::modules::cinema::entity::{HallRecord, SeatRecord, SeatTypeRecord};
use crate::modules::cinema::enums::SeatStatus;
use crate::modules::cinema::repository::seat_repository::{SeatInsert, SeatUpdate};
use crate::modules::cinema::repository::{
   hall_repository, hall_usage_repository, seat_repository, seat_type_repository,
};
use crate::modules::cinema::service::cinema_access_guard::{hall_not_found, require_accessible_hall};
use crate::modules::cinema::service::public_cinema_cache::PUBLIC_CINEMA_CACHE;
use crate::modules::cinema::service::seat_map_validator::{self, invalid_seat_type, label};
use crate::state::AppState;

const MAX_ROWS: i64 = 26;

fn seat_map_conflict() -> AppError {
   AppError::conflict(
      "SEAT_MAP_CONFLICT",
      "Sơ đồ ghế đã được người khác cập nhật, hãy tải lại trang để lấy bản mới nhất",
   )
}

fn seat_map_locked(upcoming_showtime_count: i64) -> AppError {
   AppError::conflict(
      "SEAT_MAP_LOCKED",
      &format!(
         "Không thể sửa sơ đồ ghế vì phòng chiếu còn {upcoming_showtime_count} suất chiếu chưa kết thúc \
          (đang mở bán hoặc đang chiếu). Hãy chờ các suất này kết thúc, hoặc hủy các suất chưa có khách đặt vé ở trang Suất chiếu \
          rồi chỉnh sửa lại"
      ),
   )
}

pub async fn get_seat_map(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
) -> Result<SeatLayoutResponse, AppError> {
   let hall_record: HallRecord = require_accessible_hall(app_state, actor_user_id, hall_id).await?;
   let seat_records: Vec<SeatRecord> = seat_repository::find_active_by_hall(&app_state.pg_pool, hall_id).await?;
   Ok(SeatLayoutResponse::of(hall_record, seat_records))
}

pub async fn get_seat_map_lock(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
) -> Result<SeatMapLockResponse, AppError> {
   let hall_record: HallRecord = require_accessible_hall(app_state, actor_user_id, hall_id).await?;
   let now: DateTime<Utc> = Utc::now();
   let upcoming_showtime_count: i64 =
      hall_usage_repository::count_upcoming_showtimes_by_hall(&app_state.pg_pool, hall_record.id, now).await?;
   let booked_showtime_count: i64 = if upcoming_showtime_count == 0 {
      0
   } else {
      hall_usage_repository::count_showtimes_with_bookings_by_hall(&app_state.pg_pool, hall_record.id, now).await?
   };
   Ok(SeatMapLockResponse { locked: upcoming_showtime_count > 0, upcoming_showtime_count, booked_showtime_count })
}

pub async fn save_seat_map(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
   seat_map_command: SeatMapCommand,
) -> Result<SeatLayoutResponse, AppError> {
   require_accessible_hall(app_state, actor_user_id, hall_id).await?;
   apply_seat_cells(app_state, hall_id, Some(seat_map_command.version), seat_map_command.seats, false).await
}

pub async fn generate_seats(
   app_state: &AppState,
   actor_user_id: i64,
   hall_id: i64,
   generate_seats_command: GenerateSeatsCommand,
) -> Result<SeatLayoutResponse, AppError> {
   require_accessible_hall(app_state, actor_user_id, hall_id).await?;
   if seat_repository::count_active_by_hall(&app_state.pg_pool, hall_id).await? > 0 {
      return Err(AppError::conflict(
         "HALL_ALREADY_HAS_SEATS",
         "Phòng chiếu đã có sơ đồ ghế, hãy chỉnh sửa trực tiếp trên sơ đồ",
      ));
   }
   let seat_types_by_code: HashMap<String, SeatTypeRecord> = load_seat_types_by_code(app_state).await?;
   let seat_type_record: &SeatTypeRecord = seat_types_by_code
      .get(&generate_seats_command.seat_type_code)
      .ok_or_else(|| invalid_seat_type(&generate_seats_command.seat_type_code))?;
   let mut seat_cell_commands: Vec<SeatCellCommand> = Vec::new();
   for row_index in 0..generate_seats_command.row_count.min(MAX_ROWS) {
      let row: String = ((b'A' + row_index as u8) as char).to_string();
      let mut number: i64 = 1;
      while number + i64::from(seat_type_record.seat_span) - 1 <= generate_seats_command.column_count {
         seat_cell_commands.push(SeatCellCommand {
            row: row.clone(),
            number: number as i32,
            seat_type_code: seat_type_record.code.clone(),
            status: SeatStatus::Active,
         });
         number += i64::from(seat_type_record.seat_span);
      }
   }
   apply_seat_cells(app_state, hall_id, None, seat_cell_commands, true).await
}

async fn load_seat_types_by_code(app_state: &AppState) -> Result<HashMap<String, SeatTypeRecord>, AppError> {
   Ok(seat_type_repository::find_all(&app_state.pg_pool)
      .await?
      .into_iter()
      .map(|seat_type_record| (seat_type_record.code.clone(), seat_type_record))
      .collect())
}

async fn apply_seat_cells(
   app_state: &AppState,
   hall_id: i64,
   expected_version: Option<i64>,
   seat_cell_commands: Vec<SeatCellCommand>,
   require_empty_hall: bool,
) -> Result<SeatLayoutResponse, AppError> {
   let mut transaction = app_state.pg_pool.begin().await?;
   let hall_record: HallRecord =
      hall_repository::find_active_by_id_for_update(&mut *transaction, hall_id).await?.ok_or_else(hall_not_found)?;
   if expected_version.is_some_and(|version| version != hall_record.version) {
      return Err(seat_map_conflict());
   }
   if require_empty_hall && seat_repository::count_active_by_hall(&mut *transaction, hall_id).await? > 0 {
      return Err(AppError::conflict(
         "HALL_ALREADY_HAS_SEATS",
         "Phòng chiếu đã có sơ đồ ghế, hãy chỉnh sửa trực tiếp trên sơ đồ",
      ));
   }
   let now: DateTime<Utc> = Utc::now();
   let upcoming_showtime_count: i64 =
      hall_usage_repository::count_upcoming_showtimes_by_hall(&mut *transaction, hall_id, now).await?;
   if upcoming_showtime_count > 0 {
      return Err(seat_map_locked(upcoming_showtime_count));
   }
   let seat_types_by_code: HashMap<String, SeatTypeRecord> = load_seat_types_by_code(app_state).await?;
   seat_map_validator::validate(&seat_cell_commands, &seat_types_by_code)?;

   let existing_seat_records: Vec<SeatRecord> =
      seat_repository::find_active_by_hall(&mut *transaction, hall_id).await?;
   let existing_seat_records_by_label: HashMap<String, &SeatRecord> = existing_seat_records
      .iter()
      .map(|seat_record| (label(&seat_record.seat_row, seat_record.seat_number), seat_record))
      .collect();
   let requested_labels: HashSet<String> = seat_cell_commands
      .iter()
      .map(|seat_cell_command| label(&seat_cell_command.row, seat_cell_command.number))
      .collect();
   let removed_seat_ids: Vec<i64> = existing_seat_records
      .iter()
      .filter(|seat_record| !requested_labels.contains(&label(&seat_record.seat_row, seat_record.seat_number)))
      .map(|seat_record| seat_record.id)
      .collect();
   if !removed_seat_ids.is_empty() {
      seat_repository::mark_deleted(&mut *transaction, &removed_seat_ids, now).await?;
   }

   let mut seat_inserts: Vec<SeatInsert> = Vec::new();
   let mut seat_updates: Vec<SeatUpdate> = Vec::new();
   for seat_cell_command in &seat_cell_commands {
      let seat_type_record: &SeatTypeRecord = &seat_types_by_code[&seat_cell_command.seat_type_code];
      match existing_seat_records_by_label.get(&label(&seat_cell_command.row, seat_cell_command.number)) {
         Some(existing_seat_record) => {
            if existing_seat_record.seat_type_code != seat_cell_command.seat_type_code
               || existing_seat_record.status != seat_cell_command.status
            {
               seat_updates.push(SeatUpdate {
                  seat_id: existing_seat_record.id,
                  seat_type_id: seat_type_record.id,
                  status: seat_cell_command.status.as_str().to_string(),
               });
            }
         }
         None => seat_inserts.push(SeatInsert {
            seat_type_id: seat_type_record.id,
            seat_row: seat_cell_command.row.clone(),
            seat_number: seat_cell_command.number,
            status: seat_cell_command.status.as_str().to_string(),
         }),
      }
   }
   if !seat_updates.is_empty() {
      seat_repository::update_all(&mut *transaction, &seat_updates).await?;
   }
   if !seat_inserts.is_empty() {
      seat_repository::insert_all(&mut *transaction, hall_id, &seat_inserts).await?;
   }
   let saved_hall_record: HallRecord =
      hall_repository::update_seat_map_summary(&mut *transaction, hall_id, seat_cell_commands.len() as i32, now)
         .await?;
   let seat_records: Vec<SeatRecord> = seat_repository::find_active_by_hall(&mut *transaction, hall_id).await?;
   transaction.commit().await?;
   PUBLIC_CINEMA_CACHE.evict(&app_state.redis_client).await;
   Ok(SeatLayoutResponse::of(saved_hall_record, seat_records))
}
