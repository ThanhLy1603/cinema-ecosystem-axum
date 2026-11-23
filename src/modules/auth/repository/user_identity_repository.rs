use sqlx::PgExecutor;

pub async fn find_linked_providers(executor: impl PgExecutor<'_>, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
   sqlx::query_scalar::<_, String>("SELECT provider FROM user_identities WHERE user_id = $1")
      .bind(user_id)
      .fetch_all(executor)
      .await
}
