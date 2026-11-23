use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
   pub status: &'static str,
   pub database: &'static str,
   pub redis: &'static str,
}
