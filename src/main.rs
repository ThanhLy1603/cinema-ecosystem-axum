use std::net::SocketAddr;

use cinema_ecosystem_axum::app::build_router;
use cinema_ecosystem_axum::config::AppConfig;
use cinema_ecosystem_axum::infrastructure::{database, redis};
use cinema_ecosystem_axum::state::AppState;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
   dotenvy::dotenv().ok();
   tracing_subscriber::fmt()
      .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")))
      .init();

   let app_config: AppConfig = AppConfig::from_env();
   let pg_pool = database::create_pg_pool(&app_config.database_url)
      .await
      .expect("Không kết nối được PostgreSQL, kiểm tra DB_HOST, DB_PORT, DB_NAME, DB_USER, DB_PASSWORD");
   let redis_client = redis::create_redis_client(&app_config.redis_url).expect("REDIS_URL không hợp lệ");

   let router = build_router(AppState { pg_pool, redis_client }, &app_config.cors_allowed_origins);

   let socket_addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], app_config.server_port));
   let tcp_listener: TcpListener = TcpListener::bind(socket_addr).await.expect("Không mở được cổng");
   tracing::info!("cinema-ecosystem-axum lắng nghe tại http://{}", socket_addr);
   axum::serve(tcp_listener, router).with_graceful_shutdown(shutdown_signal()).await.expect("Máy chủ dừng bất thường");
}

async fn shutdown_signal() {
   tokio::signal::ctrl_c().await.ok();
   tracing::info!("Đang tắt máy chủ");
}
