use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::common::extract::ClientInfo;
use crate::modules::auth::dto::request::GoogleLoginCommand;
use crate::modules::auth::entity::UserRecord;
use crate::modules::auth::enums::UserStatus;
use crate::modules::auth::model::{AuthSession, GoogleProfile};
use crate::modules::auth::repository;
use crate::modules::auth::repository::role_repository;
use crate::modules::auth::repository::user_identity_repository;
use crate::modules::auth::repository::user_repository;
use crate::modules::auth::service::auth_service::{account_disabled, issue_session, CUSTOMER_ROLE_CODE};
use crate::state::AppState;

const EMAIL_MAX_LENGTH: usize = 150;
const FULL_NAME_MAX_LENGTH: usize = 150;
const SECURE_URL_PREFIX: &str = "https://";

fn account_locked_message() -> AppError {
   AppError::new(StatusCode::LOCKED, "ACCOUNT_LOCKED", "Tài khoản tạm thời bị khóa, vui lòng thử lại sau")
}

fn full_name_of(google_profile: &GoogleProfile, email: &str) -> String {
   let trimmed_name: &str = google_profile.full_name.as_deref().map(str::trim).unwrap_or("");
   let full_name: &str = if trimmed_name.is_empty() { email.split('@').next().unwrap_or(email) } else { trimmed_name };
   full_name.chars().take(FULL_NAME_MAX_LENGTH).collect()
}

fn avatar_url_of(google_profile: &GoogleProfile) -> Option<&str> {
   google_profile.avatar_url.as_deref().filter(|avatar_url| avatar_url.starts_with(SECURE_URL_PREFIX))
}

pub async fn login(
   app_state: &AppState,
   google_login_command: GoogleLoginCommand,
   client_info: &ClientInfo,
) -> Result<AuthSession, AppError> {
   let google_profile: GoogleProfile =
      app_state.google_id_token_verifier.verify(&google_login_command.id_token).await?;
   let email: String = google_profile.email.trim().to_lowercase();
   if email.chars().count() > EMAIL_MAX_LENGTH {
      return Err(AppError::bad_request("INVALID_GOOGLE_EMAIL", "Email Google không hợp lệ"));
   }
   let now: DateTime<Utc> = Utc::now();
   let mut transaction = app_state.pg_pool.begin().await?;

   let linked_user_id: Option<i64> =
      user_identity_repository::find_user_id_by_google_subject(&mut *transaction, &google_profile.subject).await?;
   let mut user_record: UserRecord = match linked_user_id {
      Some(user_id) => user_repository::find_user_by_id_for_update(&mut *transaction, user_id)
         .await?
         .ok_or_else(AppError::internal)?,
      None => match user_repository::find_user_by_email_for_update(&mut *transaction, &email).await? {
         Some(existing_user_record) => {
            link_to_existing_user(&mut transaction, &existing_user_record, &google_profile, &email, now).await?;
            existing_user_record
         }
         None => register_new_user(&mut transaction, &google_profile, &email, now).await?,
      },
   };

   if !user_record.is_login_allowed(now) {
      return Err(if user_record.status == UserStatus::Locked {
         account_locked_message()
      } else {
         account_disabled(StatusCode::FORBIDDEN)
      });
   }
   user_identity_repository::touch_google_identity(&mut *transaction, user_record.id, now).await?;
   user_record.record_successful_login(now);
   user_repository::update_login_state(&mut *transaction, &user_record).await?;
   let issued_session =
      issue_session(app_state, &mut transaction, &user_record, Uuid::new_v4(), client_info, now).await?;
   transaction.commit().await?;
   Ok(issued_session.session)
}

async fn link_to_existing_user(
   connection: &mut PgConnection,
   user_record: &UserRecord,
   google_profile: &GoogleProfile,
   email: &str,
   now: DateTime<Utc>,
) -> Result<(), AppError> {
   if user_record.status == UserStatus::Deleted {
      return Err(account_disabled(StatusCode::FORBIDDEN));
   }
   if !user_record.email_verified {
      return Err(AppError::conflict(
         "ACCOUNT_EXISTS_UNVERIFIED",
         "Email này đã đăng ký bằng mật khẩu nhưng chưa được xác thực. Hãy đăng nhập bằng mật khẩu, xác thực email rồi đăng nhập lại bằng Google",
      ));
   }
   if user_identity_repository::exists_google_identity_for_user(&mut *connection, user_record.id).await? {
      return Err(AppError::conflict(
         "GOOGLE_ACCOUNT_MISMATCH",
         "Tài khoản này đã liên kết với một tài khoản Google khác",
      ));
   }
   match user_identity_repository::insert_google_identity(
      &mut *connection,
      user_record.id,
      &google_profile.subject,
      email,
      now,
   )
   .await
   {
      Ok(()) => Ok(()),
      Err(sqlx_error) if repository::is_unique_violation(&sqlx_error) => {
         Err(AppError::conflict("GOOGLE_ACCOUNT_MISMATCH", "Tài khoản này đã liên kết với một tài khoản Google khác"))
      }
      Err(sqlx_error) => Err(sqlx_error.into()),
   }
}

async fn register_new_user(
   connection: &mut PgConnection,
   google_profile: &GoogleProfile,
   email: &str,
   now: DateTime<Utc>,
) -> Result<UserRecord, AppError> {
   let customer_role_id: i64 =
      role_repository::find_role_id_by_code(&mut *connection, CUSTOMER_ROLE_CODE).await?.ok_or_else(|| {
         tracing::error!("Thiếu role {CUSTOMER_ROLE_CODE} trong bảng roles");
         AppError::internal()
      })?;
   let account_already_exists =
      || AppError::conflict("ACCOUNT_ALREADY_EXISTS", "Tài khoản đang được tạo, vui lòng thử lại");
   let user_id: i64 = match user_repository::insert_google_user(
      &mut *connection,
      email,
      &full_name_of(google_profile, email),
      avatar_url_of(google_profile),
   )
   .await
   {
      Ok(user_id) => user_id,
      Err(sqlx_error) if repository::is_unique_violation(&sqlx_error) => return Err(account_already_exists()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   };
   role_repository::insert_user_role(&mut *connection, user_id, customer_role_id).await?;
   match user_identity_repository::insert_google_identity(
      &mut *connection,
      user_id,
      &google_profile.subject,
      email,
      now,
   )
   .await
   {
      Ok(()) => {}
      Err(sqlx_error) if repository::is_unique_violation(&sqlx_error) => return Err(account_already_exists()),
      Err(sqlx_error) => return Err(sqlx_error.into()),
   }
   user_repository::find_user_by_id(&mut *connection, user_id).await?.ok_or_else(AppError::internal)
}
