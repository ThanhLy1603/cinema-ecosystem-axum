use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
   pub server_port: u16,
   pub database_url: String,
   pub redis_url: String,
   pub cors_allowed_origins: Vec<String>,
}

impl AppConfig {
   pub fn from_env() -> AppConfig {
      let server_port: u16 = env_or("SERVER_PORT", "8081").parse().unwrap_or(8081);
      let database_url: String = env::var("DATABASE_URL").unwrap_or_else(|_| build_database_url());
      let redis_url: String = env::var("REDIS_URL")
         .unwrap_or_else(|_| format!("redis://{}:{}", env_or("REDIS_HOST", "localhost"), env_or("REDIS_PORT", "6379")));
      let cors_allowed_origins: Vec<String> =
         env_or("CORS_ALLOWED_ORIGINS", "http://localhost:3000,http://127.0.0.1:3000")
            .split(',')
            .map(|origin| origin.trim().to_string())
            .filter(|origin| !origin.is_empty())
            .collect();
      AppConfig { server_port, database_url, redis_url, cors_allowed_origins }
   }
}

fn build_database_url() -> String {
   let host: String = env_or("DB_HOST", "localhost");
   let port: String = env_or("DB_PORT", "5431");
   let name: String = env_or("DB_NAME", "cinema_ecosystem_db");
   let user: String = env_or("DB_USER", "postgres");
   let password: String = env_or("DB_PASSWORD", "");
   if password.is_empty() {
      format!("postgres://{user}@{host}:{port}/{name}")
   } else {
      format!("postgres://{user}:{password}@{host}:{port}/{name}")
   }
}

fn env_or(key: &str, default_value: &str) -> String {
   env::var(key).unwrap_or_else(|_| default_value.to_string())
}
