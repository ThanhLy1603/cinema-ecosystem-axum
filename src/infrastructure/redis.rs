pub fn create_redis_client(redis_url: &str) -> Result<redis::Client, redis::RedisError> {
   redis::Client::open(redis_url)
}

pub async fn ping(redis_client: &redis::Client) -> bool {
   match redis_client.get_multiplexed_async_connection().await {
      Ok(mut connection) => redis::cmd("PING").query_async::<String>(&mut connection).await.is_ok(),
      Err(_) => false,
   }
}
