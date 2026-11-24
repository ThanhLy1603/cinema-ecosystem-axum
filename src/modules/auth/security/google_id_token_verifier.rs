use std::time::{Duration, Instant};

use axum::http::StatusCode;
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::common::error::AppError;
use crate::modules::auth::model::GoogleProfile;

const GOOGLE_JWK_SET_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const TRUSTED_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];
const CLOCK_SKEW_SECONDS: u64 = 60;
const KEY_CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

struct CachedKeys {
   jwk_set: JwkSet,
   fetched_at: Instant,
}

enum KeySource {
   Remote { http_client: reqwest::Client, url: String, cache: RwLock<Option<CachedKeys>> },
   Static(JwkSet),
}

pub struct GoogleIdTokenVerifier {
   client_id: String,
   key_source: KeySource,
}

#[derive(Deserialize)]
struct GoogleClaims {
   sub: Option<String>,
   email: Option<String>,
   email_verified: Option<serde_json::Value>,
   name: Option<String>,
   picture: Option<String>,
}

fn invalid_google_token() -> AppError {
   AppError::unauthorized("INVALID_GOOGLE_TOKEN", "Google ID Token không hợp lệ hoặc đã hết hạn")
}

fn google_unavailable() -> AppError {
   AppError::new(
      StatusCode::SERVICE_UNAVAILABLE,
      "GOOGLE_UNAVAILABLE",
      "Không thể xác thực với Google lúc này, vui lòng thử lại sau",
   )
}

fn is_email_verified(email_verified: Option<&serde_json::Value>) -> bool {
   match email_verified {
      Some(serde_json::Value::Bool(value)) => *value,
      Some(serde_json::Value::String(text)) => text.eq_ignore_ascii_case("true"),
      _ => false,
   }
}

impl GoogleIdTokenVerifier {
   pub fn remote(client_id: &str) -> GoogleIdTokenVerifier {
      let http_client: reqwest::Client =
         reqwest::Client::builder().timeout(FETCH_TIMEOUT).build().unwrap_or_else(|_| reqwest::Client::new());
      GoogleIdTokenVerifier {
         client_id: client_id.trim().to_string(),
         key_source: KeySource::Remote { http_client, url: GOOGLE_JWK_SET_URL.to_string(), cache: RwLock::new(None) },
      }
   }

   pub fn with_static_keys(client_id: &str, jwk_set: JwkSet) -> GoogleIdTokenVerifier {
      GoogleIdTokenVerifier { client_id: client_id.trim().to_string(), key_source: KeySource::Static(jwk_set) }
   }

   pub fn is_configured(&self) -> bool {
      !self.client_id.is_empty()
   }

   pub async fn verify(&self, id_token: &str) -> Result<GoogleProfile, AppError> {
      if !self.is_configured() {
         return Err(AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "GOOGLE_LOGIN_DISABLED",
            "Đăng nhập bằng Google chưa được cấu hình",
         ));
      }
      let header = decode_header(id_token).map_err(|_| invalid_google_token())?;
      if header.alg != Algorithm::RS256 {
         return Err(invalid_google_token());
      }
      let key_id: String = header.kid.ok_or_else(invalid_google_token)?;
      let jwk: Jwk = self.find_key(&key_id).await?.ok_or_else(invalid_google_token)?;
      let decoding_key: DecodingKey = DecodingKey::from_jwk(&jwk).map_err(|_| invalid_google_token())?;
      let mut validation: Validation = Validation::new(Algorithm::RS256);
      validation.set_audience(&[self.client_id.as_str()]);
      validation.set_issuer(&TRUSTED_ISSUERS);
      validation.leeway = CLOCK_SKEW_SECONDS;
      validation.validate_exp = true;
      validation.set_required_spec_claims(&["exp", "iss", "aud"]);
      let google_claims: GoogleClaims =
         decode::<GoogleClaims>(id_token, &decoding_key, &validation).map_err(|_| invalid_google_token())?.claims;
      if !is_email_verified(google_claims.email_verified.as_ref()) {
         return Err(AppError::forbidden("GOOGLE_EMAIL_NOT_VERIFIED", "Email Google chưa được xác thực"));
      }
      match (google_claims.sub, google_claims.email) {
         (Some(subject), Some(email)) if !subject.trim().is_empty() && !email.trim().is_empty() => {
            Ok(GoogleProfile { subject, email, full_name: google_claims.name, avatar_url: google_claims.picture })
         }
         _ => Err(invalid_google_token()),
      }
   }

   async fn find_key(&self, key_id: &str) -> Result<Option<Jwk>, AppError> {
      match &self.key_source {
         KeySource::Static(jwk_set) => Ok(jwk_set.find(key_id).cloned()),
         KeySource::Remote { http_client, url, cache } => {
            if let Some(cached_keys) = cache.read().await.as_ref() {
               if cached_keys.fetched_at.elapsed() < KEY_CACHE_TTL {
                  if let Some(jwk) = cached_keys.jwk_set.find(key_id) {
                     return Ok(Some(jwk.clone()));
                  }
                  if cached_keys.fetched_at.elapsed() < MIN_REFRESH_INTERVAL {
                     return Ok(None);
                  }
               }
            }
            let mut cache_guard = cache.write().await;
            if let Some(cached_keys) = cache_guard.as_ref() {
               if cached_keys.fetched_at.elapsed() < MIN_REFRESH_INTERVAL {
                  return Ok(cached_keys.jwk_set.find(key_id).cloned());
               }
            }
            let jwk_set: JwkSet = fetch_jwk_set(http_client, url).await?;
            let found_key: Option<Jwk> = jwk_set.find(key_id).cloned();
            *cache_guard = Some(CachedKeys { jwk_set, fetched_at: Instant::now() });
            Ok(found_key)
         }
      }
   }
}

async fn fetch_jwk_set(http_client: &reqwest::Client, url: &str) -> Result<JwkSet, AppError> {
   let response = http_client.get(url).send().await.map_err(|fetch_error| {
      tracing::warn!(error = %fetch_error, "Không tải được khóa công khai của Google");
      google_unavailable()
   })?;
   if !response.status().is_success() {
      tracing::warn!(status = %response.status(), "Google trả về lỗi khi tải khóa công khai");
      return Err(google_unavailable());
   }
   response.json::<JwkSet>().await.map_err(|parse_error| {
      tracing::warn!(error = %parse_error, "Khóa công khai của Google không đọc được");
      google_unavailable()
   })
}
