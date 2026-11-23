use std::convert::Infallible;
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, FromRequest, FromRequestParts, Request};
use axum::http::header::USER_AGENT;
use axum::http::request::Parts;
use axum::response::Json;
use serde::de::DeserializeOwned;

use crate::common::error::AppError;

pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
   S: Send + Sync,
   T: DeserializeOwned,
{
   type Rejection = AppError;

   async fn from_request(request: Request, state: &S) -> Result<ApiJson<T>, AppError> {
      match Json::<T>::from_request(request, state).await {
         Ok(Json(value)) => Ok(ApiJson(value)),
         Err(_) => Err(AppError::bad_request("INVALID_REQUEST_BODY", "Nội dung yêu cầu không hợp lệ")),
      }
   }
}

#[derive(Clone, Debug, Default)]
pub struct ClientInfo {
   pub device_info: Option<String>,
   pub ip_address: Option<String>,
}

impl<S> FromRequestParts<S> for ClientInfo
where
   S: Send + Sync,
{
   type Rejection = Infallible;

   async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<ClientInfo, Infallible> {
      let device_info: Option<String> =
         parts.headers.get(USER_AGENT).and_then(|header_value| header_value.to_str().ok()).map(str::to_string);
      let ip_address: Option<String> =
         parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|connect_info| connect_info.0.ip().to_string());
      Ok(ClientInfo { device_info, ip_address })
   }
}
