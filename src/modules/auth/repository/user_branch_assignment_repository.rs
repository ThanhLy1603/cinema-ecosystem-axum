use sqlx::PgExecutor;

use crate::modules::auth::entity::BranchAssignmentRecord;

pub async fn find_assigned_branches(
   executor: impl PgExecutor<'_>,
   user_id: i64,
) -> Result<Vec<BranchAssignmentRecord>, sqlx::Error> {
   sqlx::query_as::<_, BranchAssignmentRecord>(
      "SELECT assignment.cinema_id, cinema.name AS cinema_name, assignment.is_primary \
       FROM user_branch_assignments assignment LEFT JOIN cinemas cinema ON cinema.id = assignment.cinema_id \
       WHERE assignment.user_id = $1 ORDER BY assignment.is_primary DESC, assignment.cinema_id",
   )
   .bind(user_id)
   .fetch_all(executor)
   .await
}
