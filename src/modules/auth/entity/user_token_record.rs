use chrono::{DateTime, Utc};

#[derive(Clone, sqlx::FromRow)]
pub struct UserTokenRecord {
   pub id: i64,
   pub user_id: i64,
   pub expires_at: DateTime<Utc>,
   pub used_at: Option<DateTime<Utc>>,
}

impl UserTokenRecord {
   pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
      self.used_at.is_none() && now < self.expires_at
   }
}
