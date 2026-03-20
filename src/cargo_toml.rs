//! Cargo.toml generation for single-file and multi-file Windjammer builds.
//!
//! TDD FIX (Bug #2): Detect test files and generate [[bin]]/[[test]] targets.
//! Used by compiler for single-file builds (wj CLI uses this path).

use crate::CompilationTarget;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// File type for Cargo target generation
#[derive(Debug, PartialEq)]
enum RustFileType {
    Test,    // Contains #[test] functions
    Binary, // Contains fn main()
    Library, // Neither (just library code)
}

/// Detect what type of Rust file this is by scanning its contents
fn detect_rust_file_type(path: &Path) -> RustFileType {
    if let Ok(contents) = fs::read_to_string(path) {
        let has_main = contents.contains("fn main()") || contents.contains("fn main(");
        let has_test = contents.contains("#[test]");

        if has_main {
            RustFileType::Binary
        } else if has_test {
            RustFileType::Test
        } else {
            RustFileType::Library
        }
    } else {
        RustFileType::Library
    }
}

/// Find windjammer-runtime path for Cargo.toml dependency
fn find_windjammer_runtime_path() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Try current directory first (if we're in windjammer repo)
    if current.join("crates/windjammer-runtime/Cargo.toml").exists() {
        return current.join("crates/windjammer-runtime");
    }

    // Search upward (up to 5 levels)
    for _ in 0..5 {
        if let Some(parent) = current.parent() {
            if parent.join("windjammer/crates/windjammer-runtime/Cargo.toml").exists() {
                return parent.join("windjammer/crates/windjammer-runtime");
            }
            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                return parent.join("crates/windjammer-runtime");
            }
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback: relative paths that might work
    let sibling = PathBuf::from("../windjammer/crates/windjammer-runtime");
    if sibling.join("Cargo.toml").exists() {
        return sibling;
    }
    PathBuf::from("./crates/windjammer-runtime")
}

/// Convert path to string suitable for Cargo.toml (absolute for reliability)
fn path_to_toml_string(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

/// Generate Cargo.toml for single-file builds.
/// Called by compiler::build_project_ext when target is Rust.
pub fn generate_single_file_cargo_toml(
    output_dir: &Path,
    source_dir: &Path,
    target: CompilationTarget,
) -> Result<()> {
    if target != CompilationTarget::Rust {
        return Ok(());
    }

    let has_lib_rs = output_dir.join("lib.rs").exists();
    let has_main_rs = output_dir.join("main.rs").exists();
    
    let project_name = infer_project_name(source_dir);
    let lib_name = project_name.replace('-', "_");  // Rust lib names can't have hyphens

    let lib_or_bin_section = if has_lib_rs {
        format!(
            "[lib]\nname = \"{}\"\npath = \"lib.rs\"\n\n",
            lib_name
        )
    } else if has_main_rs {
        format!("[[bin]]\nname = \"{}\"\npath = \"main.rs\"\n\n", project_name)
    } else {
        // TDD FIX (Bug #2): Detect file type and generate [[bin]] or [[test]]
        let mut target_sections = Vec::new();
        let mut has_library_only = false;

        if let Ok(entries) = fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".rs") && filename != "lib.rs" {
                        let file_path = entry.path();
                        let file_type = detect_rust_file_type(&file_path);
                        let target_name = filename.strip_suffix(".rs").unwrap_or(filename);

                        match file_type {
                            RustFileType::Test => {
                                target_sections.push(format!(
                                    "[[test]]\nname = \"{}\"\npath = \"{}\"\n",
                                    target_name, filename
                                ));
                            }
                            RustFileType::Binary => {
                                target_sections.push(format!(
                                    "[[bin]]\nname = \"{}\"\npath = \"{}\"\n",
                                    target_name, filename
                                ));
                            }
                            RustFileType::Library => {
                                has_library_only = true;
                            }
                        }
                    }
                }
            }
        }

        if !target_sections.is_empty() {
            format!("{}\n", target_sections.join("\n"))
        } else if has_library_only {
            // Library-only: use first .rs file as [lib], no [[bin]] or [[test]]
            if let Ok(entries) = fs::read_dir(output_dir) {
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.ends_with(".rs") {
                            let target_name = filename.strip_suffix(".rs").unwrap_or(filename);
                            let lib_section = format!(
                                "[lib]\nname = \"{}\"\npath = \"{}\"\n\n",
                                target_name.replace('-', "_"),
                                filename
                            );
                            return write_cargo_toml(output_dir, source_dir, &lib_section);
                        }
                    }
                }
            }
            String::new()
        } else {
            String::new()
        }
    };

    if lib_or_bin_section.is_empty() && !has_lib_rs && !has_main_rs {
        // No .rs files found - skip Cargo.toml
        return Ok(());
    }

    write_cargo_toml(output_dir, source_dir, &lib_or_bin_section)
}

fn write_cargo_toml(
    output_dir: &Path,
    source_dir: &Path,
    lib_or_bin_section: &str,
) -> Result<()> {
    let runtime_path = find_windjammer_runtime_path();
    let runtime_path_str = path_to_toml_string(&runtime_path);

    let deps_section = format!(
        "[dependencies]\nwindjammer-runtime = {{ path = \"{}\" }}\nsmallvec = \"1.13\"\nserde = {{ version = \"1.0\", features = [\"derive\"] }}\n\n",
        runtime_path_str
    );

    let project_name = infer_project_name(source_dir);
    let package_name = project_name.replace('-', "_");  // Package name for consistency

    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

# Prevent this from being treated as part of parent workspace
[workspace]

{}{}[profile.release]
opt-level = 3
"#,
        package_name, deps_section, lib_or_bin_section
    );

    let cargo_toml_path = output_dir.join("Cargo.toml");
    fs::write(cargo_toml_path, cargo_toml)?;

    Ok(())
}

fn infer_project_name(source_dir: &Path) -> String {
    // Check game.toml
    let game_toml = source_dir.join("game.toml");
    if game_toml.exists() {
        if let Ok(content) = fs::read_to_string(&game_toml) {
            if let Some(name) = content
                .lines()
                .find(|l| l.trim().starts_with("name"))
                .and_then(|l| l.split('"').nth(1))
                .map(|s| s.to_lowercase().replace(' ', "-"))
            {
                return name;
            }
        }
    }

    // Check parent for game.toml
    if let Some(parent) = source_dir.parent() {
        let parent_game = parent.join("game.toml");
        if parent_game.exists() {
            if let Ok(content) = fs::read_to_string(&parent_game) {
                if let Some(name) = content
                    .lines()
                    .find(|l| l.trim().starts_with("name"))
                    .and_then(|l| l.split('"').nth(1))
                    .map(|s| s.to_lowercase().replace(' ', "-"))
                {
                    return name;
                }
            }
        }
    }

    "windjammer".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_cargo_toml_for_binary() {
        let temp = std::env::temp_dir().join("wj_cargo_test");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(
            temp.join("hello.rs"),
            "fn main() { println!(\"hi\"); }",
        )
        .unwrap();

        let result = generate_single_file_cargo_toml(&temp, &temp, CompilationTarget::Rust);
        assert!(result.is_ok(), "generate_single_file_cargo_toml failed: {:?}", result.err());

        let cargo_toml = fs::read_to_string(temp.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("[[bin]]"), "Should have [[bin]]: {}", cargo_toml);
        assert!(cargo_toml.contains("name = \"hello\""), "Should have name: {}", cargo_toml);

        fs::remove_dir_all(&temp).ok();
    }
}
