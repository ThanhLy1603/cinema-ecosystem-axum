use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;

use axum::extract::path::ErrorKind;
use axum::extract::rejection::PathRejection;
use axum::extract::{ConnectInfo, FromRequest, FromRequestParts, Path, Query, RawPathParams, Request};
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

pub struct ApiPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
   S: Send + Sync,
   T: DeserializeOwned + Send,
{
   type Rejection = AppError;

   async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<ApiPath<T>, AppError> {
      match Path::<T>::from_request_parts(parts, state).await {
         Ok(Path(value)) => Ok(ApiPath(value)),
         Err(PathRejection::FailedToDeserializePathParams(path_error)) => {
            let parameter_name: String = match path_error.kind() {
               ErrorKind::ParseErrorAtKey { key, .. } | ErrorKind::DeserializeError { key, .. } => key.clone(),
               _ => first_path_parameter_name(parts, state).await,
            };
            Err(invalid_parameter(&parameter_name))
         }
         Err(_) => Err(AppError::internal()),
      }
   }
}

async fn first_path_parameter_name<S: Send + Sync>(parts: &mut Parts, state: &S) -> String {
   match RawPathParams::from_request_parts(parts, state).await {
      Ok(raw_path_params) => {
         raw_path_params.iter().next().map(|(key, _)| key.to_string()).unwrap_or_else(|| "id".to_string())
      }
      Err(_) => "id".to_string(),
   }
}

pub fn invalid_parameter(parameter_name: &str) -> AppError {
   AppError::bad_request("INVALID_PARAMETER", &format!("Tham số '{parameter_name}' không hợp lệ"))
}

pub struct QueryParams(HashMap<String, String>);

impl<S> FromRequestParts<S> for QueryParams
where
   S: Send + Sync,
{
   type Rejection = AppError;

   async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<QueryParams, AppError> {
      Query::<HashMap<String, String>>::from_request_parts(parts, state)
         .await
         .map(|Query(parameters)| QueryParams(parameters))
         .map_err(|_| AppError::bad_request("INVALID_PARAMETER", "Tham số truy vấn không hợp lệ"))
   }
}

impl QueryParams {
   pub fn text(&self, name: &str) -> Option<String> {
      self.0.get(name).cloned()
   }

   pub fn boolean(&self, name: &str, default_value: bool) -> Result<bool, AppError> {
      match self.0.get(name).map(|value| value.trim().to_ascii_lowercase()) {
         None => Ok(default_value),
         Some(value) if value.is_empty() => Ok(default_value),
         Some(value) => match value.as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(invalid_parameter(name)),
         },
      }
   }

   pub fn integer(&self, name: &str, default_value: i64) -> Result<i64, AppError> {
      match self.0.get(name).map(|value| value.trim()) {
         None | Some("") => Ok(default_value),
         Some(value) => value.parse::<i64>().map_err(|_| invalid_parameter(name)),
      }
   }

   pub fn enumeration<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, AppError> {
      match self.0.get(name).map(|value| value.trim()) {
         None | Some("") => Ok(None),
         Some(value) => serde_json::from_value::<T>(serde_json::Value::String(value.to_string()))
            .map(Some)
            .map_err(|_| invalid_parameter(name)),
      }
   }
}
