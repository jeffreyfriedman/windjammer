//! Cargo.toml generation for single-file and multi-file Windjammer builds.
//!
//! TDD FIX (Bug #2): Detect test files and generate [[bin]]/[[test]] targets.
//! Used by compiler for single-file builds (wj CLI uses this path).

mod dependency_management;
mod feature_management;
mod toml_generation;

use crate::CompilationTarget;
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use toml_generation::{infer_project_name, write_cargo_toml};

/// When true, `generate_single_file_cargo_toml` is a no-op.
/// Set via `--no-generate-cargo-toml` CLI flag for projects that maintain
/// their own Cargo.toml.
static SKIP_CARGO_TOML_GENERATION: AtomicBool = AtomicBool::new(false);

pub fn set_skip_cargo_toml_generation(skip: bool) {
    SKIP_CARGO_TOML_GENERATION.store(skip, Ordering::Relaxed);
}

/// File type for Cargo target generation
#[derive(Debug, PartialEq)]
enum RustFileType {
    Test,    // Contains #[test] functions
    Binary,  // Contains fn main()
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
    if SKIP_CARGO_TOML_GENERATION.load(Ordering::Relaxed) {
        return Ok(());
    }

    let has_lib_rs = output_dir.join("lib.rs").exists();
    let has_main_rs = output_dir.join("main.rs").exists();
    let has_mod_rs = output_dir.join("mod.rs").exists();

    let project_name = infer_project_name(source_dir);
    let lib_name = project_name.replace('-', "_"); // Rust lib names can't have hyphens

    let lib_or_bin_section = if has_lib_rs {
        format!("[lib]\nname = \"{}\"\npath = \"lib.rs\"\n\n", lib_name)
    } else if has_mod_rs {
        format!("[lib]\nname = \"{}\"\npath = \"mod.rs\"\n\n", lib_name)
    } else if has_main_rs {
        format!(
            "[[bin]]\nname = \"{}\"\npath = \"main.rs\"\n\n",
            project_name
        )
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

pub use toml_generation::generate_wasm_cargo_toml;
pub use toml_generation::infer_project_name_from;

#[cfg(test)]
mod tests {
    use super::toml_generation::{infer_project_name, resolve_package_name_with_existing_cargo};
    use super::*;
    use crate::CompilationTarget;
    use std::fs;

    #[test]
    fn test_generate_cargo_toml_for_binary() {
        let temp = std::env::temp_dir().join("wj_cargo_test");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("hello.rs"), "fn main() { println!(\"hi\"); }").unwrap();

        let result = generate_single_file_cargo_toml(&temp, &temp, CompilationTarget::Rust);
        assert!(
            result.is_ok(),
            "generate_single_file_cargo_toml failed: {:?}",
            result.err()
        );

        let cargo_toml = fs::read_to_string(temp.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("[[bin]]"),
            "Should have [[bin]]: {}",
            cargo_toml
        );
        assert!(
            cargo_toml.contains("name = \"hello\""),
            "Should have name: {}",
            cargo_toml
        );

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_infer_project_name_defaults_to_dir_name_without_config() {
        let temp = std::env::temp_dir().join("wj_infer_default");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        // Falls back to directory name, not "windjammer"
        assert_eq!(infer_project_name(&temp), "wj_infer_default");

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_infer_project_name_from_wj_toml() {
        let temp = std::env::temp_dir().join("wj_infer_wjtoml");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(
            temp.join("wj.toml"),
            "[package]\nname = \"my-cool-engine\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        assert_eq!(infer_project_name(&temp), "my-cool-engine");

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_infer_project_name_wj_toml_takes_precedence_over_game_toml() {
        let temp = std::env::temp_dir().join("wj_infer_precedence");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(
            temp.join("wj.toml"),
            "[package]\nname = \"from-wj\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(temp.join("game.toml"), "name = \"from-game\"\n").unwrap();

        assert_eq!(infer_project_name(&temp), "from-wj");

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_infer_project_name_src_dir_uses_parent() {
        let temp = std::env::temp_dir().join("wj_infer_src_parent");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("src")).unwrap();

        // When source_dir is "src", use parent directory name
        assert_eq!(infer_project_name(&temp.join("src")), "wj_infer_src_parent");

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_infer_project_name_from_game_toml_in_source_dir() {
        let temp = std::env::temp_dir().join("wj_infer_game");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("game.toml"), r#"name = "My Game""#).unwrap();

        assert_eq!(infer_project_name(&temp), "my-game");

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_infer_project_name_from_parent_game_toml() {
        let temp = std::env::temp_dir().join("wj_infer_parent_game");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("src")).unwrap();
        fs::write(temp.join("game.toml"), r#"name = "My Game Title""#).unwrap();

        assert_eq!(infer_project_name(&temp.join("src")), "my-game-title");

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_resolve_package_name_preserves_existing_non_windjammer() {
        let temp = std::env::temp_dir().join("wj_pkg_preserve");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(
            temp.join("Cargo.toml"),
            r#"[package]
name = "my_project"
version = "0.1.0"
"#,
        )
        .unwrap();

        assert_eq!(
            resolve_package_name_with_existing_cargo(&temp, "windjammer"),
            "my_project"
        );

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_resolve_package_name_ignores_stale_windjammer_for_inferred() {
        let temp = std::env::temp_dir().join("wj_pkg_stale");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(
            temp.join("Cargo.toml"),
            r#"[package]
name = "windjammer"
version = "0.1.0"
"#,
        )
        .unwrap();

        assert_eq!(
            resolve_package_name_with_existing_cargo(&temp, "my_project"),
            "my_project"
        );

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_resolve_package_name_no_existing_file_uses_inferred() {
        let temp = std::env::temp_dir().join("wj_pkg_no_file");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        assert_eq!(
            resolve_package_name_with_existing_cargo(&temp, "windjammer"),
            "windjammer"
        );

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_write_cargo_toml_preserves_existing_package_name() {
        let out = std::env::temp_dir().join("wj_write_preserve");
        let src = std::env::temp_dir().join("wj_write_preserve_src");
        let _ = fs::remove_dir_all(&out);
        let _ = fs::remove_dir_all(&src);
        fs::create_dir_all(&out).unwrap();
        fs::create_dir_all(&src).unwrap();

        fs::write(
            out.join("Cargo.toml"),
            r#"# Auto-generated
[package]
name = "my_project"
version = "0.1.0"
edition = "2021"

[workspace]
"#,
        )
        .unwrap();
        fs::write(out.join("lib.rs"), "// lib").unwrap();

        let result = generate_single_file_cargo_toml(&out, &src, CompilationTarget::Rust);
        assert!(result.is_ok(), "{:?}", result.err());

        let cargo = fs::read_to_string(out.join("Cargo.toml")).unwrap();
        assert!(
            cargo.contains("name = \"my_project\""),
            "package name should be preserved: {}",
            cargo
        );

        fs::remove_dir_all(&out).ok();
        fs::remove_dir_all(&src).ok();
    }
}
