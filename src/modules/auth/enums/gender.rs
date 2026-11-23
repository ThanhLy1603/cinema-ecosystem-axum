use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Gender {
   Male,
   Female,
   Other,
}

impl Gender {
   pub fn as_str(self) -> &'static str {
      match self {
         Gender::Male => "MALE",
         Gender::Female => "FEMALE",
         Gender::Other => "OTHER",
      }
   }
}
