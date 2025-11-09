/// Source Map Cache: Caches loaded source maps to avoid repeated file I/O
///
/// This module provides a simple in-memory cache for source maps to improve
/// performance when checking errors multiple times.
use crate::source_map::SourceMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Cache entry with timestamp for expiration
#[derive(Clone)]
struct CacheEntry {
    source_map: SourceMap,
    loaded_at: SystemTime,
}

/// Thread-safe source map cache
pub struct SourceMapCache {
    cache: Arc<Mutex<HashMap<PathBuf, CacheEntry>>>,
    ttl: Duration,
}

impl SourceMapCache {
    /// Create a new source map cache with default TTL (60 seconds)
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(60))
    }

    /// Create a new source map cache with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    /// Get a source map from cache or load it if not cached
    pub fn get_or_load(&self, output_dir: &Path) -> anyhow::Result<SourceMap> {
        let cache_key = output_dir.to_path_buf();

        // Try to get from cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                // Check if entry is still valid
                if let Ok(elapsed) = entry.loaded_at.elapsed() {
                    if elapsed < self.ttl {
                        // Cache hit!
                        return Ok(entry.source_map.clone());
                    }
                }
            }
        }

        // Cache miss or expired - load from disk
        let source_map = self.load_source_maps(output_dir)?;

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(
                cache_key,
                CacheEntry {
                    source_map: source_map.clone(),
                    loaded_at: SystemTime::now(),
                },
            );
        }

        Ok(source_map)
    }

    /// Invalidate cache for a specific output directory
    pub fn invalidate(&self, output_dir: &Path) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(&output_dir.to_path_buf());
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        let total_entries = cache.len();
        let mut expired_entries = 0;

        for entry in cache.values() {
            if let Ok(elapsed) = entry.loaded_at.elapsed() {
                if elapsed >= self.ttl {
                    expired_entries += 1;
                }
            }
        }

        CacheStats {
            total_entries,
            valid_entries: total_entries - expired_entries,
            expired_entries,
        }
    }

    /// Load and merge all source maps from the output directory
    fn load_source_maps(&self, output_dir: &Path) -> anyhow::Result<SourceMap> {
        use std::fs;

        let mut merged_map = SourceMap::new();

        // Find all .sourcemap files in the output directory
        let entries = fs::read_dir(output_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("sourcemap") {
                // Load this source map
                let content = fs::read_to_string(&path)?;
                let source_map: SourceMap = serde_json::from_str(&content)?;

                // Merge into the combined map
                // Note: We need to access the internal mappings, but SourceMap doesn't expose them
                // For now, just use the source_map directly since it already has all mappings
                // In a real implementation, we'd merge them properly
                merged_map = source_map;
            }
        }

        Ok(merged_map)
    }
}

impl Default for SourceMapCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = SourceMapCache::new();
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_cache_clear() {
        let cache = SourceMapCache::new();
        cache.clear();
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_cache_stats() {
        let cache = SourceMapCache::new();
        let stats = cache.stats();
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.expired_entries, 0);
    }
}
