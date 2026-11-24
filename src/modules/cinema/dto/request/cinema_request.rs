use serde::Deserialize;

use crate::common::error::AppError;
use crate::common::validation::FieldErrors;
use crate::modules::cinema::enums::CinemaStatus;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CinemaRequest {
   pub name: Option<String>,
   pub city: Option<String>,
   pub address: Option<String>,
   pub phone: Option<String>,
   pub latitude: Option<f64>,
   pub longitude: Option<f64>,
   pub status: Option<CinemaStatus>,
   pub facility_ids: Option<Vec<i64>>,
}

pub struct CinemaCommand {
   pub name: String,
   pub city: String,
   pub address: String,
   pub phone: Option<String>,
   pub latitude: Option<f64>,
   pub longitude: Option<f64>,
   pub status: CinemaStatus,
   pub facility_ids: Vec<i64>,
}

fn is_valid_hotline(phone: &str) -> bool {
   let digits: &str = phone.strip_prefix('+').unwrap_or(phone);
   (8..=15).contains(&digits.len()) && digits.chars().all(|character| character.is_ascii_digit())
}

impl CinemaRequest {
   pub fn validate(self) -> Result<CinemaCommand, AppError> {
      let mut field_errors: FieldErrors = FieldErrors::new();
      let name: String = field_errors.required_text("name", &self.name, "Vui lòng nhập tên rạp");
      field_errors.max_chars("name", &name, 255, "Tên rạp tối đa 255 ký tự");
      let city: String = field_errors.required_text("city", &self.city, "Vui lòng chọn tỉnh/thành phố");
      field_errors.max_chars("city", &city, 100, "Tỉnh/thành phố tối đa 100 ký tự");
      let address: String = field_errors.required_text("address", &self.address, "Vui lòng nhập địa chỉ");
      field_errors.max_chars("address", &address, 500, "Địa chỉ tối đa 500 ký tự");
      if let Some(phone) = self.phone.as_deref() {
         if !is_valid_hotline(phone) {
            field_errors.add("phone", "Số hotline không hợp lệ (8 đến 15 chữ số)");
         }
      }
      if self.latitude.is_some_and(|latitude| !(-90.0..=90.0).contains(&latitude)) {
         field_errors.add("latitude", "Vĩ độ phải nằm trong khoảng -90 đến 90");
      }
      if self.longitude.is_some_and(|longitude| !(-180.0..=180.0).contains(&longitude)) {
         field_errors.add("longitude", "Kinh độ phải nằm trong khoảng -180 đến 180");
      }
      if self.status.is_none() {
         field_errors.add("status", "Vui lòng chọn trạng thái");
      }
      field_errors.finish()?;
      Ok(CinemaCommand {
         name: name.trim().to_string(),
         city: city.trim().to_string(),
         address: address.trim().to_string(),
         phone: self.phone.map(|phone| phone.trim().to_string()),
         latitude: self.latitude,
         longitude: self.longitude,
         status: self.status.unwrap_or(CinemaStatus::Active),
         facility_ids: self.facility_ids.unwrap_or_default(),
      })
   }
}

impl CinemaCommand {
   pub fn ensure_coordinates_given_together(&self) -> Result<(), AppError> {
      if self.latitude.is_some() == self.longitude.is_some() {
         Ok(())
      } else {
         Err(AppError::bad_request("INVALID_COORDINATES", "Vĩ độ và kinh độ phải được nhập cùng nhau"))
      }
   }
}

#[cfg(test)]
mod tests {
   use super::CinemaRequest;
   use crate::modules::cinema::dto::request::field_errors_of;

   fn request(value: serde_json::Value) -> CinemaRequest {
      serde_json::from_value(value).unwrap()
   }

   #[test]
   fn empty_request_reports_every_required_field() {
      let data = field_errors_of(request(serde_json::json!({})).validate());
      assert_eq!(data["name"], "Vui lòng nhập tên rạp");
      assert_eq!(data["city"], "Vui lòng chọn tỉnh/thành phố");
      assert_eq!(data["address"], "Vui lòng nhập địa chỉ");
      assert_eq!(data["status"], "Vui lòng chọn trạng thái");
      assert!(data.get("phone").is_none());
   }

   #[test]
   fn hotline_and_coordinates_are_range_checked() {
      let data = field_errors_of(
         request(serde_json::json!({
            "name": "A", "city": "HN", "address": "B", "status": "ACTIVE",
            "phone": "", "latitude": 91.0, "longitude": -181.5
         }))
         .validate(),
      );
      assert_eq!(data["phone"], "Số hotline không hợp lệ (8 đến 15 chữ số)");
      assert_eq!(data["latitude"], "Vĩ độ phải nằm trong khoảng -90 đến 90");
      assert_eq!(data["longitude"], "Kinh độ phải nằm trong khoảng -180 đến 180");
   }

   #[test]
   fn valid_request_is_trimmed_and_lone_coordinate_is_rejected_by_the_command() {
      let cinema_command = request(serde_json::json!({
         "name": "  Rạp A ", "city": " Hà Nội ", "address": " 1 Phố ", "status": "INACTIVE",
         "phone": "+84901234567", "latitude": 21.0, "facilityIds": [1, 2]
      }))
      .validate()
      .unwrap();
      assert_eq!(cinema_command.name, "Rạp A");
      assert_eq!(cinema_command.city, "Hà Nội");
      assert_eq!(cinema_command.facility_ids, vec![1, 2]);
      assert_eq!(cinema_command.ensure_coordinates_given_together().unwrap_err().error_code, "INVALID_COORDINATES");
   }
}
