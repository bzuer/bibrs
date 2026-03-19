use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Disk-based response cache.
///
/// Directory structure: `~/.cache/bibrs/<source>/<category>/<hash>.json`
pub struct DiskCache {
    base_dir: PathBuf,
    enabled: bool,
    ttl_search: Duration,
    ttl_id: Duration,
}

impl DiskCache {
    /// Creates a new disk cache with default settings.
    pub fn new(enabled: bool, ttl_search_days: u32, ttl_id_days: u32) -> Self {
        let base_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("bibrs");

        Self {
            base_dir,
            enabled,
            ttl_search: Duration::from_secs(ttl_search_days as u64 * 86400),
            ttl_id: Duration::from_secs(ttl_id_days as u64 * 86400),
        }
    }

    /// Creates a cache with a custom base directory (for testing).
    pub fn with_base_dir(base_dir: PathBuf, enabled: bool) -> Self {
        Self {
            base_dir,
            enabled,
            ttl_search: Duration::from_secs(7 * 86400),
            ttl_id: Duration::from_secs(30 * 86400),
        }
    }

    /// Retrieves a cached response.
    pub fn get(&self, source: &str, category: &str, key: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let path = self.path_for(source, category, key);
        if !path.exists() {
            tracing::debug!(path = %path.display(), "cache miss");
            return None;
        }

        let ttl = if category == "search" {
            self.ttl_search
        } else {
            self.ttl_id
        };

        if is_expired(&path, ttl) {
            tracing::debug!(path = %path.display(), "cache expired");
            let _ = std::fs::remove_file(&path);
            return None;
        }

        tracing::debug!(path = %path.display(), "cache hit");
        std::fs::read_to_string(&path).ok()
    }

    /// Stores a response in the cache.
    pub fn put(&self, source: &str, category: &str, key: &str, data: &str) {
        if !self.enabled {
            return;
        }

        let path = self.path_for(source, category, key);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        tracing::debug!(path = %path.display(), "cache store");
        let _ = std::fs::write(&path, data);
    }

    fn path_for(&self, source: &str, category: &str, key: &str) -> PathBuf {
        let hash = hash_key(key);
        self.base_dir
            .join(source)
            .join(category)
            .join(format!("{}.json", hash))
    }
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn is_expired(path: &Path, ttl: Duration) -> bool {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age > ttl)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_put_and_get() {
        let dir = std::env::temp_dir().join("bibrs_cache_test");
        let cache = DiskCache::with_base_dir(dir.clone(), true);

        cache.put("crossref", "doi", "10.1000/test", "{\"result\": true}");
        let result = cache.get("crossref", "doi", "10.1000/test");
        assert_eq!(result, Some("{\"result\": true}".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_miss_when_disabled() {
        let dir = std::env::temp_dir().join("bibrs_cache_disabled");
        let cache = DiskCache::with_base_dir(dir.clone(), false);

        cache.put("crossref", "doi", "10.1000/test", "data");
        let result = cache.get("crossref", "doi", "10.1000/test");
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_miss_nonexistent() {
        let dir = std::env::temp_dir().join("bibrs_cache_miss");
        let cache = DiskCache::with_base_dir(dir.clone(), true);

        let result = cache.get("crossref", "doi", "nonexistent");
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hash_key_deterministic() {
        let h1 = hash_key("test");
        let h2 = hash_key("test");
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_key("different"));
    }
}
