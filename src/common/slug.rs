use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

pub fn to_slug(name: &str, fallback_slug: &str) -> String {
   let ascii_text: String = name
      .nfd()
      .filter(|character| !is_combining_mark(*character))
      .map(|character| match character {
         'đ' | 'Đ' => 'd',
         other => other.to_ascii_lowercase(),
      })
      .collect();
   let mut slug: String = String::new();
   let mut pending_hyphen: bool = false;
   for character in ascii_text.chars() {
      if character.is_ascii_lowercase() || character.is_ascii_digit() {
         if pending_hyphen && !slug.is_empty() {
            slug.push('-');
         }
         pending_hyphen = false;
         slug.push(character);
      } else {
         pending_hyphen = true;
      }
   }
   if slug.is_empty() {
      fallback_slug.to_string()
   } else {
      slug
   }
}

#[cfg(test)]
mod tests {
   use super::to_slug;

   #[test]
   fn vietnamese_names_become_ascii_slugs() {
      assert_eq!(to_slug("CinemaSphere Quận 1", "cinema"), "cinemasphere-quan-1");
      assert_eq!(to_slug("  Đà Nẵng -- Vincom!  ", "cinema"), "da-nang-vincom");
      assert_eq!(to_slug("Rạp Đồng Khởi", "cinema"), "rap-dong-khoi");
   }

   #[test]
   fn names_without_letters_use_the_fallback() {
      assert_eq!(to_slug("***", "cinema"), "cinema");
      assert_eq!(to_slug("", "cinema"), "cinema");
   }
}
