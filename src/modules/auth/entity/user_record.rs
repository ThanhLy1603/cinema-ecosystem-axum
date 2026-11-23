use chrono::{DateTime, Duration, NaiveDate, Utc};

use crate::modules::auth::enums::UserStatus;

#[derive(Clone, sqlx::FromRow)]
pub struct UserRecord {
   pub id: i64,
   pub email: String,
   pub password_hash: Option<String>,
   pub email_verified: bool,
   pub full_name: String,
   pub phone: Option<String>,
   pub avatar_url: Option<String>,
   pub date_of_birth: Option<NaiveDate>,
   pub gender: Option<String>,
   pub tier: String,
   pub loyalty_points: i32,
   #[sqlx(try_from = "String")]
   pub status: UserStatus,
   pub failed_login_attempts: i32,
   pub locked_until: Option<DateTime<Utc>>,
   pub last_login_at: Option<DateTime<Utc>>,
}

impl UserRecord {
   pub fn has_password(&self) -> bool {
      self.password_hash.as_deref().is_some_and(|password_hash| !password_hash.trim().is_empty())
   }

   pub fn is_login_allowed(&self, now: DateTime<Utc>) -> bool {
      match self.status {
         UserStatus::Active => true,
         UserStatus::Locked => self.is_lock_expired(now),
         UserStatus::Suspended | UserStatus::Deleted => false,
      }
   }

   pub fn register_failed_login(&mut self, max_attempts: i32, lock_duration: Duration, now: DateTime<Utc>) {
      self.release_expired_lock(now);
      self.failed_login_attempts += 1;
      if self.status == UserStatus::Active && self.failed_login_attempts >= max_attempts {
         self.status = UserStatus::Locked;
         self.locked_until = Some(now + lock_duration);
      }
   }

   pub fn record_successful_login(&mut self, now: DateTime<Utc>) {
      self.failed_login_attempts = 0;
      self.locked_until = None;
      if self.status == UserStatus::Locked {
         self.status = UserStatus::Active;
      }
      self.last_login_at = Some(now);
   }

   fn is_lock_expired(&self, now: DateTime<Utc>) -> bool {
      self.locked_until.is_none_or(|locked_until| now >= locked_until)
   }

   fn release_expired_lock(&mut self, now: DateTime<Utc>) {
      if self.status == UserStatus::Locked && self.is_lock_expired(now) {
         self.status = UserStatus::Active;
         self.locked_until = None;
         self.failed_login_attempts = 0;
      }
   }
}
#[cfg(test)]
mod tests {
   use chrono::{DateTime, Duration, Utc};

   use super::UserRecord;
   use crate::modules::auth::enums::UserStatus;

   fn active_user() -> UserRecord {
      UserRecord {
         id: 1,
         email: "a@example.com".to_string(),
         password_hash: Some("hash".to_string()),
         email_verified: true,
         full_name: "A".to_string(),
         phone: None,
         avatar_url: None,
         date_of_birth: None,
         gender: None,
         tier: "STAR".to_string(),
         loyalty_points: 0,
         status: UserStatus::Active,
         failed_login_attempts: 0,
         locked_until: None,
         last_login_at: None,
      }
   }

   fn now() -> DateTime<Utc> {
      "2026-11-23T10:00:00Z".parse().unwrap()
   }

   #[test]
   fn fifth_failed_login_locks_account_for_lock_duration() {
      let mut user_record: UserRecord = active_user();
      for _ in 0..4 {
         user_record.register_failed_login(5, Duration::minutes(15), now());
      }
      assert_eq!(user_record.status, UserStatus::Active);
      assert_eq!(user_record.failed_login_attempts, 4);

      user_record.register_failed_login(5, Duration::minutes(15), now());
      assert_eq!(user_record.status, UserStatus::Locked);
      assert_eq!(user_record.locked_until, Some(now() + Duration::minutes(15)));
      assert!(!user_record.is_login_allowed(now() + Duration::minutes(14)));
      assert!(user_record.is_login_allowed(now() + Duration::minutes(15)));
   }

   #[test]
   fn failed_login_after_lock_expired_starts_a_fresh_count() {
      let mut user_record: UserRecord = active_user();
      user_record.status = UserStatus::Locked;
      user_record.failed_login_attempts = 5;
      user_record.locked_until = Some(now() - Duration::minutes(1));

      user_record.register_failed_login(5, Duration::minutes(15), now());

      assert_eq!(user_record.status, UserStatus::Active);
      assert_eq!(user_record.failed_login_attempts, 1);
      assert_eq!(user_record.locked_until, None);
   }

   #[test]
   fn successful_login_clears_counters_and_unlocks_expired_lock() {
      let mut user_record: UserRecord = active_user();
      user_record.status = UserStatus::Locked;
      user_record.failed_login_attempts = 5;
      user_record.locked_until = Some(now() - Duration::seconds(1));

      user_record.record_successful_login(now());

      assert_eq!(user_record.status, UserStatus::Active);
      assert_eq!(user_record.failed_login_attempts, 0);
      assert_eq!(user_record.locked_until, None);
      assert_eq!(user_record.last_login_at, Some(now()));
   }

   #[test]
   fn suspended_and_deleted_accounts_never_log_in() {
      let mut user_record: UserRecord = active_user();
      user_record.status = UserStatus::Suspended;
      assert!(!user_record.is_login_allowed(now()));
      user_record.status = UserStatus::Deleted;
      assert!(!user_record.is_login_allowed(now()));
   }

   #[test]
   fn locked_account_without_deadline_counts_as_expired() {
      let mut user_record: UserRecord = active_user();
      user_record.status = UserStatus::Locked;
      user_record.locked_until = None;
      assert!(user_record.is_login_allowed(now()));
   }

   #[test]
   fn blank_password_hash_means_no_password() {
      let mut user_record: UserRecord = active_user();
      assert!(user_record.has_password());
      user_record.password_hash = Some("  ".to_string());
      assert!(!user_record.has_password());
      user_record.password_hash = None;
      assert!(!user_record.has_password());
   }
}
