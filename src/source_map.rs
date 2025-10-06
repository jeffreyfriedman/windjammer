use std::collections::HashMap;
use std::path::PathBuf;

/// Tracks the mapping between generated Rust code and original Windjammer source
#[derive(Debug, Default)]
pub struct SourceMap {
    /// Map: (rust_line_number) -> (wj_file, wj_line)
    /// Rust line numbers are 1-indexed to match compiler output
    mappings: HashMap<usize, SourceLocation>,
    /// The current Windjammer file being compiled
    current_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            current_file: None,
        }
    }

    pub fn set_current_file(&mut self, file: PathBuf) {
        self.current_file = Some(file);
    }

    /// Record a mapping from generated Rust line to original Windjammer line
    pub fn add_mapping(&mut self, rust_line: usize, wj_line: usize) {
        if let Some(ref file) = self.current_file {
            self.mappings.insert(
                rust_line,
                SourceLocation {
                    file: file.clone(),
                    line: wj_line,
                },
            );
        }
    }

    /// Look up the Windjammer source location for a Rust line number
    pub fn get_location(&self, rust_line: usize) -> Option<&SourceLocation> {
        self.mappings.get(&rust_line)
    }

    /// Get all mappings (for debugging/export)
    pub fn all_mappings(&self) -> &HashMap<usize, SourceLocation> {
        &self.mappings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map() {
        let mut map = SourceMap::new();
        map.set_current_file(PathBuf::from("test.wj"));

        map.add_mapping(1, 1);
        map.add_mapping(5, 10);
        map.add_mapping(10, 20);

        assert_eq!(map.get_location(5).unwrap().line, 10);
        assert_eq!(map.get_location(10).unwrap().line, 20);
        assert!(map.get_location(99).is_none());
    }
}
