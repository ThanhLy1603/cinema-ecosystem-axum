use std::future::Future;
use std::time::Duration;

use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;

const DEFAULT_VERSION: &str = "0";

pub struct VersionedCache {
   key_prefix: &'static str,
   time_to_live: Duration,
   label: &'static str,
}

impl VersionedCache {
   pub const fn new(key_prefix: &'static str, time_to_live: Duration, label: &'static str) -> VersionedCache {
      VersionedCache { key_prefix, time_to_live, label }
   }

   fn version_key(&self) -> String {
      format!("{}version", self.key_prefix)
   }

   pub async fn get_or_load<T, E, F, Fut>(&self, redis_client: &redis::Client, name: &str, loader: F) -> Result<T, E>
   where
      T: Serialize + DeserializeOwned,
      F: FnOnce() -> Fut,
      Fut: Future<Output = Result<T, E>>,
   {
      let mut cache_key: Option<String> = None;
      match self.read(redis_client, name).await {
         Ok((read_key, Some(cached_value))) => {
            cache_key = Some(read_key);
            if let Ok(value) = serde_json::from_str::<T>(&cached_value) {
               return Ok(value);
            }
            tracing::warn!(label = self.label, name, "Cache hỏng, tải lại từ DB");
         }
         Ok((read_key, None)) => cache_key = Some(read_key),
         Err(redis_error) => {
            tracing::warn!(label = self.label, name, error = %redis_error, "Không đọc được cache");
         }
      }
      let loaded_value: T = loader().await?;
      if let Some(cache_key) = cache_key {
         self.write(redis_client, &cache_key, &loaded_value).await;
      }
      Ok(loaded_value)
   }

   pub async fn evict(&self, redis_client: &redis::Client) {
      let result: redis::RedisResult<i64> = async {
         let mut connection = redis_client.get_multiplexed_async_connection().await?;
         connection.incr(self.version_key(), 1).await
      }
      .await;
      if let Err(redis_error) = result {
         tracing::warn!(label = self.label, error = %redis_error, "Không làm mới được cache");
      }
   }

   async fn read(&self, redis_client: &redis::Client, name: &str) -> redis::RedisResult<(String, Option<String>)> {
      let mut connection = redis_client.get_multiplexed_async_connection().await?;
      let version: Option<String> = connection.get(self.version_key()).await?;
      let cache_key: String = format!("{}{}:{}", self.key_prefix, version.as_deref().unwrap_or(DEFAULT_VERSION), name);
      let cached_value: Option<String> = connection.get(&cache_key).await?;
      Ok((cache_key, cached_value))
   }

   async fn write<T: Serialize>(&self, redis_client: &redis::Client, cache_key: &str, value: &T) {
      let Ok(json) = serde_json::to_string(value) else {
         return;
      };
      let result: redis::RedisResult<()> = async {
         let mut connection = redis_client.get_multiplexed_async_connection().await?;
         connection.set_ex(cache_key, json, self.time_to_live.as_secs()).await
      }
      .await;
      if let Err(redis_error) = result {
         tracing::warn!(label = self.label, cache_key, error = %redis_error, "Không ghi được cache");
      }
   }
}
