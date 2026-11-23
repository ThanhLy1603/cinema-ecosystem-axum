mod common;

use axum::http::StatusCode;
use common::{get_json, router_for, UNREACHABLE_DATABASE_URL};

#[tokio::test]
async fn database_error_is_mapped_to_internal_error() {
   let (status, body) = get_json(router_for(UNREACHABLE_DATABASE_URL), "/api/v1/cinemas/public/cities").await;
   assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
   assert_eq!(body["errorCode"], "INTERNAL_SERVER_ERROR");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL trỏ tới PostgreSQL đã chạy migration của Spring"]
async fn public_cinemas_and_cities_read_from_real_database() {
   let database_url: String = std::env::var("DATABASE_URL").expect("DATABASE_URL");
   let router = router_for(&database_url);
   let (status, body) = get_json(router.clone(), "/api/v1/cinemas/public").await;
   assert_eq!(status, StatusCode::OK);
   assert_eq!(body["success"], true);
   assert!(body["data"].is_array());
   let (cities_status, cities_body) = get_json(router, "/api/v1/cinemas/public/cities").await;
   assert_eq!(cities_status, StatusCode::OK);
   assert!(cities_body["data"].is_array());
}
