use axum::extract::State;
use axum::response::Json;
use axum_extra::extract::cookie::CookieJar;

use crate::common::error::AppError;
use crate::common::extract::{ApiJson, ClientInfo};
use crate::common::response::ApiResponse;
use crate::modules::auth::dto::request::ChangePasswordRequest;
use crate::modules::auth::dto::request::UpdateProfileRequest;
use crate::modules::auth::dto::response::LoginResponse;
use crate::modules::auth::dto::response::UserProfileResponse;
use crate::modules::auth::security::session_cookies;
use crate::modules::auth::security::AuthenticatedUser;
use crate::modules::auth::security::SessionCookieSettings;
use crate::modules::auth::service::user_profile_service;
use crate::state::AppState;

pub async fn get_profile(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
   let user_profile_response: UserProfileResponse =
      user_profile_service::get_profile(&app_state, authenticated_user.user_id).await?;
   Ok(Json(ApiResponse::success(user_profile_response)))
}

pub async fn update_profile(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   ApiJson(update_profile_request): ApiJson<UpdateProfileRequest>,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
   let update_profile_command = update_profile_request.validate()?;
   let user_profile_response: UserProfileResponse =
      user_profile_service::update_profile(&app_state, authenticated_user.user_id, update_profile_command).await?;
   Ok(Json(ApiResponse::success_with_message("Cập nhật hồ sơ thành công", user_profile_response)))
}

pub async fn change_password(
   State(app_state): State<AppState>,
   authenticated_user: AuthenticatedUser,
   cookie_jar: CookieJar,
   client_info: ClientInfo,
   ApiJson(change_password_request): ApiJson<ChangePasswordRequest>,
) -> Result<(CookieJar, Json<ApiResponse<LoginResponse>>), AppError> {
   let change_password_command = change_password_request.validate()?;
   let auth_session = user_profile_service::change_password(
      &app_state,
      authenticated_user.user_id,
      change_password_command,
      &client_info,
   )
   .await?;
   let response_body: ApiResponse<LoginResponse> =
      ApiResponse::success_with_message("Đổi mật khẩu thành công", auth_session.to_login_response());
   let cookie_jar: CookieJar = session_cookies::add_session_cookies(
      cookie_jar,
      auth_session.access_token,
      auth_session.refresh_token,
      &SessionCookieSettings::from_config(&app_state.app_config),
   );
   Ok((cookie_jar, Json(response_body)))
}
