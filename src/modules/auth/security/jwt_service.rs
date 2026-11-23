use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::common::error::AppError;
use crate::modules::auth::security::AuthenticatedUser;

const ISSUER: &str = "cinema-ecosystem";
const MIN_SECRET_BYTES: usize = 32;

#[derive(Serialize, Deserialize)]
struct AccessTokenClaims {
   iss: String,
   sub: String,
   email: String,
   roles: Vec<String>,
   iat: i64,
   exp: i64,
}

pub struct JwtService {
   encoding_key: EncodingKey,
   decoding_key: DecodingKey,
   validation: Validation,
}

impl JwtService {
   pub fn new(secret: &str) -> Result<JwtService, String> {
      if secret.len() < MIN_SECRET_BYTES {
         return Err(format!("JWT_SECRET phải dài tối thiểu {MIN_SECRET_BYTES} byte (256 bit) cho HS256"));
      }
      let mut validation: Validation = Validation::new(Algorithm::HS256);
      validation.set_issuer(&[ISSUER]);
      validation.set_required_spec_claims(&["exp", "iss", "sub"]);
      validation.leeway = 0;
      Ok(JwtService {
         encoding_key: EncodingKey::from_secret(secret.as_bytes()),
         decoding_key: DecodingKey::from_secret(secret.as_bytes()),
         validation,
      })
   }

   pub fn generate_access_token(
      &self,
      user_id: i64,
      email: &str,
      role_codes: &[String],
      issued_at: DateTime<Utc>,
      ttl_seconds: i64,
   ) -> Result<String, AppError> {
      let mut sorted_role_codes: Vec<String> = role_codes.to_vec();
      sorted_role_codes.sort();
      let access_token_claims: AccessTokenClaims = AccessTokenClaims {
         iss: ISSUER.to_string(),
         sub: user_id.to_string(),
         email: email.to_string(),
         roles: sorted_role_codes,
         iat: issued_at.timestamp(),
         exp: issued_at.timestamp() + ttl_seconds,
      };
      encode(&Header::new(Algorithm::HS256), &access_token_claims, &self.encoding_key).map_err(|jwt_error| {
         tracing::error!(error = %jwt_error, "không ký được access token");
         AppError::internal()
      })
   }

   pub fn parse_access_token(&self, access_token: &str) -> Option<AuthenticatedUser> {
      let token_data = decode::<AccessTokenClaims>(access_token, &self.decoding_key, &self.validation).ok()?;
      let user_id: i64 = token_data.claims.sub.parse().ok()?;
      Some(AuthenticatedUser { user_id, email: token_data.claims.email, roles: token_data.claims.roles })
   }
}

#[cfg(test)]
mod tests {
   use chrono::{Duration, Utc};

   use super::JwtService;

   const SECRET: &str = "0123456789abcdef0123456789abcdef";

   fn roles() -> Vec<String> {
      vec!["ROLE_STAFF".to_string(), "ROLE_CUSTOMER".to_string()]
   }

   #[test]
   fn rejects_secret_shorter_than_32_bytes() {
      assert!(JwtService::new("ngan").is_err());
      assert!(JwtService::new(SECRET).is_ok());
   }

   #[test]
   fn token_round_trips_with_sorted_roles() {
      let jwt_service: JwtService = JwtService::new(SECRET).unwrap();
      let access_token: String =
         jwt_service.generate_access_token(7, "a@example.com", &roles(), Utc::now(), 900).unwrap();

      let authenticated_user = jwt_service.parse_access_token(&access_token).unwrap();

      assert_eq!(authenticated_user.user_id, 7);
      assert_eq!(authenticated_user.email, "a@example.com");
      assert_eq!(authenticated_user.roles, vec!["ROLE_CUSTOMER".to_string(), "ROLE_STAFF".to_string()]);
   }

   #[test]
   fn expired_token_is_rejected_without_leeway() {
      let jwt_service: JwtService = JwtService::new(SECRET).unwrap();
      let issued_at = Utc::now() - Duration::seconds(901);
      let access_token: String =
         jwt_service.generate_access_token(7, "a@example.com", &roles(), issued_at, 900).unwrap();
      assert!(jwt_service.parse_access_token(&access_token).is_none());
   }

   #[test]
   fn token_signed_with_another_secret_is_rejected() {
      let signer: JwtService = JwtService::new("ffffffffffffffffffffffffffffffff").unwrap();
      let verifier: JwtService = JwtService::new(SECRET).unwrap();
      let access_token: String = signer.generate_access_token(7, "a@example.com", &roles(), Utc::now(), 900).unwrap();
      assert!(verifier.parse_access_token(&access_token).is_none());
   }

   #[test]
   fn garbage_token_is_rejected() {
      let jwt_service: JwtService = JwtService::new(SECRET).unwrap();
      assert!(jwt_service.parse_access_token("khong.phai.jwt").is_none());
      assert!(jwt_service.parse_access_token("").is_none());
   }
}
