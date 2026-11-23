use serde::Serialize;

use crate::modules::auth::entity::UserRecord;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
   pub id: i64,
   pub email: String,
   pub full_name: String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub phone: Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub avatar_url: Option<String>,
   pub roles: Vec<String>,
   pub loyalty_points: i32,
   pub tier: String,
   pub email_verified: bool,
}

impl UserResponse {
   pub fn from_record(user_record: &UserRecord, role_codes: &[String]) -> UserResponse {
      let mut sorted_role_codes: Vec<String> = role_codes.to_vec();
      sorted_role_codes.sort();
      UserResponse {
         id: user_record.id,
         email: user_record.email.clone(),
         full_name: user_record.full_name.clone(),
         phone: user_record.phone.clone(),
         avatar_url: user_record.avatar_url.clone(),
         roles: sorted_role_codes,
         loyalty_points: user_record.loyalty_points,
         tier: user_record.tier.clone(),
         email_verified: user_record.email_verified,
      }
   }
}
