use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

const RANDOM_TOKEN_BYTES: usize = 32;

pub fn generate_random_token() -> String {
   let mut random_bytes: [u8; RANDOM_TOKEN_BYTES] = [0; RANDOM_TOKEN_BYTES];
   rand::rng().fill_bytes(&mut random_bytes);
   URL_SAFE_NO_PAD.encode(random_bytes)
}

pub fn sha256_hex(value: &str) -> String {
   hex::encode(Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
   use super::{generate_random_token, sha256_hex};

   #[test]
   fn random_token_is_43_url_safe_characters_and_unique() {
      let first_token: String = generate_random_token();
      let second_token: String = generate_random_token();
      assert_eq!(first_token.len(), 43);
      assert!(first_token
         .chars()
         .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_'));
      assert_ne!(first_token, second_token);
   }

   #[test]
   fn sha256_matches_known_vector() {
      assert_eq!(sha256_hex("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
   }
}
