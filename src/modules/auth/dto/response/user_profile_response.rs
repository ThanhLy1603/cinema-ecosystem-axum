use chrono::NaiveDate;
use serde::Serialize;

use super::BranchAssignmentResponse;
use crate::modules::auth::entity::UserRecord;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponse {
   pub id: i64,
   pub email: String,
   pub full_name: String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub phone: Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub avatar_url: Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub date_of_birth: Option<NaiveDate>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub gender: Option<String>,
   pub roles: Vec<String>,
   pub loyalty_points: i32,
   pub tier: String,
   pub email_verified: bool,
   pub has_password: bool,
   pub linked_providers: Vec<String>,
   pub branches: Vec<BranchAssignmentResponse>,
}

impl UserProfileResponse {
   pub fn from_record(
      user_record: &UserRecord,
      role_codes: &[String],
      linked_providers: Vec<String>,
      branches: Vec<BranchAssignmentResponse>,
   ) -> UserProfileResponse {
      let mut sorted_role_codes: Vec<String> = role_codes.to_vec();
      sorted_role_codes.sort();
      let mut sorted_providers: Vec<String> = linked_providers;
      sorted_providers.sort();
      UserProfileResponse {
         id: user_record.id,
         email: user_record.email.clone(),
         full_name: user_record.full_name.clone(),
         phone: user_record.phone.clone(),
         avatar_url: user_record.avatar_url.clone(),
         date_of_birth: user_record.date_of_birth,
         gender: user_record.gender.clone(),
         roles: sorted_role_codes,
         loyalty_points: user_record.loyalty_points,
         tier: user_record.tier.clone(),
         email_verified: user_record.email_verified,
         has_password: user_record.has_password(),
         linked_providers: sorted_providers,
         branches,
      }
   }
}
