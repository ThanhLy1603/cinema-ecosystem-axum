use crate::modules::auth::dto::response::{LoginResponse, RefreshTokenResponse, UserResponse};

pub struct AuthSession {
   pub user: UserResponse,
   pub access_token: String,
   pub refresh_token: String,
}

impl AuthSession {
   pub fn to_login_response(&self) -> LoginResponse {
      LoginResponse { user: self.user.clone() }
   }

   pub fn to_refresh_token_response(&self) -> RefreshTokenResponse {
      RefreshTokenResponse { user: self.user.clone() }
   }
}
