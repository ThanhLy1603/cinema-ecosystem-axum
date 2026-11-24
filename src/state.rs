use std::sync::Arc;

use sqlx::PgPool;

use crate::config::AppConfig;
use crate::infrastructure::mailer::Mailer;
use crate::modules::auth::{GoogleIdTokenVerifier, JwtService, PasswordService};

#[derive(Clone)]
pub struct AppState {
   pub pg_pool: PgPool,
   pub redis_client: redis::Client,
   pub app_config: Arc<AppConfig>,
   pub jwt_service: Arc<JwtService>,
   pub password_service: Arc<PasswordService>,
   pub mailer: Arc<Mailer>,
   pub google_id_token_verifier: Arc<GoogleIdTokenVerifier>,
}

impl AppState {
   pub fn new(
      app_config: AppConfig,
      pg_pool: PgPool,
      redis_client: redis::Client,
      mailer: Mailer,
   ) -> Result<AppState, String> {
      let jwt_service: JwtService = JwtService::new(&app_config.jwt.secret)?;
      let password_service: PasswordService = PasswordService::new(app_config.bcrypt_cost);
      let google_id_token_verifier: GoogleIdTokenVerifier = GoogleIdTokenVerifier::remote(&app_config.google_client_id);
      Ok(AppState {
         pg_pool,
         redis_client,
         app_config: Arc::new(app_config),
         jwt_service: Arc::new(jwt_service),
         password_service: Arc::new(password_service),
         mailer: Arc::new(mailer),
         google_id_token_verifier: Arc::new(google_id_token_verifier),
      })
   }
}
