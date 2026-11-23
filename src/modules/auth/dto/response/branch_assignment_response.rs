use serde::Serialize;

use crate::modules::auth::entity::BranchAssignmentRecord;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchAssignmentResponse {
   pub cinema_id: i64,
   pub cinema_name: Option<String>,
   pub primary: bool,
}

impl From<BranchAssignmentRecord> for BranchAssignmentResponse {
   fn from(branch_assignment_record: BranchAssignmentRecord) -> BranchAssignmentResponse {
      BranchAssignmentResponse {
         cinema_id: branch_assignment_record.cinema_id,
         cinema_name: branch_assignment_record.cinema_name,
         primary: branch_assignment_record.is_primary,
      }
   }
}
