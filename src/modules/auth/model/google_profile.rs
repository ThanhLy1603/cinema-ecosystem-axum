#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleProfile {
   pub subject: String,
   pub email: String,
   pub full_name: Option<String>,
   pub avatar_url: Option<String>,
}
