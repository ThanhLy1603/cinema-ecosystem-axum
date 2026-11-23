use std::time::Duration;

const KEY_PREFIX: &str = "rate-limit:";

pub async fn try_acquire(redis_client: &redis::Client, key: &str, max_attempts: i64, window: Duration) -> bool {
   match count_attempt(redis_client, key, window).await {
      Ok(attempts) => attempts <= max_attempts,
      Err(redis_error) => {
         tracing::warn!(error = %redis_error, key, "Redis không khả dụng, bỏ qua giới hạn tần suất");
         true
      }
   }
}

async fn count_attempt(redis_client: &redis::Client, key: &str, window: Duration) -> redis::RedisResult<i64> {
   let redis_key: String = format!("{KEY_PREFIX}{key}");
   let mut connection = redis_client.get_multiplexed_async_connection().await?;
   redis::cmd("SET")
      .arg(&redis_key)
      .arg(0)
      .arg("NX")
      .arg("EX")
      .arg(window.as_secs())
      .query_async::<redis::Value>(&mut connection)
      .await?;
   redis::cmd("INCR").arg(&redis_key).query_async::<i64>(&mut connection).await
}
