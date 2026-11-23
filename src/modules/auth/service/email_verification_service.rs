use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};

use crate::common::error::AppError;
use crate::common::rate_limit;
use crate::modules::auth::dto::request::VerifyEmailCommand;
use crate::modules::auth::entity::UserRecord;
use crate::modules::auth::enums::UserStatus;
use crate::modules::auth::enums::UserTokenType;
use crate::modules::auth::repository::user_repository;
use crate::modules::auth::repository::user_token_repository;
use crate::modules::auth::service::auth_mail_service;
use crate::modules::auth::service::one_time_token_service;
use crate::state::AppState;

const MAX_RESENDS_PER_WINDOW: i64 = 3;

fn token_ttl() -> Duration {
   Duration::hours(24)
}

fn rate_limit_window() -> StdDuration {
   StdDuration::from_secs(60 * 60)
}

fn invalid_or_expired_token() -> AppError {
   AppError::bad_request("INVALID_OR_EXPIRED_TOKEN", "Liên kết xác thực email không hợp lệ hoặc đã hết hạn")
}

fn session_expired() -> AppError {
   AppError::unauthorized("UNAUTHORIZED", "Phiên đăng nhập không hợp lệ hoặc đã hết hạn")
}

async fn load_user(app_state: &AppState, user_id: i64) -> Result<UserRecord, AppError> {
   user_repository::find_user_by_id(&app_state.pg_pool, user_id)
      .await?
      .filter(|user_record| user_record.status != UserStatus::Deleted)
      .ok_or_else(session_expired)
}

pub async fn send_verification(app_state: &AppState, user_id: i64, ip_address: Option<&str>) -> Result<(), AppError> {
   let user_record: UserRecord = load_user(app_state, user_id).await?;
   if user_record.email_verified {
      return Ok(());
   }
   let mut transaction = app_state.pg_pool.begin().await?;
   let raw_token: String = one_time_token_service::issue(
      &mut transaction,
      user_record.id,
      UserTokenType::EmailVerification,
      token_ttl(),
      ip_address,
      Utc::now(),
   )
   .await?;
   transaction.commit().await?;
   auth_mail_service::send_email_verification(
      &app_state.mailer,
      &app_state.app_config.frontend_base_url,
      &user_record.email,
      &user_record.full_name,
      &raw_token,
   );
   Ok(())
}

pub async fn resend_verification(app_state: &AppState, user_id: i64, ip_address: Option<&str>) -> Result<(), AppError> {
   let user_record: UserRecord = load_user(app_state, user_id).await?;
   if user_record.email_verified {
      return Err(AppError::conflict("EMAIL_ALREADY_VERIFIED", "Email của bạn đã được xác thực"));
   }
   let rate_limit_key: String = format!("resend-verification:{user_id}");
   if !rate_limit::try_acquire(&app_state.redis_client, &rate_limit_key, MAX_RESENDS_PER_WINDOW, rate_limit_window())
      .await
   {
      return Err(AppError::new(
         axum::http::StatusCode::TOO_MANY_REQUESTS,
         "TOO_MANY_REQUESTS",
         "Bạn đã yêu cầu gửi lại quá nhiều lần, vui lòng thử lại sau",
      ));
   }
   send_verification(app_state, user_id, ip_address).await
}

pub async fn verify_email(app_state: &AppState, verify_email_command: VerifyEmailCommand) -> Result<(), AppError> {
   let now = Utc::now();
   let mut transaction = app_state.pg_pool.begin().await?;
   let user_token_record = one_time_token_service::find_usable_for_update(
      &mut transaction,
      &verify_email_command.token,
      UserTokenType::EmailVerification,
      now,
   )
   .await?
   .ok_or_else(invalid_or_expired_token)?;
   let user_record: UserRecord = user_repository::find_user_by_id(&mut *transaction, user_token_record.user_id)
      .await?
      .ok_or_else(invalid_or_expired_token)?;
   if user_record.status == UserStatus::Deleted {
      return Err(invalid_or_expired_token());
   }
   user_repository::mark_email_verified(&mut *transaction, user_record.id).await?;
   user_token_repository::mark_user_token_used(&mut *transaction, user_token_record.id, now).await?;
   transaction.commit().await?;
   Ok(())
}
