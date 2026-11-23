use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method};
use axum_extra::extract::cookie::CookieJar;

use crate::common::error::AppError;
use crate::modules::auth::security::ACCESS_TOKEN_COOKIE;
use crate::state::AppState;

pub const CSRF_GUARD_HEADER: &str = "x-requested-with";
const BEARER_PREFIX: &str = "Bearer ";

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
   pub user_id: i64,
   pub email: String,
   pub roles: Vec<String>,
}

impl AuthenticatedUser {
   pub fn has_any_role(&self, role_codes: &[&str]) -> bool {
      self.roles.iter().any(|role| role_codes.contains(&role.as_str()))
   }

   pub fn require_any_role(&self, role_codes: &[&str]) -> Result<(), AppError> {
      if self.has_any_role(role_codes) {
         Ok(())
      } else {
         Err(AppError::forbidden("ACCESS_DENIED", "Bạn không có quyền thực hiện thao tác này"))
      }
   }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
   type Rejection = AppError;

   async fn from_request_parts(parts: &mut Parts, app_state: &AppState) -> Result<AuthenticatedUser, AppError> {
      resolve_access_token(&parts.method, &parts.headers)
         .and_then(|access_token| app_state.jwt_service.parse_access_token(&access_token))
         .ok_or_else(|| AppError::unauthorized("UNAUTHORIZED", "Phiên đăng nhập không hợp lệ hoặc đã hết hạn"))
   }
}

pub fn resolve_access_token(method: &Method, headers: &HeaderMap) -> Option<String> {
   if let Some(bearer_token) = extract_bearer_token(headers) {
      return Some(bearer_token);
   }
   if !is_csrf_guard_satisfied(method, headers) {
      return None;
   }
   CookieJar::from_headers(headers)
      .get(ACCESS_TOKEN_COOKIE)
      .map(|cookie| cookie.value().to_string())
      .filter(|access_token| !access_token.trim().is_empty())
}

fn is_csrf_guard_satisfied(method: &Method, headers: &HeaderMap) -> bool {
   if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE) {
      return true;
   }
   headers
      .get(CSRF_GUARD_HEADER)
      .and_then(|header_value| header_value.to_str().ok())
      .is_some_and(|header_value| !header_value.trim().is_empty())
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
   let authorization: &str = headers.get(AUTHORIZATION)?.to_str().ok()?;
   let bearer_token: &str = authorization.strip_prefix(BEARER_PREFIX)?.trim();
   if bearer_token.is_empty() {
      None
   } else {
      Some(bearer_token.to_string())
   }
}

#[cfg(test)]
mod tests {
   use axum::http::{HeaderMap, HeaderValue, Method};

   use super::{resolve_access_token, AuthenticatedUser};

   fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
      let mut header_map: HeaderMap = HeaderMap::new();
      for (name, value) in pairs {
         header_map.insert(*name, HeaderValue::from_str(value).unwrap());
      }
      header_map
   }

   #[test]
   fn bearer_token_wins_over_cookie() {
      let header_map = headers(&[("authorization", "Bearer from-header"), ("cookie", "access_token=from-cookie")]);
      assert_eq!(resolve_access_token(&Method::POST, &header_map).as_deref(), Some("from-header"));
   }

   #[test]
   fn cookie_on_post_requires_csrf_guard_header() {
      let without_guard = headers(&[("cookie", "access_token=from-cookie")]);
      assert_eq!(resolve_access_token(&Method::POST, &without_guard), None);

      let with_guard = headers(&[("cookie", "access_token=from-cookie"), ("x-requested-with", "XMLHttpRequest")]);
      assert_eq!(resolve_access_token(&Method::POST, &with_guard).as_deref(), Some("from-cookie"));
   }

   #[test]
   fn blank_csrf_guard_header_does_not_count() {
      let header_map = headers(&[("cookie", "access_token=from-cookie"), ("x-requested-with", "  ")]);
      assert_eq!(resolve_access_token(&Method::PUT, &header_map), None);
   }

   #[test]
   fn cookie_on_safe_methods_needs_no_guard_header() {
      let header_map = headers(&[("cookie", "access_token=from-cookie")]);
      for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
         assert_eq!(resolve_access_token(&method, &header_map).as_deref(), Some("from-cookie"));
      }
   }

   #[test]
   fn empty_bearer_and_empty_cookie_are_ignored() {
      let empty_bearer = headers(&[("authorization", "Bearer   ")]);
      assert_eq!(resolve_access_token(&Method::GET, &empty_bearer), None);
      let empty_cookie = headers(&[("cookie", "access_token=")]);
      assert_eq!(resolve_access_token(&Method::GET, &empty_cookie), None);
      let other_scheme = headers(&[("authorization", "Basic abc")]);
      assert_eq!(resolve_access_token(&Method::GET, &other_scheme), None);
   }

   #[test]
   fn role_checks_use_exact_role_codes() {
      let authenticated_user: AuthenticatedUser =
         AuthenticatedUser { user_id: 1, email: "a@example.com".to_string(), roles: vec!["ROLE_STAFF".to_string()] };
      assert!(authenticated_user.has_any_role(&["ROLE_STAFF", "ROLE_ADMIN"]));
      assert!(!authenticated_user.has_any_role(&["ROLE_ADMIN"]));
      assert_eq!(authenticated_user.require_any_role(&["ROLE_ADMIN"]).unwrap_err().error_code, "ACCESS_DENIED");
   }
}
