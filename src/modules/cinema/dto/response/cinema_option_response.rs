use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CinemaOptionResponse {
   pub id: i64,
   pub name: String,
   pub city: String,
}
