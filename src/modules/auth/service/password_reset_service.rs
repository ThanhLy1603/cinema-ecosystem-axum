use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};

use crate::common::error::AppError;
use crate::common::rate_limit;
use crate::modules::auth::dto::request::ForgotPasswordCommand;
use crate::modules::auth::dto::request::ResetPasswordCommand;
use crate::modules::auth::entity::UserRecord;
use crate::modules::auth::enums::UserStatus;
use crate::modules::auth::enums::UserTokenType;
use crate::modules::auth::repository::refresh_token_repository;
use crate::modules::auth::repository::user_repository;
use crate::modules::auth::repository::user_token_repository;
use crate::modules::auth::security::ensure_password_fits_bcrypt;
use crate::modules::auth::security::sha256_hex;
use crate::modules::auth::service::auth_mail_service;
use crate::modules::auth::service::one_time_token_service;
use crate::state::AppState;

const MAX_REQUESTS_PER_WINDOW: i64 = 3;

fn token_ttl() -> Duration {
   Duration::minutes(30)
}

fn rate_limit_window() -> StdDuration {
   StdDuration::from_secs(60 * 60)
}

fn invalid_or_expired_token() -> AppError {
   AppError::bad_request("INVALID_OR_EXPIRED_TOKEN", "Liên kết đặt lại mật khẩu không hợp lệ hoặc đã hết hạn")
}

fn can_reset_password(user_record: &UserRecord) -> bool {
   matches!(user_record.status, UserStatus::Active | UserStatus::Locked)
}

pub async fn request_reset(
   app_state: &AppState,
   forgot_password_command: ForgotPasswordCommand,
   ip_address: Option<&str>,
) -> Result<(), AppError> {
   let rate_limit_key: String = format!("forgot-password:{}", sha256_hex(&forgot_password_command.email));
   if !rate_limit::try_acquire(&app_state.redis_client, &rate_limit_key, MAX_REQUESTS_PER_WINDOW, rate_limit_window())
      .await
   {
      return Ok(());
   }
   let Some(user_record) =
      user_repository::find_user_by_email(&app_state.pg_pool, &forgot_password_command.email).await?
   else {
      return Ok(());
   };
   if !can_reset_password(&user_record) {
      return Ok(());
   }
   let mut transaction = app_state.pg_pool.begin().await?;
   let raw_token: String = one_time_token_service::issue(
      &mut transaction,
      user_record.id,
      UserTokenType::PasswordReset,
      token_ttl(),
      ip_address,
      Utc::now(),
   )
   .await?;
   transaction.commit().await?;
   auth_mail_service::send_password_reset(
      &app_state.mailer,
      &app_state.app_config.frontend_base_url,
      &user_record.email,
      &user_record.full_name,
      &raw_token,
   );
   Ok(())
}

pub async fn reset_password(
   app_state: &AppState,
   reset_password_command: ResetPasswordCommand,
) -> Result<(), AppError> {
   let now = Utc::now();
   let mut transaction = app_state.pg_pool.begin().await?;
   let user_token_record = one_time_token_service::find_usable_for_update(
      &mut transaction,
      &reset_password_command.token,
      UserTokenType::PasswordReset,
      now,
   )
   .await?
   .ok_or_else(invalid_or_expired_token)?;
   let user_record: UserRecord =
      user_repository::find_user_by_id_for_update(&mut *transaction, user_token_record.user_id)
         .await?
         .ok_or_else(invalid_or_expired_token)?;
   if !can_reset_password(&user_record) {
      return Err(invalid_or_expired_token());
   }
   ensure_password_fits_bcrypt(&reset_password_command.new_password)?;
   let password_hash: String = app_state.password_service.hash(reset_password_command.new_password).await?;
   user_repository::update_password_after_reset(&mut *transaction, user_record.id, &password_hash, now).await?;
   user_token_repository::mark_user_token_used(&mut *transaction, user_token_record.id, now).await?;
   refresh_token_repository::revoke_all_by_user_id(&mut *transaction, user_record.id, now).await?;
   transaction.commit().await?;
   Ok(())
}
