mod common;

use axum::http::StatusCode;
use common::{get_json, router_for, UNREACHABLE_DATABASE_URL};

#[tokio::test]
async fn health_reports_down_when_database_unreachable() {
   let (status, body) = get_json(router_for(UNREACHABLE_DATABASE_URL), "/api/v1/health").await;
   assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
   assert_eq!(body["data"]["database"], "DOWN");
}
