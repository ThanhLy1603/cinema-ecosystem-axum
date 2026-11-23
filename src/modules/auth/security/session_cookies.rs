use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration;

use crate::config::AppConfig;

pub const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";
const ACCESS_TOKEN_COOKIE_PATH: &str = "/";
const REFRESH_TOKEN_COOKIE_PATH: &str = "/api/v1/auth";

fn build_cookie(
   name: &'static str,
   value: String,
   path: &'static str,
   max_age_seconds: i64,
   secure: bool,
) -> Cookie<'static> {
   Cookie::build((name, value))
      .http_only(true)
      .secure(secure)
      .same_site(SameSite::Strict)
      .path(path)
      .max_age(Duration::seconds(max_age_seconds))
      .build()
}

pub struct SessionCookieSettings {
   pub secure: bool,
   pub access_token_ttl_seconds: i64,
   pub refresh_token_ttl_seconds: i64,
}

impl SessionCookieSettings {
   pub fn from_config(app_config: &AppConfig) -> SessionCookieSettings {
      SessionCookieSettings {
         secure: app_config.cookie_secure,
         access_token_ttl_seconds: app_config.jwt.access_token_ttl_seconds,
         refresh_token_ttl_seconds: app_config.jwt.refresh_token_ttl_seconds,
      }
   }
}

pub fn add_session_cookies(
   cookie_jar: CookieJar,
   access_token: String,
   refresh_token: String,
   settings: &SessionCookieSettings,
) -> CookieJar {
   cookie_jar
      .add(build_cookie(
         REFRESH_TOKEN_COOKIE,
         refresh_token,
         REFRESH_TOKEN_COOKIE_PATH,
         settings.refresh_token_ttl_seconds,
         settings.secure,
      ))
      .add(build_cookie(
         ACCESS_TOKEN_COOKIE,
         access_token,
         ACCESS_TOKEN_COOKIE_PATH,
         settings.access_token_ttl_seconds,
         settings.secure,
      ))
}

pub fn add_cleared_cookies(cookie_jar: CookieJar, secure: bool) -> CookieJar {
   cookie_jar
      .add(build_cookie(REFRESH_TOKEN_COOKIE, String::new(), REFRESH_TOKEN_COOKIE_PATH, 0, secure))
      .add(build_cookie(ACCESS_TOKEN_COOKIE, String::new(), ACCESS_TOKEN_COOKIE_PATH, 0, secure))
}

#[cfg(test)]
mod tests {
   use axum::http::header::SET_COOKIE;
   use axum::response::IntoResponse;
   use axum_extra::extract::cookie::CookieJar;

   use super::{add_cleared_cookies, add_session_cookies, SessionCookieSettings};

   fn set_cookie_headers(cookie_jar: CookieJar) -> Vec<String> {
      cookie_jar
         .into_response()
         .headers()
         .get_all(SET_COOKIE)
         .iter()
         .map(|header_value| header_value.to_str().unwrap().to_string())
         .collect()
   }

   fn settings(secure: bool) -> SessionCookieSettings {
      SessionCookieSettings { secure, access_token_ttl_seconds: 900, refresh_token_ttl_seconds: 604800 }
   }

   #[test]
   fn session_cookies_are_http_only_strict_with_expected_paths_and_ages() {
      let headers: Vec<String> = set_cookie_headers(add_session_cookies(
         CookieJar::new(),
         "jwt".to_string(),
         "refresh".to_string(),
         &settings(false),
      ));

      let refresh_cookie: &String = headers.iter().find(|header| header.starts_with("refresh_token=refresh")).unwrap();
      assert!(refresh_cookie.contains("HttpOnly"));
      assert!(refresh_cookie.contains("SameSite=Strict"));
      assert!(refresh_cookie.contains("Path=/api/v1/auth"));
      assert!(refresh_cookie.contains("Max-Age=604800"));
      assert!(!refresh_cookie.contains("Secure"));

      let access_cookie: &String = headers.iter().find(|header| header.starts_with("access_token=jwt")).unwrap();
      assert!(access_cookie.contains("HttpOnly"));
      assert!(access_cookie.contains("SameSite=Strict"));
      assert!(access_cookie.contains("Path=/;") || access_cookie.ends_with("Path=/"));
      assert!(access_cookie.contains("Max-Age=900"));
   }

   #[test]
   fn secure_flag_follows_configuration() {
      let headers: Vec<String> = set_cookie_headers(add_session_cookies(
         CookieJar::new(),
         "jwt".to_string(),
         "refresh".to_string(),
         &settings(true),
      ));
      assert!(headers.iter().all(|header| header.contains("Secure")));
   }

   #[test]
   fn cleared_cookies_are_empty_with_zero_max_age() {
      let headers: Vec<String> = set_cookie_headers(add_cleared_cookies(CookieJar::new(), false));
      assert_eq!(headers.len(), 2);
      assert!(headers.iter().all(|header| header.contains("Max-Age=0")));
      assert!(headers
         .iter()
         .any(|header| header.starts_with("refresh_token=;") && header.contains("Path=/api/v1/auth")));
      assert!(headers.iter().any(|header| header.starts_with("access_token=;")));
   }
}
