use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use cinema_ecosystem_axum::app::build_router;
use cinema_ecosystem_axum::infrastructure::{database, redis};
use cinema_ecosystem_axum::state::AppState;
use http_body_util::BodyExt;
use tower::ServiceExt;

pub const UNREACHABLE_DATABASE_URL: &str = "postgres://nobody@127.0.0.1:1/none";

pub fn router_for(database_url: &str) -> Router {
   let pg_pool = database::create_lazy_pg_pool(database_url).expect("database url");
   let redis_client = redis::create_redis_client("redis://127.0.0.1:1").expect("redis url");
   build_router(AppState { pg_pool, redis_client }, &["http://localhost:3000".to_string()])
}

pub async fn get_json(router: Router, uri: &str) -> (StatusCode, serde_json::Value) {
   let response = router.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
   let status: StatusCode = response.status();
   let bytes = response.into_body().collect().await.unwrap().to_bytes();
   (status, serde_json::from_slice(&bytes).unwrap())
}
