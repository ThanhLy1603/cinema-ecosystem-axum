use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenType {
   #[serde(rename = "2D", alias = "2d")]
   Screen2d,
   #[serde(rename = "3D", alias = "3d")]
   Screen3d,
   #[serde(rename = "IMAX", alias = "imax")]
   Imax,
   #[serde(rename = "4DX", alias = "4dx")]
   Screen4dx,
}

impl ScreenType {
   pub fn as_str(self) -> &'static str {
      match self {
         ScreenType::Screen2d => "2D",
         ScreenType::Screen3d => "3D",
         ScreenType::Imax => "IMAX",
         ScreenType::Screen4dx => "4DX",
      }
   }
}

impl TryFrom<String> for ScreenType {
   type Error = String;

   fn try_from(code: String) -> Result<ScreenType, String> {
      match code.to_ascii_uppercase().as_str() {
         "2D" => Ok(ScreenType::Screen2d),
         "3D" => Ok(ScreenType::Screen3d),
         "IMAX" => Ok(ScreenType::Imax),
         "4DX" => Ok(ScreenType::Screen4dx),
         other => Err(format!("Loại màn hình không hợp lệ: {other}")),
      }
   }
}

#[cfg(test)]
mod tests {
   use super::ScreenType;

   #[test]
   fn screen_type_uses_the_database_codes_in_json() {
      for screen_type in [ScreenType::Screen2d, ScreenType::Screen3d, ScreenType::Imax, ScreenType::Screen4dx] {
         assert_eq!(serde_json::to_value(screen_type).unwrap(), screen_type.as_str());
         assert_eq!(ScreenType::try_from(screen_type.as_str().to_string()).unwrap(), screen_type);
      }
      assert_eq!(serde_json::from_str::<ScreenType>("\"imax\"").unwrap(), ScreenType::Imax);
      assert!(serde_json::from_str::<ScreenType>("\"5D\"").is_err());
   }
}
