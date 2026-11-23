use tokio::sync::OnceCell;

use crate::common::error::AppError;

const MAX_BCRYPT_PASSWORD_BYTES: usize = 72;

pub struct PasswordService {
   cost: u32,
   dummy_hash: OnceCell<String>,
}

impl PasswordService {
   pub fn new(cost: u32) -> PasswordService {
      PasswordService { cost, dummy_hash: OnceCell::new() }
   }

   pub async fn hash(&self, password: String) -> Result<String, AppError> {
      let cost: u32 = self.cost;
      match tokio::task::spawn_blocking(move || bcrypt::hash(password, cost)).await {
         Ok(Ok(password_hash)) => Ok(password_hash),
         _ => {
            tracing::error!("không băm được mật khẩu");
            Err(AppError::internal())
         }
      }
   }

   pub async fn verify(&self, password: String, password_hash: String) -> bool {
      tokio::task::spawn_blocking(move || bcrypt::verify(password, &password_hash).unwrap_or(false))
         .await
         .unwrap_or(false)
   }

   pub async fn verify_against_dummy(&self, password: String) {
      let dummy_hash: String = self
         .dummy_hash
         .get_or_init(|| async { self.hash(uuid::Uuid::new_v4().to_string()).await.unwrap_or_default() })
         .await
         .clone();
      self.verify(password, dummy_hash).await;
   }
}

pub fn ensure_password_fits_bcrypt(password: &str) -> Result<(), AppError> {
   if password.len() > MAX_BCRYPT_PASSWORD_BYTES {
      return Err(AppError::bad_request("PASSWORD_TOO_LONG", "Mật khẩu quá dài (tối đa 72 byte)"));
   }
   Ok(())
}

#[cfg(test)]
mod tests {
   use super::{ensure_password_fits_bcrypt, PasswordService};

   #[tokio::test]
   async fn hash_verifies_only_the_original_password() {
      let password_service: PasswordService = PasswordService::new(4);
      let password_hash: String = password_service.hash("matkhau123".to_string()).await.unwrap();
      assert!(password_hash.starts_with("$2"));
      assert!(password_service.verify("matkhau123".to_string(), password_hash.clone()).await);
      assert!(!password_service.verify("matkhau124".to_string(), password_hash).await);
   }

   #[tokio::test]
   async fn verifies_hash_produced_by_spring_bcrypt() {
      let password_service: PasswordService = PasswordService::new(4);
      let spring_hash: String = "$2a$12$AcTR8z2DSsnyE8kKkOXc7.YB.jmMTcXeU5/QGnIHzuu9BywvvmupS".to_string();
      assert!(password_service.verify("matkhau123".to_string(), spring_hash.clone()).await);
      assert!(!password_service.verify("matkhau124".to_string(), spring_hash).await);
   }

   #[tokio::test]
   async fn malformed_hash_never_verifies() {
      let password_service: PasswordService = PasswordService::new(4);
      assert!(!password_service.verify("x".to_string(), "khong-phai-bcrypt".to_string()).await);
   }

   #[tokio::test]
   async fn dummy_verification_completes_without_panicking() {
      let password_service: PasswordService = PasswordService::new(4);
      password_service.verify_against_dummy("bat-ky".to_string()).await;
      password_service.verify_against_dummy("bat-ky".to_string()).await;
   }

   #[test]
   fn rejects_passwords_longer_than_72_bytes() {
      assert!(ensure_password_fits_bcrypt(&"a".repeat(72)).is_ok());
      assert_eq!(ensure_password_fits_bcrypt(&"a".repeat(73)).unwrap_err().error_code, "PASSWORD_TOO_LONG");
      assert!(ensure_password_fits_bcrypt(&"ế".repeat(25)).is_err());
   }
}
