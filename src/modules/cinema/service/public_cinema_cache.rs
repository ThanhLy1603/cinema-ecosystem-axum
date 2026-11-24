use std::time::Duration;

use crate::infrastructure::versioned_cache::VersionedCache;

pub static PUBLIC_CINEMA_CACHE: VersionedCache =
   VersionedCache::new("cinema:public:", Duration::from_secs(60 * 60), "danh mục rạp");
