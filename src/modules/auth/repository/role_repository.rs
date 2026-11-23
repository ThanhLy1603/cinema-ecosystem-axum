use sqlx::PgExecutor;

pub async fn find_role_codes(executor: impl PgExecutor<'_>, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
   sqlx::query_scalar::<_, String>(
      "SELECT role.code FROM roles role JOIN user_roles user_role ON user_role.role_id = role.id \
       WHERE user_role.user_id = $1 ORDER BY role.code",
   )
   .bind(user_id)
   .fetch_all(executor)
   .await
}

pub async fn find_role_id_by_code(executor: impl PgExecutor<'_>, code: &str) -> Result<Option<i64>, sqlx::Error> {
   sqlx::query_scalar::<_, i64>("SELECT id FROM roles WHERE code = $1").bind(code).fetch_optional(executor).await
}

pub async fn insert_user_role(executor: impl PgExecutor<'_>, user_id: i64, role_id: i64) -> Result<(), sqlx::Error> {
   sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
      .bind(user_id)
      .bind(role_id)
      .execute(executor)
      .await
      .map(|_| ())
}
