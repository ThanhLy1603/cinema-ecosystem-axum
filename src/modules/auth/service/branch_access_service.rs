use std::collections::BTreeSet;
use std::time::Duration;

use redis::AsyncCommands;

use crate::common::error::AppError;
use crate::modules::auth::repository::role_repository;
use crate::modules::auth::repository::user_branch_assignment_repository;
use crate::state::AppState;

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_KEY_PREFIX: &str = "branch-access:";
const ALL_BRANCHES_MARKER: &str = "*";
const ID_SEPARATOR: char = ',';
const ROLE_ADMIN: &str = "ROLE_ADMIN";
const ROLE_STAFF: &str = "ROLE_STAFF";
const ROLE_MANAGER: &str = "ROLE_MANAGER";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchScope {
   All,
   Only(BTreeSet<i64>),
}

impl BranchScope {
   pub fn can_access(&self, cinema_id: i64) -> bool {
      match self {
         BranchScope::All => true,
         BranchScope::Only(cinema_ids) => cinema_ids.contains(&cinema_id),
      }
   }

   pub fn has_access_to_all(&self) -> bool {
      matches!(self, BranchScope::All)
   }

   fn encode(&self) -> String {
      match self {
         BranchScope::All => ALL_BRANCHES_MARKER.to_string(),
         BranchScope::Only(cinema_ids) => cinema_ids
            .iter()
            .map(|cinema_id| cinema_id.to_string())
            .collect::<Vec<String>>()
            .join(&ID_SEPARATOR.to_string()),
      }
   }

   fn decode(cached_value: &str) -> Option<BranchScope> {
      if cached_value == ALL_BRANCHES_MARKER {
         return Some(BranchScope::All);
      }
      if cached_value.is_empty() {
         return Some(BranchScope::Only(BTreeSet::new()));
      }
      cached_value
         .split(ID_SEPARATOR)
         .map(|cinema_id| cinema_id.parse::<i64>().ok())
         .collect::<Option<BTreeSet<i64>>>()
         .map(BranchScope::Only)
   }
}

fn branch_access_denied() -> AppError {
   AppError::forbidden("BRANCH_ACCESS_DENIED", "Bạn không có quyền truy cập dữ liệu của chi nhánh này")
}

fn cache_key(user_id: i64) -> String {
   format!("{CACHE_KEY_PREFIX}{user_id}")
}

pub async fn resolve_branch_scope(app_state: &AppState, user_id: i64) -> Result<BranchScope, AppError> {
   if let Some(cached_scope) = read_cache(app_state, user_id).await {
      return Ok(cached_scope);
   }
   let branch_scope: BranchScope = load_branch_scope(app_state, user_id).await?;
   write_cache(app_state, user_id, &branch_scope).await;
   Ok(branch_scope)
}

pub async fn assert_can_access_cinema(app_state: &AppState, user_id: i64, cinema_id: i64) -> Result<(), AppError> {
   if resolve_branch_scope(app_state, user_id).await?.can_access(cinema_id) {
      Ok(())
   } else {
      Err(branch_access_denied())
   }
}

pub async fn evict_branch_scope(app_state: &AppState, user_id: i64) {
   let result: redis::RedisResult<i64> = async {
      let mut connection = app_state.redis_client.get_multiplexed_async_connection().await?;
      connection.del(cache_key(user_id)).await
   }
   .await;
   if let Err(redis_error) = result {
      tracing::warn!(user_id, error = %redis_error, "Không xóa được cache phạm vi chi nhánh");
   }
}

async fn load_branch_scope(app_state: &AppState, user_id: i64) -> Result<BranchScope, AppError> {
   let role_codes: Vec<String> = role_repository::find_role_codes(&app_state.pg_pool, user_id).await?;
   if role_codes.iter().any(|role_code| role_code == ROLE_ADMIN) {
      return Ok(BranchScope::All);
   }
   if role_codes.iter().any(|role_code| role_code == ROLE_STAFF || role_code == ROLE_MANAGER) {
      let cinema_ids: Vec<i64> =
         user_branch_assignment_repository::find_assigned_cinema_ids(&app_state.pg_pool, user_id).await?;
      return Ok(BranchScope::Only(cinema_ids.into_iter().collect()));
   }
   Ok(BranchScope::Only(BTreeSet::new()))
}

async fn read_cache(app_state: &AppState, user_id: i64) -> Option<BranchScope> {
   let result: redis::RedisResult<Option<String>> = async {
      let mut connection = app_state.redis_client.get_multiplexed_async_connection().await?;
      connection.get(cache_key(user_id)).await
   }
   .await;
   match result {
      Ok(Some(cached_value)) => {
         let branch_scope: Option<BranchScope> = BranchScope::decode(&cached_value);
         if branch_scope.is_none() {
            tracing::warn!(user_id, "Cache phạm vi chi nhánh bị hỏng, tải lại từ DB");
         }
         branch_scope
      }
      Ok(None) => None,
      Err(redis_error) => {
         tracing::warn!(user_id, error = %redis_error, "Không đọc được cache phạm vi chi nhánh");
         None
      }
   }
}

async fn write_cache(app_state: &AppState, user_id: i64, branch_scope: &BranchScope) {
   let result: redis::RedisResult<()> = async {
      let mut connection = app_state.redis_client.get_multiplexed_async_connection().await?;
      connection.set_ex(cache_key(user_id), branch_scope.encode(), CACHE_TTL.as_secs()).await
   }
   .await;
   if let Err(redis_error) = result {
      tracing::warn!(user_id, error = %redis_error, "Không ghi được cache phạm vi chi nhánh");
   }
}

#[cfg(test)]
mod tests {
   use std::collections::BTreeSet;

   use super::BranchScope;

   #[test]
   fn scope_round_trips_through_the_cache_encoding_shared_with_spring() {
      let only: BranchScope = BranchScope::Only(BTreeSet::from([3, 1, 2]));
      assert_eq!(only.encode(), "1,2,3");
      assert_eq!(BranchScope::decode("1,2,3"), Some(only));
      assert_eq!(BranchScope::decode("*"), Some(BranchScope::All));
      assert_eq!(BranchScope::decode(""), Some(BranchScope::Only(BTreeSet::new())));
      assert_eq!(BranchScope::decode("1,x"), None);
   }

   #[test]
   fn only_scope_limits_access_to_assigned_cinemas() {
      let only: BranchScope = BranchScope::Only(BTreeSet::from([1]));
      assert!(only.can_access(1) && !only.can_access(2) && !only.has_access_to_all());
      assert!(BranchScope::All.can_access(99) && BranchScope::All.has_access_to_all());
   }
}
