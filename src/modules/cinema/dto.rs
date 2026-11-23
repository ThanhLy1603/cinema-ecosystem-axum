use serde::{Deserialize, Serialize};

use super::repository::CinemaRow;

#[derive(Deserialize)]
pub struct CityQuery {
   pub city: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FacilityResponse {
   pub id: i64,
   pub name: String,
   pub icon_url: Option<String>,
   pub description: Option<String>,
   pub deleted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CinemaResponse {
   pub id: i64,
   pub name: String,
   pub slug: String,
   pub city: String,
   pub address: String,
   pub phone: Option<String>,
   pub latitude: Option<f64>,
   pub longitude: Option<f64>,
   pub status: String,
   pub facilities: Vec<FacilityResponse>,
   pub hall_count: i32,
   pub deleted: bool,
}

impl From<CinemaRow> for CinemaResponse {
   fn from(cinema_row: CinemaRow) -> CinemaResponse {
      CinemaResponse {
         id: cinema_row.id,
         name: cinema_row.name,
         slug: cinema_row.slug,
         city: cinema_row.city,
         address: cinema_row.address,
         phone: cinema_row.phone,
         latitude: cinema_row.latitude,
         longitude: cinema_row.longitude,
         status: cinema_row.status,
         facilities: cinema_row.facilities.0,
         hall_count: cinema_row.hall_count,
         deleted: false,
      }
   }
}
