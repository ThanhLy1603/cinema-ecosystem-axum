use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::common::extract::ClientInfo;
use crate::modules::auth::dto::request::LoginCommand;
use crate::modules::auth::dto::request::RegisterCommand;
use crate::modules::auth::dto::response::UserResponse;
use crate::modules::auth::entity::UserRecord;
use crate::modules::auth::enums::UserStatus;
use crate::modules::auth::model::AuthSession;
use crate::modules::auth::repository;
use crate::modules::auth::repository::refresh_token_repository;
use crate::modules::auth::repository::refresh_token_repository::NewRefreshToken;
use crate::modules::auth::repository::role_repository;
use crate::modules::auth::repository::user_repository;
use crate::modules::auth::security::ensure_password_fits_bcrypt;
use crate::modules::auth::security::generate_random_token;
use crate::modules::auth::security::sha256_hex;
use crate::state::AppState;

pub const REFRESH_IN_PROGRESS_ERROR_CODE: &str = "REFRESH_IN_PROGRESS";
pub const CUSTOMER_ROLE_CODE: &str = "ROLE_CUSTOMER";
pub const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;
const DEVICE_INFO_MAX_LENGTH: usize = 255;
const IP_ADDRESS_MAX_LENGTH: usize = 45;

pub fn lock_duration() -> Duration {
   Duration::minutes(15)
}

fn reuse_grace_period() -> Duration {
   Duration::seconds(10)
}

pub fn bad_credentials() -> AppError {
   AppError::unauthorized("BAD_CREDENTIALS", "Email hoặc mật khẩu không chính xác")
}

fn invalid_refresh_token() -> AppError {
   AppError::unauthorized("INVALID_REFRESH_TOKEN", "Phiên đăng nhập không hợp lệ hoặc đã hết hạn")
}

pub fn account_disabled(status: axum::http::StatusCode) -> AppError {
   AppError::new(status, "ACCOUNT_DISABLED", "Tài khoản đã bị tạm ngưng hoặc không khả dụng")
}

pub fn account_locked() -> AppError {
   AppError::new(
      axum::http::StatusCode::LOCKED,
      "ACCOUNT_LOCKED",
      "Tài khoản tạm thời bị khóa do đăng nhập sai nhiều lần, vui lòng thử lại sau",
   )
}

fn truncate(value: Option<&str>, max_length: usize) -> Option<String> {
   value.map(|text| text.chars().take(max_length).collect())
}

pub struct IssuedSession {
   pub session: AuthSession,
   pub refresh_token_id: i64,
}

pub async fn issue_session(
   app_state: &AppState,
   connection: &mut PgConnection,
   user_record: &UserRecord,
   family_id: Uuid,
   client_info: &ClientInfo,
   now: DateTime<Utc>,
) -> Result<IssuedSession, AppError> {
   let role_codes: Vec<String> = role_repository::find_role_codes(&mut *connection, user_record.id).await?;
   let raw_refresh_token: String = generate_random_token();
   let device_info: Option<String> = truncate(client_info.device_info.as_deref(), DEVICE_INFO_MAX_LENGTH);
   let ip_address: Option<String> = truncate(client_info.ip_address.as_deref(), IP_ADDRESS_MAX_LENGTH);
   let token_hash: String = sha256_hex(&raw_refresh_token);
   let refresh_token_id: i64 = refresh_token_repository::insert_refresh_token(
      &mut *connection,
      &NewRefreshToken {
         user_id: user_record.id,
         token_hash: &token_hash,
         family_id,
         device_info: device_info.as_deref(),
         ip_address: ip_address.as_deref(),
         expires_at: now + Duration::seconds(app_state.app_config.jwt.refresh_token_ttl_seconds),
      },
   )
   .await?;
   let access_token: String = app_state.jwt_service.generate_access_token(
      user_record.id,
      &user_record.email,
      &role_codes,
      now,
      app_state.app_config.jwt.access_token_ttl_seconds,
   )?;
   Ok(IssuedSession {
      session: AuthSession {
         user: UserResponse::from_record(user_record, &role_codes),
         access_token,
         refresh_token: raw_refresh_token,
      },
      refresh_token_id,
   })
}

pub async fn register(app_state: &AppState, register_command: RegisterCommand) -> Result<UserResponse, AppError> {
   ensure_password_fits_bcrypt(&register_command.password)?;
   let pg_pool = &app_state.pg_pool;

   if user_repository::exists_user_by_email(pg_pool, &register_command.email).await? {
      return Err(AppError::conflict("EMAIL_ALREADY_EXISTS", "Email đã được sử dụng"));
   }
   if let Some(phone) = register_command.phone.as_deref() {
      if user_repository::exists_user_by_phone(pg_pool, phone).await? {
         return Err(AppError::conflict("PHONE_ALREADY_EXISTS", "Số điện thoại đã được sử dụng"));
      }
   }
   let customer_role_id: i64 =
      role_repository::find_role_id_by_code(pg_pool, CUSTOMER_ROLE_CODE).await?.ok_or_else(|| {
         tracing::error!("Thiếu role {CUSTOMER_ROLE_CODE} trong bảng roles");
         AppError::internal()
      })?;
   let password_hash: String = app_state.password_service.hash(register_command.password.clone()).await?;

   let mut transaction = pg_pool.begin().await?;
   let user_id: i64 = match user_repository::insert_user(
      &mut *transaction,
      &register_command.email,
      &password_hash,
      &register_command.full_name,
      register_command.phone.as_deref(),
   )
   .await
   {
      Ok(user_id) => user_id,
      Err(sqlx_error) if repository::is_unique_violation(&sqlx_error) => {
         return Err(AppError::conflict("ACCOUNT_ALREADY_EXISTS", "Email hoặc số điện thoại đã được sử dụng"));
      }
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   role_repository::insert_user_role(&mut *transaction, user_id, customer_role_id).await?;
   let user_record: UserRecord =
      user_repository::find_user_by_id(&mut *transaction, user_id).await?.ok_or_else(AppError::internal)?;
   transaction.commit().await?;
   Ok(UserResponse::from_record(&user_record, &[CUSTOMER_ROLE_CODE.to_string()]))
}

pub async fn login(
   app_state: &AppState,
   login_command: LoginCommand,
   client_info: &ClientInfo,
) -> Result<AuthSession, AppError> {
   let now: DateTime<Utc> = Utc::now();
   let mut transaction = app_state.pg_pool.begin().await?;

   let found_user: Option<UserRecord> =
      user_repository::find_user_by_email_for_update(&mut *transaction, &login_command.email).await?;
   let mut user_record: UserRecord = match found_user {
      Some(user_record) if user_record.status != UserStatus::Deleted => user_record,
      _ => {
         transaction.rollback().await?;
         app_state.password_service.verify_against_dummy(login_command.password).await;
         return Err(bad_credentials());
      }
   };

   if user_record.status == UserStatus::Locked && !user_record.is_login_allowed(now) {
      return Err(account_locked());
   }

   let password_matches: bool = match user_record.password_hash.clone().filter(|_| user_record.has_password()) {
      Some(password_hash) => app_state.password_service.verify(login_command.password, password_hash).await,
      None => {
         app_state.password_service.verify_against_dummy(login_command.password).await;
         false
      }
   };
   if !password_matches {
      user_record.register_failed_login(MAX_FAILED_LOGIN_ATTEMPTS, lock_duration(), now);
      user_repository::update_login_state(&mut *transaction, &user_record).await?;
      transaction.commit().await?;
      return Err(bad_credentials());
   }

   if !user_record.is_login_allowed(now) {
      return Err(account_disabled(axum::http::StatusCode::FORBIDDEN));
   }

   user_record.record_successful_login(now);
   user_repository::update_login_state(&mut *transaction, &user_record).await?;
   let issued_session: IssuedSession =
      issue_session(app_state, &mut transaction, &user_record, Uuid::new_v4(), client_info, now).await?;
   transaction.commit().await?;
   Ok(issued_session.session)
}

pub async fn refresh(
   app_state: &AppState,
   raw_refresh_token: Option<String>,
   client_info: &ClientInfo,
) -> Result<AuthSession, AppError> {
   let raw_refresh_token: String = match raw_refresh_token {
      Some(token) if !token.trim().is_empty() => token,
      _ => return Err(invalid_refresh_token()),
   };
   let now: DateTime<Utc> = Utc::now();
   let mut transaction = app_state.pg_pool.begin().await?;

   let current_refresh_token = refresh_token_repository::find_refresh_token_by_hash_for_update(
      &mut *transaction,
      &sha256_hex(&raw_refresh_token),
   )
   .await?
   .ok_or_else(invalid_refresh_token)?;

   if current_refresh_token.revoked {
      let is_within_grace_period: bool = current_refresh_token.replaced_by_id.is_some()
         && current_refresh_token.revoked_at.is_some_and(|revoked_at| now < revoked_at + reuse_grace_period());
      if is_within_grace_period {
         return Err(AppError::unauthorized(
            REFRESH_IN_PROGRESS_ERROR_CODE,
            "Phiên đăng nhập vừa được gia hạn bởi một yêu cầu khác",
         ));
      }
      refresh_token_repository::revoke_family(&mut *transaction, current_refresh_token.family_id, now).await?;
      transaction.commit().await?;
      return Err(AppError::unauthorized(
         "REFRESH_TOKEN_REUSED",
         "Phiên đăng nhập không còn hợp lệ, vui lòng đăng nhập lại",
      ));
   }
   if now >= current_refresh_token.expires_at {
      return Err(invalid_refresh_token());
   }

   let user_record: UserRecord = user_repository::find_user_by_id(&mut *transaction, current_refresh_token.user_id)
      .await?
      .ok_or_else(invalid_refresh_token)?;
   if matches!(user_record.status, UserStatus::Suspended | UserStatus::Deleted) {
      refresh_token_repository::revoke_family(&mut *transaction, current_refresh_token.family_id, now).await?;
      transaction.commit().await?;
      return Err(account_disabled(axum::http::StatusCode::UNAUTHORIZED));
   }

   let issued_session: IssuedSession =
      issue_session(app_state, &mut transaction, &user_record, current_refresh_token.family_id, client_info, now)
         .await?;
   refresh_token_repository::revoke_refresh_token_replaced_by(
      &mut *transaction,
      current_refresh_token.id,
      issued_session.refresh_token_id,
      now,
   )
   .await?;
   transaction.commit().await?;
   Ok(issued_session.session)
}

pub async fn logout(app_state: &AppState, raw_refresh_token: Option<String>) -> Result<(), AppError> {
   let Some(raw_refresh_token) = raw_refresh_token.filter(|token| !token.trim().is_empty()) else {
      return Ok(());
   };
   let pg_pool = &app_state.pg_pool;
   if let Some(refresh_token_record) =
      refresh_token_repository::find_refresh_token_by_hash(pg_pool, &sha256_hex(&raw_refresh_token)).await?
   {
      refresh_token_repository::revoke_family(pg_pool, refresh_token_record.family_id, Utc::now()).await?;
   }
   Ok(())
}

pub async fn logout_all(app_state: &AppState, user_id: i64) -> Result<(), AppError> {
   refresh_token_repository::revoke_all_by_user_id(&app_state.pg_pool, user_id, Utc::now()).await?;
   Ok(())
}
