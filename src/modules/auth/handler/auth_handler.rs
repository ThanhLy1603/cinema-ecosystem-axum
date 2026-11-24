use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum_extra::extract::cookie::CookieJar;

use crate::common::error::AppError;
use crate::common::extract::{ApiJson, ClientInfo};
use crate::common::response::ApiResponse;
use crate::modules::auth::dto::request::ForgotPasswordRequest;
use crate::modules::auth::dto::request::GoogleLoginRequest;
use crate::modules::auth::dto::request::LoginRequest;
use crate::modules::auth::dto::request::RegisterRequest;
use crate::modules::auth::dto::request::ResetPasswordRequest;
use crate::modules::auth::dto::request::VerifyEmailRequest;
use crate::modules::auth::dto::response::LoginResponse;
use crate::modules::auth::dto::response::RefreshTokenResponse;
use crate::modules::auth::dto::response::UserResponse;
use crate::modules::auth::security::session_cookies;
use crate::modules::auth::security::AuthenticatedUser;
use crate::modules::auth::security::SessionCookieSettings;
use crate::modules::auth::security::REFRESH_TOKEN_COOKIE;
use crate::modules::auth::service::auth_service;
use crate::modules::auth::service::auth_service::REFRESH_IN_PROGRESS_ERROR_CODE;
use crate::modules::auth::service::email_verification_service;
use crate::modules::auth::service::google_auth_service;
use crate::modules::auth::service::password_reset_service;
use crate::state::AppState;

const FORGOT_PASSWORD_MESSAGE: &str = "Nếu email tồn tại trong hệ thống, chúng tôi đã gửi hướng dẫn đặt lại mật khẩu";

fn refresh_token_of(cookie_jar: &CookieJar) -> Option<String> {
   cookie_jar.get(REFRESH_TOKEN_COOKIE).map(|cookie| cookie.value().to_string())
}

pub async fn register(
   State(app_state): State<AppState>,
   client_info: ClientInfo,
   ApiJson(register_request): ApiJson<RegisterRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserResponse>>), AppError> {
   let register_command = register_request.validate()?;
   let user_response: UserResponse = auth_service::register(&app_state, register_command).await?;
   if let Err(app_error) =
      email_verification_service::send_verification(&app_state, user_response.id, client_info.ip_address.as_deref())
         .await
   {
      tracing::warn!(user_id = user_response.id, error_code = %app_error.error_code, "Không tạo được email xác thực");
   }
   Ok((StatusCode::CREATED, Json(ApiResponse::success_with_message("Đăng ký thành công", user_response))))
}

pub async fn login(
   State(app_state): State<AppState>,
   cookie_jar: CookieJar,
   client_info: ClientInfo,
   ApiJson(login_request): ApiJson<LoginRequest>,
) -> Result<(CookieJar, Json<ApiResponse<LoginResponse>>), AppError> {
   let login_command = login_request.validate()?;
   let auth_session = auth_service::login(&app_state, login_command, &client_info).await?;
   let response_body: ApiResponse<LoginResponse> =
      ApiResponse::success_with_message("Đăng nhập thành công", auth_session.to_login_response());
   let cookie_jar: CookieJar = session_cookies::add_session_cookies(
      cookie_jar,
      auth_session.access_token,
      auth_session.refresh_token,
      &SessionCookieSettings::from_config(&app_state.app_config),
   );
   Ok((cookie_jar, Json(response_body)))
}

pub async fn login_with_google(
   State(app_state): State<AppState>,
   cookie_jar: CookieJar,
   client_info: ClientInfo,
   ApiJson(google_login_request): ApiJson<GoogleLoginRequest>,
) -> Result<(CookieJar, Json<ApiResponse<LoginResponse>>), AppError> {
   let google_login_command = google_login_request.validate()?;
   let auth_session = google_auth_service::login(&app_state, google_login_command, &client_info).await?;
   let response_body: ApiResponse<LoginResponse> =
      ApiResponse::success_with_message("Đăng nhập thành công", auth_session.to_login_response());
   let cookie_jar: CookieJar = session_cookies::add_session_cookies(
      cookie_jar,
      auth_session.access_token,
      auth_session.refresh_token,
      &SessionCookieSettings::from_config(&app_state.app_config),
   );
   Ok((cookie_jar, Json(response_body)))
}

pub async fn refresh_token(
   State(app_state): State<AppState>,
   cookie_jar: CookieJar,
   client_info: ClientInfo,
) -> Response {
   let settings: SessionCookieSettings = SessionCookieSettings::from_config(&app_state.app_config);
   match auth_service::refresh(&app_state, refresh_token_of(&cookie_jar), &client_info).await {
      Ok(auth_session) => {
         let response_body: ApiResponse<RefreshTokenResponse> =
            ApiResponse::success(auth_session.to_refresh_token_response());
         let cookie_jar: CookieJar = session_cookies::add_session_cookies(
            cookie_jar,
            auth_session.access_token,
            auth_session.refresh_token,
            &settings,
         );
         (cookie_jar, Json(response_body)).into_response()
      }
      Err(app_error) if app_error.error_code == REFRESH_IN_PROGRESS_ERROR_CODE => app_error.into_response(),
      Err(app_error) => (session_cookies::add_cleared_cookies(cookie_jar, settings.secure), app_error).into_response(),
   }
}

pub async fn forgot_password(
   State(app_state): State<AppState>,
   client_info: ClientInfo,
   ApiJson(forgot_password_request): ApiJson<ForgotPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
   let forgot_password_command = forgot_password_request.validate()?;
   password_reset_service::request_reset(&app_state, forgot_password_command, client_info.ip_address.as_deref())
      .await?;
   Ok(Json(ApiResponse::message_only(FORGOT_PASSWORD_MESSAGE)))
}

pub async fn reset_password(
   State(app_state): State<AppState>,
   ApiJson(reset_password_request): ApiJson<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
   let reset_password_command = reset_password_request.validate()?;
   password_reset_service::reset_password(&app_state, reset_password_command).await?;
   Ok(Json(ApiResponse::message_only("Đặt lại mật khẩu thành công, vui lòng đăng nhập lại")))
}

pub async fn verify_email(
   State(app_state): State<AppState>,
   ApiJson(verify_email_request): ApiJson<VerifyEmailRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
   let verify_email_command = verify_email_request.validate()?;
   email_verification_service::verify_email(&app_state, verify_email_command).await?;
   Ok(Json(ApiResponse::message_only("Xác thực email thành công")))
}

pub async fn resend_verification(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   client_info: ClientInfo,
) -> Result<Json<ApiResponse<()>>, AppError> {
   email_verification_service::resend_verification(
      &app_state,
      authenticated_user.user_id,
      client_info.ip_address.as_deref(),
   )
   .await?;
   Ok(Json(ApiResponse::message_only("Đã gửi lại email xác thực")))
}

pub async fn logout(
   State(app_state): State<AppState>,
   _authenticated_user: AuthenticatedUser,
   cookie_jar: CookieJar,
) -> Result<(CookieJar, Json<ApiResponse<()>>), AppError> {
   auth_service::logout(&app_state, refresh_token_of(&cookie_jar)).await?;
   Ok((
      session_cookies::add_cleared_cookies(cookie_jar, app_state.app_config.cookie_secure),
      Json(ApiResponse::message_only("Đăng xuất thành công")),
   ))
}

pub async fn logout_all(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   cookie_jar: CookieJar,
) -> Result<(CookieJar, Json<ApiResponse<()>>), AppError> {
   auth_service::logout_all(&app_state, authenticated_user.user_id).await?;
   Ok((
      session_cookies::add_cleared_cookies(cookie_jar, app_state.app_config.cookie_secure),
      Json(ApiResponse::message_only("Đã đăng xuất khỏi tất cả thiết bị")),
   ))
}
