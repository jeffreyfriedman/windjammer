//! Persistent disk cache for fast startup
//!
//! This module provides on-disk caching of parsed ASTs and symbol tables
//! to enable near-instant LSP server startup on subsequent runs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tower_lsp::lsp_types::Url;

/// Cache version - increment when cache format changes
const CACHE_VERSION: u32 = 1;

/// Cache entry for a single source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// URI of the source file
    pub uri: String,

    /// Hash of file content (for invalidation)
    pub content_hash: u64,

    /// Last modified time of the source file
    pub modified_time: SystemTime,

    /// Cached symbols (names only for now)
    pub symbols: Vec<CachedSymbol>,

    /// Cached imports
    pub imports: Vec<String>,
}

/// A cached symbol (simplified for serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSymbol {
    pub name: String,
    pub kind: String, // "Function", "Struct", etc.
    pub line: u32,
    pub character: u32,
    pub type_info: Option<String>,
}

/// The complete cache index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndex {
    /// Cache format version
    pub version: u32,

    /// All cached entries, keyed by file URI
    pub entries: HashMap<String, CacheEntry>,

    /// Last update time
    pub last_updated: SystemTime,
}

impl Default for CacheIndex {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
            last_updated: SystemTime::now(),
        }
    }
}

impl CacheIndex {
    /// Create a new empty cache index
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update an entry in the cache
    pub fn insert(&mut self, uri: String, entry: CacheEntry) {
        self.entries.insert(uri, entry);
        self.last_updated = SystemTime::now();
    }

    /// Get a cached entry if it exists and is valid
    pub fn get(&self, uri: &str) -> Option<&CacheEntry> {
        self.entries.get(uri)
    }

    /// Check if a cache entry is still valid
    pub fn is_valid(&self, uri: &str, content_hash: u64) -> bool {
        if let Some(entry) = self.entries.get(uri) {
            entry.content_hash == content_hash
        } else {
            false
        }
    }

    /// Remove an entry from the cache
    pub fn remove(&mut self, uri: &str) -> Option<CacheEntry> {
        self.entries.remove(uri)
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_updated = SystemTime::now();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Manager for persistent cache operations
pub struct CacheManager {
    /// Path to the cache file
    cache_path: PathBuf,

    /// In-memory cache index
    index: CacheIndex,

    /// Whether the cache is enabled
    enabled: bool,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        let (cache_path, enabled) = if let Some(dir) = cache_dir {
            // Ensure cache directory exists
            if let Err(e) = fs::create_dir_all(&dir) {
                tracing::warn!("Failed to create cache directory: {}", e);
                (PathBuf::new(), false)
            } else {
                (dir.join("windjammer-lsp.cache"), true)
            }
        } else {
            // Use default cache location: ~/.cache/windjammer-lsp/
            let default_cache = Self::default_cache_path();
            if let Some(path) = default_cache {
                if let Some(parent) = path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        tracing::warn!("Failed to create default cache directory: {}", e);
                        (PathBuf::new(), false)
                    } else {
                        (path, true)
                    }
                } else {
                    (PathBuf::new(), false)
                }
            } else {
                (PathBuf::new(), false)
            }
        };

        let index = if enabled && cache_path.exists() {
            Self::load_from_disk(&cache_path).unwrap_or_default()
        } else {
            CacheIndex::new()
        };

        tracing::info!(
            "Cache manager initialized: enabled={}, path={:?}, entries={}",
            enabled,
            cache_path,
            index.len()
        );

        Self {
            cache_path,
            index,
            enabled,
        }
    }

    /// Get the default cache path
    fn default_cache_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(home) = std::env::var("HOME") {
                return Some(PathBuf::from(home).join(".cache/windjammer-lsp/cache.bin"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                return Some(PathBuf::from(home).join("Library/Caches/windjammer-lsp/cache.bin"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                return Some(PathBuf::from(local_app_data).join("windjammer-lsp\\cache.bin"));
            }
        }

        None
    }

    /// Load cache from disk
    fn load_from_disk(path: &Path) -> Result<CacheIndex, Box<dyn std::error::Error>> {
        let data = fs::read(path)?;
        let index: CacheIndex = bincode::deserialize(&data)?;

        // Check version compatibility
        if index.version != CACHE_VERSION {
            tracing::warn!(
                "Cache version mismatch: expected {}, got {}. Cache will be rebuilt.",
                CACHE_VERSION,
                index.version
            );
            return Ok(CacheIndex::new());
        }

        Ok(index)
    }

    /// Save cache to disk
    pub fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Ok(());
        }

        let data = bincode::serialize(&self.index)?;
        fs::write(&self.cache_path, data)?;

        tracing::debug!(
            "Saved cache with {} entries to {:?}",
            self.index.len(),
            self.cache_path
        );
        Ok(())
    }

    /// Get a cache entry
    pub fn get(&self, uri: &Url) -> Option<&CacheEntry> {
        if !self.enabled {
            return None;
        }
        self.index.get(uri.as_str())
    }

    /// Check if a cache entry is valid
    pub fn is_valid(&self, uri: &Url, content_hash: u64) -> bool {
        if !self.enabled {
            return false;
        }
        self.index.is_valid(uri.as_str(), content_hash)
    }

    /// Insert or update a cache entry
    pub fn insert(&mut self, uri: Url, entry: CacheEntry) {
        if !self.enabled {
            return;
        }
        self.index.insert(uri.to_string(), entry);
    }

    /// Remove a cache entry
    pub fn remove(&mut self, uri: &Url) -> Option<CacheEntry> {
        if !self.enabled {
            return None;
        }
        self.index.remove(uri.as_str())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.index.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            enabled: self.enabled,
            entries: self.index.len(),
            last_updated: self.index.last_updated,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub enabled: bool,
    pub entries: usize,
    pub last_updated: SystemTime,
}

/// Calculate a simple hash for file content
pub fn calculate_content_hash(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_index_new() {
        let index = CacheIndex::new();
        assert_eq!(index.version, CACHE_VERSION);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_cache_index_insert_get() {
        let mut index = CacheIndex::new();
        let uri = "file:///test.wj".to_string();

        let entry = CacheEntry {
            uri: uri.clone(),
            content_hash: 12345,
            modified_time: SystemTime::now(),
            symbols: vec![],
            imports: vec![],
        };

        index.insert(uri.clone(), entry);
        assert_eq!(index.len(), 1);

        let retrieved = index.get(&uri);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content_hash, 12345);
    }

    #[test]
    fn test_cache_index_is_valid() {
        let mut index = CacheIndex::new();
        let uri = "file:///test.wj".to_string();

        let entry = CacheEntry {
            uri: uri.clone(),
            content_hash: 12345,
            modified_time: SystemTime::now(),
            symbols: vec![],
            imports: vec![],
        };

        index.insert(uri.clone(), entry);

        assert!(index.is_valid(&uri, 12345)); // Same hash
        assert!(!index.is_valid(&uri, 54321)); // Different hash
        assert!(!index.is_valid("file:///other.wj", 12345)); // Non-existent
    }

    #[test]
    fn test_cache_index_remove() {
        let mut index = CacheIndex::new();
        let uri = "file:///test.wj".to_string();

        let entry = CacheEntry {
            uri: uri.clone(),
            content_hash: 12345,
            modified_time: SystemTime::now(),
            symbols: vec![],
            imports: vec![],
        };

        index.insert(uri.clone(), entry);
        assert_eq!(index.len(), 1);

        let removed = index.remove(&uri);
        assert!(removed.is_some());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_cache_index_clear() {
        let mut index = CacheIndex::new();

        for i in 0..5 {
            let uri = format!("file:///test{}.wj", i);
            let entry = CacheEntry {
                uri: uri.clone(),
                content_hash: i as u64,
                modified_time: SystemTime::now(),
                symbols: vec![],
                imports: vec![],
            };
            index.insert(uri, entry);
        }

        assert_eq!(index.len(), 5);
        index.clear();
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_calculate_content_hash() {
        let content1 = "fn test() {}";
        let content2 = "fn test() {}";
        let content3 = "fn other() {}";

        let hash1 = calculate_content_hash(content1);
        let hash2 = calculate_content_hash(content2);
        let hash3 = calculate_content_hash(content3);

        assert_eq!(hash1, hash2); // Same content = same hash
        assert_ne!(hash1, hash3); // Different content = different hash
    }

    #[test]
    fn test_cache_manager_disabled() {
        let manager = CacheManager::new(None);
        // Without a valid cache directory, it should work but be disabled
        // or have default cache enabled
        assert!(manager.index.len() == 0);
    }

    #[test]
    fn test_cached_symbol() {
        let symbol = CachedSymbol {
            name: "test_function".to_string(),
            kind: "Function".to_string(),
            line: 10,
            character: 5,
            type_info: Some("int".to_string()),
        };

        // Test serialization
        let serialized = bincode::serialize(&symbol).unwrap();
        let deserialized: CachedSymbol = bincode::deserialize(&serialized).unwrap();

        assert_eq!(symbol.name, deserialized.name);
        assert_eq!(symbol.kind, deserialized.kind);
        assert_eq!(symbol.line, deserialized.line);
        assert_eq!(symbol.type_info, deserialized.type_info);
    }

    #[test]
    fn test_cache_entry_serialization() {
        let entry = CacheEntry {
            uri: "file:///test.wj".to_string(),
            content_hash: 98765,
            modified_time: SystemTime::now(),
            symbols: vec![
                CachedSymbol {
                    name: "func1".to_string(),
                    kind: "Function".to_string(),
                    line: 1,
                    character: 0,
                    type_info: None,
                },
                CachedSymbol {
                    name: "func2".to_string(),
                    kind: "Function".to_string(),
                    line: 5,
                    character: 0,
                    type_info: Some("string".to_string()),
                },
            ],
            imports: vec!["std.fs".to_string(), "std.http".to_string()],
        };

        // Test serialization round-trip
        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: CacheEntry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(entry.uri, deserialized.uri);
        assert_eq!(entry.content_hash, deserialized.content_hash);
        assert_eq!(entry.symbols.len(), deserialized.symbols.len());
        assert_eq!(entry.imports.len(), deserialized.imports.len());
    }

    #[test]
    fn test_cache_index_serialization() {
        let mut index = CacheIndex::new();

        for i in 0..3 {
            let uri = format!("file:///test{}.wj", i);
            let entry = CacheEntry {
                uri: uri.clone(),
                content_hash: i as u64,
                modified_time: SystemTime::now(),
                symbols: vec![],
                imports: vec![],
            };
            index.insert(uri, entry);
        }

        // Test serialization round-trip
        let serialized = bincode::serialize(&index).unwrap();
        let deserialized: CacheIndex = bincode::deserialize(&serialized).unwrap();

        assert_eq!(index.version, deserialized.version);
        assert_eq!(index.entries.len(), deserialized.entries.len());
    }
}
