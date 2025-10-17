//! Source map generation for JavaScript output
//!
//! Generates source maps that map generated JavaScript code back to original
//! Windjammer source files for debugging.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source map output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceMapMode {
    /// Generate external .map file
    #[default]
    External,
    /// Inline source map as base64 data URL
    Inline,
    /// Both external and inline
    Both,
}

/// Source map format (v3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
    /// Source map version (always 3)
    pub version: u32,
    /// Output file name
    pub file: String,
    /// List of source file names
    pub sources: Vec<String>,
    /// Optional source content (for embedded sources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources_content: Option<Vec<String>>,
    /// List of symbol names
    pub names: Vec<String>,
    /// Base64 VLQ encoded mappings
    pub mappings: String,
}

impl Default for SourceMap {
    fn default() -> Self {
        Self {
            version: 3,
            file: String::new(),
            sources: Vec::new(),
            sources_content: None,
            names: Vec::new(),
            mappings: String::new(),
        }
    }
}

/// Represents a single mapping from generated to source position
#[derive(Debug, Clone, Copy)]
pub struct Mapping {
    /// Generated line number (0-based)
    pub generated_line: usize,
    /// Generated column number (0-based)
    pub generated_column: usize,
    /// Source file index
    pub source_index: usize,
    /// Original line number (0-based)
    pub original_line: usize,
    /// Original column number (0-based)
    pub original_column: usize,
    /// Name index (optional)
    pub name_index: Option<usize>,
}

/// Source map builder
pub struct SourceMapBuilder {
    file: String,
    sources: Vec<String>,
    sources_content: Option<Vec<String>>,
    names: Vec<String>,
    name_indices: HashMap<String, usize>,
    mappings: Vec<Mapping>,
}

impl SourceMapBuilder {
    /// Create a new source map builder
    pub fn new(output_file: String) -> Self {
        Self {
            file: output_file,
            sources: Vec::new(),
            sources_content: None,
            names: Vec::new(),
            name_indices: HashMap::new(),
            mappings: Vec::new(),
        }
    }

    /// Add a source file
    pub fn add_source(&mut self, source_path: String) -> usize {
        let index = self.sources.len();
        self.sources.push(source_path);
        index
    }

    /// Add source content (for embedded sources)
    pub fn add_source_content(&mut self, content: String) {
        if self.sources_content.is_none() {
            self.sources_content = Some(Vec::new());
        }
        if let Some(ref mut contents) = self.sources_content {
            contents.push(content);
        }
    }

    /// Add a name (for symbol mapping)
    pub fn add_name(&mut self, name: String) -> usize {
        if let Some(&index) = self.name_indices.get(&name) {
            return index;
        }
        let index = self.names.len();
        self.name_indices.insert(name.clone(), index);
        self.names.push(name);
        index
    }

    /// Add a mapping
    pub fn add_mapping(&mut self, mapping: Mapping) {
        self.mappings.push(mapping);
    }

    /// Build the source map
    pub fn build(mut self) -> SourceMap {
        // Sort mappings by generated position
        self.mappings.sort_by(|a, b| {
            a.generated_line
                .cmp(&b.generated_line)
                .then(a.generated_column.cmp(&b.generated_column))
        });

        let mappings_string = self.encode_mappings();

        SourceMap {
            version: 3,
            file: self.file,
            sources: self.sources,
            sources_content: self.sources_content,
            names: self.names,
            mappings: mappings_string,
        }
    }

    /// Encode mappings to Base64 VLQ format
    fn encode_mappings(&self) -> String {
        let mut result = String::new();
        let mut prev_generated_line = 0;
        let mut prev_generated_column = 0;
        let mut prev_source_index = 0;
        let mut prev_original_line = 0;
        let mut prev_original_column = 0;
        let mut prev_name_index = 0;

        for mapping in &self.mappings {
            // Handle line breaks
            while prev_generated_line < mapping.generated_line {
                result.push(';');
                prev_generated_line += 1;
                prev_generated_column = 0;
            }

            if !result.is_empty() && !result.ends_with(';') {
                result.push(',');
            }

            // Generated column (relative)
            let generated_column_delta =
                mapping.generated_column as i32 - prev_generated_column as i32;
            result.push_str(&encode_vlq(generated_column_delta));

            // Source file index (relative)
            let source_index_delta = mapping.source_index as i32 - prev_source_index as i32;
            result.push_str(&encode_vlq(source_index_delta));

            // Original line (relative)
            let original_line_delta = mapping.original_line as i32 - prev_original_line as i32;
            result.push_str(&encode_vlq(original_line_delta));

            // Original column (relative)
            let original_column_delta =
                mapping.original_column as i32 - prev_original_column as i32;
            result.push_str(&encode_vlq(original_column_delta));

            // Name index (optional, relative)
            if let Some(name_index) = mapping.name_index {
                let name_index_delta = name_index as i32 - prev_name_index as i32;
                result.push_str(&encode_vlq(name_index_delta));
                prev_name_index = name_index;
            }

            // Update previous values
            prev_generated_column = mapping.generated_column;
            prev_source_index = mapping.source_index;
            prev_original_line = mapping.original_line;
            prev_original_column = mapping.original_column;
        }

        result
    }
}

/// Base64 VLQ encoding for source maps
fn encode_vlq(value: i32) -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut vlq = if value < 0 {
        ((-value) << 1) | 1
    } else {
        value << 1
    };

    loop {
        let mut digit = vlq & 0b11111;
        vlq >>= 5;

        if vlq > 0 {
            digit |= 0b100000; // Continuation bit
        }

        result.push(BASE64_CHARS[digit as usize] as char);

        if vlq == 0 {
            break;
        }
    }

    result
}

/// Generate a source map for the given JavaScript output
pub fn generate_source_map(
    output_file: &str,
    source_file: &str,
    _source_content: &str,
    _generated_code: &str,
) -> SourceMap {
    let mut builder = SourceMapBuilder::new(output_file.to_string());
    let source_index = builder.add_source(source_file.to_string());

    // TODO: Parse AST positions and generate accurate mappings
    // For now, create a basic 1:1 line mapping
    for line in 0..100 {
        builder.add_mapping(Mapping {
            generated_line: line,
            generated_column: 0,
            source_index,
            original_line: line,
            original_column: 0,
            name_index: None,
        });
    }

    builder.build()
}

/// Convert source map to inline base64 data URL
pub fn source_map_to_inline(source_map: &SourceMap) -> String {
    let json = serde_json::to_string(source_map).unwrap_or_default();
    let base64 = base64_encode(json.as_bytes());
    format!(
        "//# sourceMappingURL=data:application/json;charset=utf-8;base64,{}",
        base64
    )
}

/// Generate external source map reference comment
pub fn source_map_external_reference(map_filename: &str) -> String {
    format!("//# sourceMappingURL={}", map_filename)
}

/// Apply source map to JavaScript code based on mode
pub fn apply_source_map(
    code: &str,
    source_map: &SourceMap,
    mode: SourceMapMode,
    map_filename: &str,
) -> String {
    let mut result = code.to_string();

    match mode {
        SourceMapMode::External => {
            result.push('\n');
            result.push_str(&source_map_external_reference(map_filename));
            result.push('\n');
        }
        SourceMapMode::Inline => {
            result.push('\n');
            result.push_str(&source_map_to_inline(source_map));
            result.push('\n');
        }
        SourceMapMode::Both => {
            result.push('\n');
            result.push_str(&source_map_external_reference(map_filename));
            result.push('\n');
            result.push_str(&source_map_to_inline(source_map));
            result.push('\n');
        }
    }

    result
}

/// Simple base64 encoding (sufficient for source maps)
fn base64_encode(data: &[u8]) -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }

        result.push(BASE64_CHARS[(buf[0] >> 2) as usize] as char);
        result.push(BASE64_CHARS[(((buf[0] & 0x03) << 4) | (buf[1] >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(BASE64_CHARS[(((buf[1] & 0x0f) << 2) | (buf[2] >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(buf[2] & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_vlq() {
        assert_eq!(encode_vlq(0), "A");
        assert_eq!(encode_vlq(1), "C");
        assert_eq!(encode_vlq(-1), "D");
        assert_eq!(encode_vlq(15), "e");
        assert_eq!(encode_vlq(16), "gB");
    }

    #[test]
    fn test_source_map_builder() {
        let mut builder = SourceMapBuilder::new("output.js".to_string());
        let source_idx = builder.add_source("input.wj".to_string());

        builder.add_mapping(Mapping {
            generated_line: 0,
            generated_column: 0,
            source_index: source_idx,
            original_line: 0,
            original_column: 0,
            name_index: None,
        });

        let source_map = builder.build();
        assert_eq!(source_map.version, 3);
        assert_eq!(source_map.file, "output.js");
        assert_eq!(source_map.sources.len(), 1);
        assert!(!source_map.mappings.is_empty());
    }

    #[test]
    fn test_generate_source_map() {
        let source_map = generate_source_map(
            "output.js",
            "input.wj",
            "fn main() { println!(\"hello\"); }",
            "function main() { console.log(\"hello\"); }",
        );

        assert_eq!(source_map.file, "output.js");
        assert_eq!(source_map.sources[0], "input.wj");
        assert!(!source_map.mappings.is_empty());
    }
}
