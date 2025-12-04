// Source Map - Map generated Rust code back to Windjammer source
//
// This module provides source map generation for world-class error messages.
// When Rust compiler errors occur, we use the source map to translate them
// back to the original Windjammer source location.
//
// Reference: docs/ERROR_MAPPING.md, docs/design/error-mapping.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Location in a Windjammer source file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Location {
    /// Path to the Windjammer file
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)  
    pub column: usize,
}

/// A mapping from a location in generated Rust code to original Windjammer source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Mapping {
    /// Path to the generated Rust file
    pub rust_file: PathBuf,
    /// Line number in the generated Rust file (1-indexed)
    pub rust_line: usize,
    /// Column number in the generated Rust file (1-indexed)
    pub rust_column: usize,
    /// Path to the original Windjammer file
    pub wj_file: PathBuf,
    /// Line number in the original Windjammer file (1-indexed)
    pub wj_line: usize,
    /// Column number in the original Windjammer file (1-indexed)
    pub wj_column: usize,
}

/// Source map that tracks all mappings from Rust to Windjammer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
    /// All mappings stored as a vector for JSON serialization
    #[serde(rename = "mappings")]
    mappings_vec: Vec<Mapping>,
    /// Version of the source map format
    version: u32,
    /// Workspace root path for relative path calculation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_root: Option<PathBuf>,
    /// Internal index for fast lookup (not serialized)
    #[serde(skip)]
    lookup_index: HashMap<(PathBuf, usize), usize>, // Maps (rust_file, rust_line) -> index in mappings_vec
}

impl SourceMap {
    /// Create a new empty source map
    pub fn new() -> Self {
        Self {
            mappings_vec: Vec::new(),
            version: 1,
            workspace_root: None,
            lookup_index: HashMap::new(),
        }
    }

    /// Set the workspace root for relative path calculation
    pub fn set_workspace_root(&mut self, root: impl Into<PathBuf>) {
        self.workspace_root = Some(root.into());
    }

    /// Convert an absolute path to a path relative to the workspace root
    fn to_relative_path(&self, path: &Path) -> PathBuf {
        if let Some(ref root) = self.workspace_root {
            // Try to make the path relative to the workspace root
            if let Ok(relative) = path.strip_prefix(root) {
                return relative.to_path_buf();
            }
        }
        // Return original path if not under workspace root or no root set
        path.to_path_buf()
    }

    /// Rebuild the lookup index from the mappings vector
    fn rebuild_index(&mut self) {
        self.lookup_index.clear();
        for (idx, mapping) in self.mappings_vec.iter().enumerate() {
            self.lookup_index
                .insert((mapping.rust_file.clone(), mapping.rust_line), idx);
        }
    }

    /// Add a mapping from Rust location to Windjammer location
    /// If a workspace root is set, paths are automatically converted to relative paths
    pub fn add_mapping(
        &mut self,
        rust_file: impl Into<PathBuf>,
        rust_line: usize,
        rust_column: usize,
        wj_file: impl Into<PathBuf>,
        wj_line: usize,
        wj_column: usize,
    ) {
        let rust_file_abs = rust_file.into();
        let wj_file_abs = wj_file.into();

        // Convert to relative paths if workspace root is set
        let rust_file_rel = self.to_relative_path(&rust_file_abs);
        let wj_file_rel = self.to_relative_path(&wj_file_abs);

        let mapping = Mapping {
            rust_file: rust_file_rel.clone(),
            rust_line,
            rust_column,
            wj_file: wj_file_rel,
            wj_line,
            wj_column,
        };

        let key = (rust_file_rel, rust_line);
        if let Some(&idx) = self.lookup_index.get(&key) {
            // Update existing mapping
            self.mappings_vec[idx] = mapping;
        } else {
            // Add new mapping
            let idx = self.mappings_vec.len();
            self.mappings_vec.push(mapping);
            self.lookup_index.insert(key, idx);
        }
    }

    /// Look up the Windjammer location for a given Rust location
    pub fn lookup(&self, rust_file: &Path, rust_line: usize) -> Option<&Mapping> {
        let key = (rust_file.to_path_buf(), rust_line);
        self.lookup_index
            .get(&key)
            .and_then(|&idx| self.mappings_vec.get(idx))
    }

    /// Look up with fuzzy matching - finds the closest mapping if exact match not found
    pub fn lookup_fuzzy(&self, rust_file: &Path, rust_line: usize) -> Option<&Mapping> {
        // Try exact match first
        if let Some(mapping) = self.lookup(rust_file, rust_line) {
            return Some(mapping);
        }

        // Try nearby lines (within 5 lines)
        for offset in 1..=5 {
            // Try above
            if rust_line > offset {
                if let Some(mapping) = self.lookup(rust_file, rust_line - offset) {
                    return Some(mapping);
                }
            }

            // Try below
            if let Some(mapping) = self.lookup(rust_file, rust_line + offset) {
                return Some(mapping);
            }
        }

        None
    }

    /// Save the source map to a JSON file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a source map from a JSON file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let mut source_map: Self = serde_json::from_str(&json)?;
        // Rebuild the lookup index after deserialization
        source_map.rebuild_index();
        Ok(source_map)
    }

    /// Get the number of mappings in the source map
    pub fn len(&self) -> usize {
        self.mappings_vec.len()
    }

    /// Check if the source map is empty
    pub fn is_empty(&self) -> bool {
        self.mappings_vec.is_empty()
    }

    /// Get all mappings for a specific Windjammer file
    pub fn mappings_for_wj_file(&self, wj_file: &Path) -> Vec<&Mapping> {
        self.mappings_vec
            .iter()
            .filter(|m| m.wj_file == wj_file)
            .collect()
    }

    /// Get all mappings for a specific Rust file
    pub fn mappings_for_rust_file(&self, rust_file: &Path) -> Vec<&Mapping> {
        self.mappings_vec
            .iter()
            .filter(|m| m.rust_file == rust_file)
            .collect()
    }

    /// Get the Windjammer location for a given Rust line (backward compatibility)
    ///
    /// This is a convenience method that returns a Location struct instead of a Mapping.
    /// Used by error_mapper.rs for translating Rust compiler errors.
    pub fn get_location(&self, rust_line: usize) -> Option<Location> {
        // Try to find any mapping with this line number (assumes single file project for now)
        // TODO: Make this file-aware for multi-file projects
        self.mappings_vec
            .iter()
            .find(|m| m.rust_line == rust_line)
            .map(|m| Location {
                file: m.wj_file.clone(),
                line: m.wj_line,
                column: m.wj_column,
            })
    }

    /// Map a Rust location to a Windjammer location
    ///
    /// This is the primary method used by error_mapper.rs to translate
    /// Rust compiler errors back to Windjammer source locations.
    pub fn map_rust_to_windjammer(&self, rust_location: &Location) -> Option<Location> {
        // Try exact match first
        if let Some(mapping) = self.lookup(&rust_location.file, rust_location.line) {
            return Some(Location {
                file: mapping.wj_file.clone(),
                line: mapping.wj_line,
                column: mapping.wj_column,
            });
        }

        // Try fuzzy match (nearby lines)
        if let Some(mapping) = self.lookup_fuzzy(&rust_location.file, rust_location.line) {
            return Some(Location {
                file: mapping.wj_file.clone(),
                line: mapping.wj_line,
                column: mapping.wj_column,
            });
        }

        None
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_creation() {
        let source_map = SourceMap::new();
        assert_eq!(source_map.len(), 0);
        assert!(source_map.is_empty());
    }

    #[test]
    fn test_add_and_lookup_mapping() {
        let mut source_map = SourceMap::new();

        source_map.add_mapping("output/main.rs", 10, 5, "src/main.wj", 5, 1);

        let mapping = source_map.lookup(Path::new("output/main.rs"), 10);
        assert!(mapping.is_some());

        let mapping = mapping.unwrap();
        assert_eq!(mapping.rust_line, 10);
        assert_eq!(mapping.rust_column, 5);
        assert_eq!(mapping.wj_line, 5);
        assert_eq!(mapping.wj_column, 1);
        assert_eq!(mapping.wj_file, PathBuf::from("src/main.wj"));
    }

    #[test]
    fn test_fuzzy_lookup() {
        let mut source_map = SourceMap::new();

        source_map.add_mapping("output/main.rs", 10, 5, "src/main.wj", 5, 1);

        // Exact match
        assert!(source_map
            .lookup_fuzzy(Path::new("output/main.rs"), 10)
            .is_some());

        // Nearby lines should also match
        assert!(source_map
            .lookup_fuzzy(Path::new("output/main.rs"), 11)
            .is_some());
        assert!(source_map
            .lookup_fuzzy(Path::new("output/main.rs"), 9)
            .is_some());
        assert!(source_map
            .lookup_fuzzy(Path::new("output/main.rs"), 12)
            .is_some());
        assert!(source_map
            .lookup_fuzzy(Path::new("output/main.rs"), 8)
            .is_some());

        // Too far away should not match
        assert!(source_map
            .lookup_fuzzy(Path::new("output/main.rs"), 20)
            .is_none());
    }

    #[test]
    fn test_save_and_load() {
        let mut source_map = SourceMap::new();

        source_map.add_mapping("output/main.rs", 10, 5, "src/main.wj", 5, 1);

        source_map.add_mapping("output/lib.rs", 20, 10, "src/lib.wj", 15, 3);

        // Save to temp file
        let temp_file = std::env::temp_dir().join("test_source_map.json");
        source_map.save_to_file(&temp_file).unwrap();

        // Load back
        let loaded = SourceMap::load_from_file(&temp_file).unwrap();

        assert_eq!(loaded.len(), 2);
        assert!(loaded.lookup(Path::new("output/main.rs"), 10).is_some());
        assert!(loaded.lookup(Path::new("output/lib.rs"), 20).is_some());

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_mappings_for_file() {
        let mut source_map = SourceMap::new();

        source_map.add_mapping("output/main.rs", 10, 5, "src/main.wj", 5, 1);
        source_map.add_mapping("output/main.rs", 20, 10, "src/main.wj", 15, 3);
        source_map.add_mapping("output/lib.rs", 5, 1, "src/lib.wj", 3, 1);

        let main_mappings = source_map.mappings_for_rust_file(Path::new("output/main.rs"));
        assert_eq!(main_mappings.len(), 2);

        let wj_main_mappings = source_map.mappings_for_wj_file(Path::new("src/main.wj"));
        assert_eq!(wj_main_mappings.len(), 2);

        let lib_mappings = source_map.mappings_for_rust_file(Path::new("output/lib.rs"));
        assert_eq!(lib_mappings.len(), 1);
    }
}
