//! Console output helpers for the legacy `windjammer` binary.

use crate::error_mapper;
use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::source_map;

/// TDD FIX (Bug #2): File type detection for Cargo.toml target generation
#[derive(Debug, PartialEq)]
pub enum RustFileType {
    Test,    // Contains #[test] functions
    Binary,  // Contains fn main()
    Library, // Neither (just library code)
}

/// Detect what type of Rust file this is by scanning its contents
pub fn detect_rust_file_type(path: &Path) -> RustFileType {
    if let Ok(contents) = std::fs::read_to_string(path) {
        let has_main = contents.contains("fn main()") || contents.contains("fn main(");
        let has_test = contents.contains("#[test]");

        // Priority: main() takes precedence (binaries can have tests)
        // Files with ONLY tests (no main) are test targets
        // Files with neither are library modules (no target needed)
        if has_main {
            RustFileType::Binary
        } else if has_test {
            RustFileType::Test
        } else {
            RustFileType::Library
        }
    } else {
        // Can't read file - assume library
        RustFileType::Library
    }
}

/// Load and merge all source maps from the output directory
pub fn load_source_maps(output_dir: &Path) -> Result<source_map::SourceMap> {
    let mut merged_map = source_map::SourceMap::new();
    let mut map_count = 0;
    let mut mapping_count = 0;

    // Find all .rs.map files in the output directory
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("map") {
                // Check if this is a .rs.map file (not just any .map file)
                if let Some(stem) = path.file_stem() {
                    if let Some(stem_str) = stem.to_str() {
                        if !stem_str.ends_with(".rs") {
                            continue;
                        }
                    }
                }

                // Load this source map
                if let Ok(map) = source_map::SourceMap::load_from_file(&path) {
                    // Get the corresponding .rs file path
                    let rust_file = path.with_extension("").with_extension("rs");

                    // Merge all mappings from this source map
                    let mappings = map.mappings_for_rust_file(&rust_file);
                    for mapping in mappings {
                        merged_map.add_mapping(
                            &mapping.rust_file,
                            mapping.rust_line,
                            mapping.rust_column,
                            &mapping.wj_file,
                            mapping.wj_line,
                            mapping.wj_column,
                        );
                        mapping_count += 1;
                    }
                    map_count += 1;
                }
            }
        }
    }

    if map_count == 0 {
        eprintln!(
            "{} No source maps found in {}. Errors will reference Rust code.",
            "Warning:".yellow().bold(),
            output_dir.display()
        );
    } else {
        eprintln!(
            "{} Loaded {} source map{} with {} mapping{}",
            "Info:".cyan(),
            map_count,
            if map_count == 1 { "" } else { "s" },
            mapping_count,
            if mapping_count == 1 { "" } else { "s" }
        );
    }

    Ok(merged_map)
}

/// Colorize diagnostic output based on level
pub fn colorize_diagnostic(text: &str, _level: &error_mapper::DiagnosticLevel) -> String {
    let mut result = String::new();
    for line in text.lines() {
        if line.starts_with("error") {
            result.push_str(&line.red().bold().to_string());
        } else if line.starts_with("warning") {
            result.push_str(&line.yellow().bold().to_string());
        } else if line.contains("-->") {
            result.push_str(&line.blue().bold().to_string());
        } else if line.starts_with("  = help:") {
            result.push_str(&line.cyan().to_string());
        } else if line.starts_with("  = suggestion:") {
            result.push_str(&line.green().bold().to_string());
        } else if line.starts_with("  = note:") {
            result.push_str(&line.white().dimmed().to_string());
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}
