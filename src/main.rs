use std::net::SocketAddr;

use cinema_ecosystem_axum::app::build_router;
use cinema_ecosystem_axum::config::AppConfig;
use cinema_ecosystem_axum::infrastructure::mailer::Mailer;
use cinema_ecosystem_axum::infrastructure::{database, redis};
use cinema_ecosystem_axum::modules::auth::spawn_token_cleanup;
use cinema_ecosystem_axum::state::AppState;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
   dotenvy::dotenv().ok();
   tracing_subscriber::fmt()
      .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")))
      .init();

   let app_config: AppConfig = AppConfig::from_env().unwrap_or_else(|config_error| {
      eprintln!("Cấu hình không hợp lệ: {config_error}");
      std::process::exit(1);
   });
   let pg_pool = database::create_pg_pool(&app_config.database_url)
      .await
      .expect("Không kết nối được PostgreSQL, kiểm tra DB_HOST, DB_PORT, DB_NAME, DB_USER, DB_PASSWORD");
   let redis_client = redis::create_redis_client(&app_config.redis_url).expect("REDIS_URL không hợp lệ");
   let mailer: Mailer =
      Mailer::smtp(&app_config.mail).expect("Cấu hình email không hợp lệ, kiểm tra MAIL_HOST, MAIL_FROM");

   let cors_allowed_origins: Vec<String> = app_config.cors_allowed_origins.clone();
   let server_port: u16 = app_config.server_port;
   let app_state: AppState =
      AppState::new(app_config, pg_pool.clone(), redis_client, mailer).expect("Không khởi tạo được AppState");
   spawn_token_cleanup(pg_pool);

   let router = build_router(app_state, &cors_allowed_origins);

   let socket_addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], server_port));
   let tcp_listener: TcpListener = TcpListener::bind(socket_addr).await.expect("Không mở được cổng");
   tracing::info!("cinema-ecosystem-axum lắng nghe tại http://{}", socket_addr);
   axum::serve(tcp_listener, router.into_make_service_with_connect_info::<SocketAddr>())
      .with_graceful_shutdown(shutdown_signal())
      .await
      .expect("Máy chủ dừng bất thường");
}

async fn shutdown_signal() {
   tokio::signal::ctrl_c().await.ok();
   tracing::info!("Đang tắt máy chủ");
}
