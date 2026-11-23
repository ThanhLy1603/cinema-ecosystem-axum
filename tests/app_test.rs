mod common;

use axum::http::StatusCode;
use common::{get_json, router_for, UNREACHABLE_DATABASE_URL};

#[tokio::test]
async fn unknown_route_returns_not_found_in_api_response_shape() {
   let (status, body) = get_json(router_for(UNREACHABLE_DATABASE_URL), "/api/v1/does-not-exist").await;
   assert_eq!(status, StatusCode::NOT_FOUND);
   assert_eq!(body["success"], false);
   assert_eq!(body["errorCode"], "NOT_FOUND");
}
