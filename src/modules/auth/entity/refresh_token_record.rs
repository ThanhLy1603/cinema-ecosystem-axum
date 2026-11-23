use chrono::{DateTime, Utc};

#[derive(Clone, sqlx::FromRow)]
pub struct RefreshTokenRecord {
   pub id: i64,
   pub user_id: i64,
   pub family_id: uuid::Uuid,
   pub expires_at: DateTime<Utc>,
   pub revoked: bool,
   pub revoked_at: Option<DateTime<Utc>>,
   pub replaced_by_id: Option<i64>,
}
