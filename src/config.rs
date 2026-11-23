use std::env;
use std::fmt;

const MIN_JWT_SECRET_BYTES: usize = 32;

#[derive(Clone)]
pub struct JwtConfig {
   pub secret: String,
   pub access_token_ttl_seconds: i64,
   pub refresh_token_ttl_seconds: i64,
}

impl fmt::Debug for JwtConfig {
   fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
      formatter
         .debug_struct("JwtConfig")
         .field("secret", &"***")
         .field("access_token_ttl_seconds", &self.access_token_ttl_seconds)
         .field("refresh_token_ttl_seconds", &self.refresh_token_ttl_seconds)
         .finish()
   }
}

#[derive(Clone)]
pub struct MailConfig {
   pub host: String,
   pub port: u16,
   pub username: String,
   pub password: String,
   pub starttls: bool,
   pub from: String,
}

impl fmt::Debug for MailConfig {
   fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
      formatter
         .debug_struct("MailConfig")
         .field("host", &self.host)
         .field("port", &self.port)
         .field("username", &self.username)
         .field("password", &"***")
         .field("starttls", &self.starttls)
         .field("from", &self.from)
         .finish()
   }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
   pub server_port: u16,
   pub database_url: String,
   pub redis_url: String,
   pub cors_allowed_origins: Vec<String>,
   pub jwt: JwtConfig,
   pub cookie_secure: bool,
   pub bcrypt_cost: u32,
   pub frontend_base_url: String,
   pub mail: MailConfig,
}

impl AppConfig {
   pub fn from_env() -> Result<AppConfig, String> {
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
      let jwt_secret: String =
         env::var("JWT_SECRET").map_err(|_| "Thiếu biến môi trường JWT_SECRET (tối thiểu 32 byte)".to_string())?;
      if jwt_secret.len() < MIN_JWT_SECRET_BYTES {
         return Err(format!("JWT_SECRET phải dài tối thiểu {MIN_JWT_SECRET_BYTES} byte (256 bit) cho HS256"));
      }
      let jwt: JwtConfig = JwtConfig {
         secret: jwt_secret,
         access_token_ttl_seconds: env_or("JWT_ACCESS_TOKEN_EXPIRATION_SECONDS", "900").parse().unwrap_or(900),
         refresh_token_ttl_seconds: env_or("JWT_REFRESH_TOKEN_EXPIRATION_SECONDS", "604800").parse().unwrap_or(604800),
      };
      let mail: MailConfig = MailConfig {
         host: env_or("MAIL_HOST", "localhost"),
         port: env_or("MAIL_PORT", "1025").parse().unwrap_or(1025),
         username: env_or("MAIL_USERNAME", ""),
         password: env_or("MAIL_PASSWORD", ""),
         starttls: env_or("MAIL_STARTTLS", "false").eq_ignore_ascii_case("true"),
         from: env_or("MAIL_FROM", "CinemaSphere <noreply@cinemasphere.local>"),
      };
      Ok(AppConfig {
         server_port,
         database_url,
         redis_url,
         cors_allowed_origins,
         jwt,
         cookie_secure: env_or("COOKIE_SECURE", "false").eq_ignore_ascii_case("true"),
         bcrypt_cost: 12,
         frontend_base_url: env_or("FRONTEND_BASE_URL", "http://localhost:5173"),
         mail,
      })
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
