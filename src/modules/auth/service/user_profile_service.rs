use chrono::Utc;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::common::extract::ClientInfo;
use crate::modules::auth::dto::request::ChangePasswordCommand;
use crate::modules::auth::dto::request::UpdateProfileCommand;
use crate::modules::auth::dto::response::BranchAssignmentResponse;
use crate::modules::auth::dto::response::UserProfileResponse;
use crate::modules::auth::entity::UserRecord;
use crate::modules::auth::enums::UserStatus;
use crate::modules::auth::model::AuthSession;
use crate::modules::auth::repository;
use crate::modules::auth::repository::refresh_token_repository;
use crate::modules::auth::repository::role_repository;
use crate::modules::auth::repository::user_branch_assignment_repository;
use crate::modules::auth::repository::user_identity_repository;
use crate::modules::auth::repository::user_repository;
use crate::modules::auth::repository::user_repository::ProfileUpdate;
use crate::modules::auth::security::ensure_password_fits_bcrypt;
use crate::modules::auth::service::auth_service::account_disabled;
use crate::modules::auth::service::auth_service::account_locked;
use crate::modules::auth::service::auth_service::issue_session;
use crate::modules::auth::service::auth_service::lock_duration;
use crate::modules::auth::service::auth_service::IssuedSession;
use crate::modules::auth::service::auth_service::MAX_FAILED_LOGIN_ATTEMPTS;
use crate::state::AppState;

fn session_expired() -> AppError {
   AppError::unauthorized("UNAUTHORIZED", "Phiên đăng nhập không hợp lệ hoặc đã hết hạn")
}

fn phone_already_used() -> AppError {
   AppError::conflict("PHONE_ALREADY_EXISTS", "Số điện thoại đã được sử dụng")
}

fn ensure_usable(user_record: UserRecord) -> Result<UserRecord, AppError> {
   if matches!(user_record.status, UserStatus::Suspended | UserStatus::Deleted) {
      return Err(account_disabled(axum::http::StatusCode::FORBIDDEN));
   }
   Ok(user_record)
}

async fn build_profile_response(
   app_state: &AppState,
   user_record: &UserRecord,
) -> Result<UserProfileResponse, AppError> {
   let pg_pool = &app_state.pg_pool;
   let role_codes: Vec<String> = role_repository::find_role_codes(pg_pool, user_record.id).await?;
   let linked_providers: Vec<String> = user_identity_repository::find_linked_providers(pg_pool, user_record.id).await?;
   let branches: Vec<BranchAssignmentResponse> =
      user_branch_assignment_repository::find_assigned_branches(pg_pool, user_record.id)
         .await?
         .into_iter()
         .map(BranchAssignmentResponse::from)
         .collect();
   Ok(UserProfileResponse::from_record(user_record, &role_codes, linked_providers, branches))
}

pub async fn get_profile(app_state: &AppState, user_id: i64) -> Result<UserProfileResponse, AppError> {
   let user_record: UserRecord =
      ensure_usable(user_repository::find_user_by_id(&app_state.pg_pool, user_id).await?.ok_or_else(session_expired)?)?;
   build_profile_response(app_state, &user_record).await
}

pub async fn update_profile(
   app_state: &AppState,
   user_id: i64,
   update_profile_command: UpdateProfileCommand,
) -> Result<UserProfileResponse, AppError> {
   let pg_pool = &app_state.pg_pool;
   ensure_usable(user_repository::find_user_by_id(pg_pool, user_id).await?.ok_or_else(session_expired)?)?;
   if let Some(phone) = update_profile_command.phone.as_deref() {
      if user_repository::exists_user_by_phone_and_id_not(pg_pool, phone, user_id).await? {
         return Err(phone_already_used());
      }
   }
   let profile_update: ProfileUpdate = ProfileUpdate {
      full_name: &update_profile_command.full_name,
      phone: update_profile_command.phone.as_deref(),
      date_of_birth: update_profile_command.date_of_birth,
      gender: update_profile_command.gender.map(|gender| gender.as_str()),
      avatar_url: update_profile_command.avatar_url.as_deref(),
   };
   if let Err(sqlx_error) = user_repository::update_profile(pg_pool, user_id, &profile_update).await {
      return Err(if repository::is_unique_violation(&sqlx_error) { phone_already_used() } else { sqlx_error.into() });
   }
   let user_record: UserRecord =
      user_repository::find_user_by_id(pg_pool, user_id).await?.ok_or_else(session_expired)?;
   build_profile_response(app_state, &user_record).await
}

pub async fn change_password(
   app_state: &AppState,
   user_id: i64,
   change_password_command: ChangePasswordCommand,
   client_info: &ClientInfo,
) -> Result<AuthSession, AppError> {
   let now = Utc::now();
   let mut transaction = app_state.pg_pool.begin().await?;
   let mut user_record: UserRecord = ensure_usable(
      user_repository::find_user_by_id_for_update(&mut *transaction, user_id).await?.ok_or_else(session_expired)?,
   )?;
   if user_record.status == UserStatus::Locked && !user_record.is_login_allowed(now) {
      return Err(account_locked());
   }
   ensure_password_fits_bcrypt(&change_password_command.new_password)?;

   if let Some(current_password_hash) = user_record.password_hash.clone().filter(|_| user_record.has_password()) {
      let current_password: String = match change_password_command.current_password.clone() {
         Some(current_password) if !current_password.trim().is_empty() => current_password,
         _ => {
            return Err(AppError::bad_request("CURRENT_PASSWORD_REQUIRED", "Vui lòng nhập mật khẩu hiện tại"));
         }
      };
      let current_password_matches: bool =
         app_state.password_service.verify(current_password, current_password_hash.clone()).await;
      if !current_password_matches {
         user_record.register_failed_login(MAX_FAILED_LOGIN_ATTEMPTS, lock_duration(), now);
         user_repository::update_login_state(&mut *transaction, &user_record).await?;
         transaction.commit().await?;
         return Err(AppError::bad_request("INVALID_CURRENT_PASSWORD", "Mật khẩu hiện tại không chính xác"));
      }
      let new_password_is_same: bool =
         app_state.password_service.verify(change_password_command.new_password.clone(), current_password_hash).await;
      if new_password_is_same {
         return Err(AppError::bad_request("NEW_PASSWORD_SAME_AS_CURRENT", "Mật khẩu mới phải khác mật khẩu hiện tại"));
      }
   }

   let password_hash: String = app_state.password_service.hash(change_password_command.new_password).await?;
   user_repository::update_password_after_change(&mut *transaction, user_id, &password_hash, now).await?;
   refresh_token_repository::revoke_all_by_user_id(&mut *transaction, user_id, now).await?;
   user_record.failed_login_attempts = 0;
   user_record.locked_until = None;
   let issued_session: IssuedSession =
      issue_session(app_state, &mut transaction, &user_record, Uuid::new_v4(), client_info, now).await?;
   transaction.commit().await?;
   Ok(issued_session.session)
}
