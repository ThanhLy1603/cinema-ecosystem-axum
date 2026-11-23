use serde::Serialize;

use super::UserResponse;

#[derive(Serialize)]
pub struct LoginResponse {
   pub user: UserResponse,
}

#[derive(Serialize)]
pub struct RefreshTokenResponse {
   pub user: UserResponse,
}
