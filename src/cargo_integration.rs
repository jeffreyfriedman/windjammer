//! Cargo integration module
//!
//! This module handles all interaction with the Rust Cargo build system:
//! - Generating Cargo.toml files for Rust and WebAssembly targets
//! - Running cargo check for error validation
//! - Managing dependencies (stdlib, external crates)
//! - Handling cross-platform path formatting for Cargo

use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::test_runner;
use crate::{error_mapper, source_map, CompilationTarget};
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
    use colored::*;
    use std::fs;

    let mut merged_map = source_map::SourceMap::new();
    let mut map_count = 0;
    let mut mapping_count = 0;

    // Find all .rs.map files in the output directory
    if let Ok(entries) = fs::read_dir(output_dir) {
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
    use colored::*;

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

/// Lint a Windjammer project using the LSP diagnostics engine
#[allow(dead_code)]

pub fn create_cargo_toml_with_deps(
    output_dir: &Path,
    imported_modules: &HashSet<String>,
    external_crates: &[String],
    target: CompilationTarget,
    source_dir: &Path,
) -> Result<()> {
    use std::env;
    use std::fs;

    // THE WINDJAMMER WAY: Copy FFI dependencies from source project's Cargo.toml
    // This enables projects with FFI (like windjammer-game-core) to compile correctly
    let mut source_cargo_deps = Vec::new();

    // Search upward for Cargo.toml (similar to windjammer.toml search)
    let mut search_dir = source_dir;
    let mut source_cargo_toml = None;
    for _ in 0..5 {
        let candidate = search_dir.join("Cargo.toml");
        if candidate.exists() {
            source_cargo_toml = Some(candidate);
            break;
        }

        // Go up one directory
        if let Some(parent) = search_dir.parent() {
            search_dir = parent;
        } else {
            break;
        }
    }

    // If we found a Cargo.toml, extract dependencies
    if let Some(cargo_toml_path) = source_cargo_toml {
        let source_dir_for_paths = cargo_toml_path.parent().unwrap();

        if let Ok(content) = fs::read_to_string(&cargo_toml_path) {
            // Parse Cargo.toml to extract [dependencies] section
            let mut in_dependencies = false;
            for line in content.lines() {
                let trimmed = line.trim();

                // Check if we're entering [dependencies] section
                if trimmed == "[dependencies]" {
                    in_dependencies = true;
                    continue;
                }

                // Check if we're leaving [dependencies] section (another [section])
                if in_dependencies && trimmed.starts_with('[') {
                    break;
                }

                // If we're in dependencies and line has content (not comment/empty)
                if in_dependencies && !trimmed.is_empty() && !trimmed.starts_with('#') {
                    // THE WINDJAMMER WAY: Convert relative paths to absolute
                    // This prevents "No such file" errors when building from build/
                    let processed_line = if trimmed.contains("path = ") {
                        // Extract the path value
                        if let Some(path_start) = trimmed.find("path = \"") {
                            let after_quote = path_start + 8; // len("path = \"")
                            if let Some(path_end) = trimmed[after_quote..].find('"') {
                                let rel_path = &trimmed[after_quote..after_quote + path_end];

                                // Check if it's a relative path (../ or ./)
                                if rel_path.starts_with("../") || rel_path.starts_with("./") {
                                    // Convert to absolute path
                                    let abs_path = source_dir_for_paths.join(rel_path);
                                    let abs_path = abs_path.canonicalize().unwrap_or(abs_path);

                                    // Replace the relative path with absolute path
                                    let before = &trimmed[..after_quote];
                                    let after = &trimmed[after_quote + path_end..];
                                    let new_line = format!(
                                        "{}{}{}",
                                        before,
                                        test_runner::path_to_toml_string(&abs_path),
                                        after
                                    );
                                    new_line
                                } else {
                                    // Already absolute or not a path pattern we handle
                                    trimmed.to_string()
                                }
                            } else {
                                trimmed.to_string()
                            }
                        } else {
                            trimmed.to_string()
                        }
                    } else {
                        trimmed.to_string()
                    };

                    let dep_name = processed_line.split(['=', ' ']).next().unwrap_or("").trim();
                    let skip = matches!(
                        dep_name,
                        "windjammer" | "windjammer-runtime" | "windjammer_runtime"
                    );
                    if !skip {
                        source_cargo_deps.push(processed_line);
                    }
                }
            }
        }
    }

    // For WASM target, generate WASM-specific Cargo.toml
    if target == CompilationTarget::Wasm {
        return create_wasm_cargo_toml(output_dir, imported_modules);
    }

    // Map imported stdlib modules to their Cargo dependencies
    let mut deps = Vec::new();

    // If ANY stdlib module is used, add windjammer-runtime
    if !imported_modules.is_empty() {
        // Add windjammer-runtime dependency (path-based for now)
        // Always search for workspace root, don't trust CARGO_MANIFEST_DIR
        let windjammer_runtime_path = {
            // Start from current directory and search upward
            let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut found = false;

            // Try current directory first (if we're in windjammer repo)
            if current
                .join("crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                current.join("crates/windjammer-runtime")
            }
            // Check if windjammer is a sibling directory (e.g., we're in windjammer-ui)
            else if let Some(parent) = current.parent() {
                if parent
                    .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                    .exists()
                {
                    parent.join("windjammer/crates/windjammer-runtime")
                } else {
                    // Search upward (up to 5 levels)
                    for _ in 0..5 {
                        if let Some(parent) = current.parent() {
                            // Check for windjammer/crates/windjammer-runtime (sibling repo)
                            if parent
                                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                                .exists()
                            {
                                found = true;
                                current = parent.to_path_buf();
                                break;
                            }
                            // Check for crates/windjammer-runtime (legacy path)
                            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                                current = parent.to_path_buf();
                                found = true;
                                break;
                            }
                            current = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }

                    if found {
                        if current
                            .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                            .exists()
                        {
                            current.join("windjammer/crates/windjammer-runtime")
                        } else {
                            current.join("crates/windjammer-runtime")
                        }
                    } else {
                        // Fallback: try sibling first, then legacy path
                        let sibling_path = PathBuf::from("../windjammer/crates/windjammer-runtime");
                        if sibling_path.join("Cargo.toml").exists() {
                            sibling_path
                        } else {
                            PathBuf::from("./crates/windjammer-runtime")
                        }
                    }
                }
            } else {
                // Fallback when no parent
                PathBuf::from("../windjammer/crates/windjammer-runtime")
            }
        };

        deps.push(format!(
            "windjammer-runtime = {{ path = \"{}\" }}",
            test_runner::path_to_toml_string(&windjammer_runtime_path)
        ));
    }

    // Users should add windjammer-ui or other frameworks explicitly in their Cargo.toml
    // The compiler no longer auto-adds these dependencies to avoid filesystem path issues
    let external_crates = external_crates.to_vec();

    // Legacy: Keep old dependencies for modules not yet in runtime
    for module in imported_modules {
        match module.as_str() {
            // These are now in windjammer-runtime, no extra deps needed
            "fs" | "http" | "mime" | "json" => {}

            // UI and other frameworks should be added explicitly by users
            "ui" | "game" => {}

            // Legacy modules that still need direct dependencies
            "csv" => {
                deps.push("csv = \"1.3\"".to_string());
            }
            "time" => {
                deps.push("chrono = \"0.4\"".to_string());
            }
            "log" => {
                deps.push("log = \"0.4\"".to_string());
                deps.push("env_logger = \"0.11\"".to_string());
            }
            "regex" => {
                deps.push("regex = \"1.10\"".to_string());
            }
            "cli" => {
                deps.push("clap = { version = \"4.5\", features = [\"derive\"] }".to_string());
            }
            "crypto" => {
                deps.push("sha2 = \"0.10\"".to_string());
                deps.push("bcrypt = \"0.15\"".to_string());
                deps.push("base64 = \"0.21\"".to_string());
            }
            "random" => {
                deps.push("rand = \"0.8\"".to_string());
            }
            "async" => {
                deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
            }
            "db" => {
                deps.push("sqlx = { version = \"0.7\", features = [\"runtime-tokio-native-tls\", \"sqlite\"] }".to_string());
                deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
            }
            // fs, strings, math, env, process use std library or windjammer-runtime
            _ => {}
        }
    }

    // Add external crates (user-specified or from crates.io)
    // NOTE: Users should explicitly add windjammer-ui or other framework dependencies
    // to their Cargo.toml - the compiler no longer auto-adds filesystem paths
    let mut external_deps = Vec::new();
    for crate_name in external_crates {
        // THE WINDJAMMER WAY: Filter out Rust keywords (crate, super, self)
        // These are language features, not external dependencies!
        if crate_name == "crate" || crate_name == "super" || crate_name == "self" {
            continue; // Skip Rust keywords
        }

        // THE WINDJAMMER WAY: Check if windjammer engine crates are imported
        // If so, add them as path dependencies to the local framework
        if crate_name == "windjammer_game"
            || crate_name == "windjammer-game"
            || crate_name == "windjammer_game_core"
            || crate_name == "windjammer-game-core"
            || crate_name == "windjammer_app"
            || crate_name == "windjammer-app"
            || crate_name == "windjammer_runtime"
            || crate_name == "windjammer-runtime"
        {
            // Try to find windjammer-game in the workspace
            // First, check if WINDJAMMER_GAME_PATH env var is set (for development)
            if let Ok(game_path) = std::env::var("WINDJAMMER_GAME_PATH") {
                let game_path = PathBuf::from(game_path);
                if game_path.exists() {
                    external_deps.push(format!(
                        "windjammer-game = {{ path = \"{}\" }}",
                        test_runner::path_to_toml_string(&game_path)
                    ));
                    continue; // Skip the crates.io fallback
                }
            }

            // Second, try to find it relative to the compiler source
            // This works when compiling from source
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // /path/to/windjammer
            let src_root = manifest_dir.parent().unwrap(); // /path/to (parent of windjammer)

            // THE WINDJAMMER WAY: Determine the correct path based on which crate is imported
            let final_path = if crate_name.contains("_runtime") || crate_name.contains("-runtime") {
                // windjammer-runtime is in windjammer/crates/windjammer-runtime
                src_root.join("windjammer/crates/windjammer-runtime")
            } else {
                // windjammer-app is in windjammer-game/windjammer-game-core
                let game_path = src_root.join("windjammer-game/windjammer-game-core");
                let legacy_game_path = src_root.join("windjammer-game/windjammer-game");

                if game_path.exists() {
                    game_path
                } else if legacy_game_path.exists() {
                    legacy_game_path
                } else {
                    PathBuf::new() // Empty path, will fallback to crates.io
                }
            };

            if !final_path.as_os_str().is_empty() && final_path.exists() {
                // Read the actual crate name from Cargo.toml at the path
                let cargo_toml_path = final_path.join("Cargo.toml");
                let crate_name_normalized = if cargo_toml_path.exists() {
                    // Try to read the actual crate name from Cargo.toml
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                        // Parse name = "..." line
                        if let Some(line) = content.lines().find(|l| l.trim().starts_with("name")) {
                            if let Some(name_part) = line.split('"').nth(1) {
                                name_part.to_string()
                            } else {
                                // Fallback: guess based on crate_name
                                if crate_name.contains("_core") || crate_name.contains("-core") {
                                    "windjammer-game-core".to_string()
                                } else if crate_name.contains("_app") || crate_name.contains("-app")
                                {
                                    "windjammer-app".to_string()
                                } else if crate_name.contains("_runtime")
                                    || crate_name.contains("-runtime")
                                {
                                    "windjammer-runtime".to_string()
                                } else {
                                    "windjammer-game".to_string()
                                }
                            }
                        } else {
                            // Fallback: guess based on crate_name
                            if crate_name.contains("_core") || crate_name.contains("-core") {
                                "windjammer-game-core".to_string()
                            } else if crate_name.contains("_app") || crate_name.contains("-app") {
                                "windjammer-app".to_string()
                            } else if crate_name.contains("_runtime")
                                || crate_name.contains("-runtime")
                            {
                                "windjammer-runtime".to_string()
                            } else {
                                "windjammer-game".to_string()
                            }
                        }
                    } else {
                        // Fallback: guess based on crate_name
                        if crate_name.contains("_core") || crate_name.contains("-core") {
                            "windjammer-game-core".to_string()
                        } else if crate_name.contains("_app") || crate_name.contains("-app") {
                            "windjammer-app".to_string()
                        } else if crate_name.contains("_runtime") || crate_name.contains("-runtime")
                        {
                            "windjammer-runtime".to_string()
                        } else {
                            "windjammer-game".to_string()
                        }
                    }
                } else {
                    // Fallback: guess based on crate_name
                    if crate_name.contains("_core") || crate_name.contains("-core") {
                        "windjammer-game-core".to_string()
                    } else if crate_name.contains("_app") || crate_name.contains("-app") {
                        "windjammer-app".to_string()
                    } else if crate_name.contains("_runtime") || crate_name.contains("-runtime") {
                        "windjammer-runtime".to_string()
                    } else {
                        "windjammer-game".to_string()
                    }
                };

                external_deps.push(format!(
                    "{} = {{ path = \"{}\" }}",
                    crate_name_normalized,
                    test_runner::path_to_toml_string(&final_path)
                ));
                continue; // Skip the crates.io fallback
            }

            // Fallback: assume it's on crates.io (for published version)
            let crate_name_normalized =
                if crate_name.contains("_core") || crate_name.contains("-core") {
                    "windjammer-game-core"
                } else if crate_name.contains("_app") || crate_name.contains("-app") {
                    "windjammer-app"
                } else if crate_name.contains("_runtime") || crate_name.contains("-runtime") {
                    "windjammer-runtime"
                } else {
                    "windjammer-game"
                };
            external_deps.push(format!("{} = \"*\"", crate_name_normalized));
        } else {
            // All other external crates are assumed to be from crates.io
            external_deps.push(format!("{} = \"*\"", crate_name));
        }
    }

    deps.extend(external_deps);

    // Add optimization dependencies (always included for now)
    // TODO: Only add these if actually used by checking CodeGenerator flags
    deps.push("smallvec = \"1.13\"".to_string());
    deps.push("serde = { version = \"1.0\", features = [\"derive\"] }".to_string());

    // THE WINDJAMMER WAY: Merge in FFI dependencies from source Cargo.toml
    // This enables dogfooding with game engine that has FFI dependencies
    deps.extend(source_cargo_deps);

    // Smart deduplication: extract package names and keep more specific versions
    let mut seen_packages = std::collections::HashSet::new();
    let mut deduplicated_deps = Vec::new();

    // Sort so that more specific versions (with braces) come after simple ones
    deps.sort_by(|a, b| {
        let a_has_braces = a.contains('{');
        let b_has_braces = b.contains('{');
        match (a_has_braces, b_has_braces) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });

    for dep in deps {
        // Extract package name (everything before '=')
        if let Some(pkg_name) = dep.split('=').next() {
            let pkg_name = pkg_name.trim();
            if !seen_packages.contains(pkg_name) {
                seen_packages.insert(pkg_name.to_string());
                deduplicated_deps.push(dep);
            } else {
                // If we've seen this package before, check if this version is more specific
                if dep.contains('{') {
                    // Remove the old simple version and add this more specific one
                    deduplicated_deps.retain(|d| !d.starts_with(pkg_name));
                    deduplicated_deps.push(dep);
                }
            }
        }
    }

    deps = deduplicated_deps;

    let deps_section = if deps.is_empty() {
        String::new()
    } else {
        format!("[dependencies]\n{}\n\n", deps.join("\n"))
    };

    // Use project name from wj.toml (canonical) or game.toml (legacy fallback)
    let project_name = crate::cargo_toml::infer_project_name_from(source_dir);
    let lib_name_normalized = project_name.replace('-', "_");

    // THE WINDJAMMER WAY: Check if lib.rs exists to determine if this is a library or binary project
    let has_lib_rs = output_dir.join("lib.rs").exists();
    let has_main_rs = output_dir.join("main.rs").exists();

    let lib_or_bin_section = if has_lib_rs {
        // Library project - generate [lib] section
        format!(
            "[lib]\nname = \"{}\"\npath = \"lib.rs\"\n\n",
            lib_name_normalized
        )
    } else if has_main_rs {
        // Binary project with main.rs - generate [[bin]] section
        format!(
            "[[bin]]\nname = \"{}\"\npath = \"main.rs\"\n\n",
            project_name
        )
    } else {
        // TDD FIX (Bug #2): Detect test files and generate appropriate targets
        // Multiple standalone files - detect file type and generate [[bin]] or [[test]]
        let mut target_sections = Vec::new();
        if let Ok(entries) = fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".rs") {
                        let file_path = entry.path();
                        let file_type = detect_rust_file_type(&file_path);

                        let target_name = filename.strip_suffix(".rs").unwrap_or(filename);

                        match file_type {
                            RustFileType::Test => {
                                // Test file: generate [[test]] target
                                target_sections.push(format!(
                                    "[[test]]\nname = \"{}\"\npath = \"{}\"\n",
                                    target_name, filename
                                ));
                            }
                            RustFileType::Binary => {
                                // Executable: generate [[bin]] target
                                target_sections.push(format!(
                                    "[[bin]]\nname = \"{}\"\npath = \"{}\"\n",
                                    target_name, filename
                                ));
                            }
                            RustFileType::Library => {
                                // Library code: no target needed (just a module)
                            }
                        }
                    }
                }
            }
        }

        if !target_sections.is_empty() {
            format!("{}\n", target_sections.join("\n"))
        } else {
            String::new()
        }
    };

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
        project_name, deps_section, lib_or_bin_section
    );

    eprintln!(
        "DEBUG: Generated Cargo.toml with package name: {}",
        project_name
    );

    let cargo_toml_path = output_dir.join("Cargo.toml");
    fs::write(cargo_toml_path, cargo_toml)?;

    Ok(())
}

/// Create WASM-specific Cargo.toml
pub fn create_wasm_cargo_toml(output_dir: &Path, imported_modules: &HashSet<String>) -> Result<()> {
    use std::env;
    use std::fs;

    // Check if platform APIs are used (requires windjammer-runtime)
    let uses_platform_apis = imported_modules.iter().any(|m| {
        m == "fs"
            || m == "process"
            || m == "dialog"
            || m == "env"
            || m == "encoding"
            || m.starts_with("fs::")
            || m.starts_with("process::")
            || m.starts_with("dialog::")
            || m.starts_with("env::")
            || m.starts_with("encoding::")
    });

    // Find windjammer-runtime path
    let windjammer_runtime_path = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir).join("crates/windjammer-runtime")
    } else {
        let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut found = false;

        if current
            .join("crates/windjammer-runtime/Cargo.toml")
            .exists()
        {
            current.join("crates/windjammer-runtime")
        } else {
            for _ in 0..5 {
                if let Some(parent) = current.parent() {
                    if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                        current = parent.to_path_buf();
                        found = true;
                        break;
                    }
                    current = parent.to_path_buf();
                } else {
                    break;
                }
            }

            if found {
                current.join("crates/windjammer-runtime")
            } else {
                PathBuf::from("./crates/windjammer-runtime")
            }
        }
    };

    // Find the first .rs file in the output directory to use as lib.rs
    let lib_file = fs::read_dir(output_dir)?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry.path().extension().and_then(|s| s.to_str()) == Some("rs")
                && entry.path().file_name().and_then(|s| s.to_str()) != Some("main.rs")
        })
        .and_then(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "lib.rs".to_string());

    let cargo_toml = format!(
        r#"[package]
name = "windjammer-wasm"
version = "0.1.0"
edition = "2021"

# Prevent this from being treated as part of parent workspace
[workspace]

[lib]
crate-type = ["cdylib"]
path = "{}"

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde-wasm-bindgen = "0.6"
web-sys = {{ version = "0.3", features = [
    "Document",
    "Element",
    "HtmlElement",
    "Node",
    "Text",
    "Window",
    "Event",
    "MouseEvent",
    "KeyboardEvent",
] }}
js-sys = "0.3"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
console_error_panic_hook = "0.1"
{}
[profile.release]
opt-level = "z"  # Optimize for size
lto = true
"#,
        lib_file,
        if uses_platform_apis {
            format!(
                "windjammer-runtime = {{ path = \"{}\", features = [\"wasm\"] }}",
                test_runner::path_to_toml_string(&windjammer_runtime_path)
            )
        } else {
            String::new()
        }
    );

    let cargo_toml_path = output_dir.join("Cargo.toml");
    fs::write(cargo_toml_path, cargo_toml)?;

    Ok(())
}

/// Run cargo build on the generated Rust code and display errors with source mapping
pub fn check_with_cargo(output_dir: &Path, show_raw_errors: bool) -> Result<()> {
    use colored::*;
    use std::process::Command;

    println!("\n{} Rust compilation...", "Checking".cyan().bold());

    let output = Command::new("cargo")
        .arg("build")
        .arg("--message-format=json")
        .current_dir(output_dir)
        .output()?;

    if output.status.success() {
        println!("{} No Rust compilation errors!", "Success!".green().bold());
        return Ok(());
    }

    // Combine stderr and stdout (cargo outputs to both)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined_output = format!("{}{}", stderr, stdout);

    // If raw errors requested, show them and exit
    if show_raw_errors {
        println!("{} Rust compilation errors (raw):", "Error:".red().bold());
        println!("{}", combined_output);
        return Err(anyhow::anyhow!("Rust compilation failed"));
    }

    // Load all source maps from the output directory
    let source_maps = load_source_maps(output_dir)?;

    // Create error mapper with merged source maps
    let error_mapper = error_mapper::ErrorMapper::new(source_maps);

    // Map rustc output to Windjammer diagnostics
    let wj_diagnostics = error_mapper.map_rustc_output(&combined_output);

    if wj_diagnostics.is_empty() {
        // Fallback: show raw output if we couldn't parse any diagnostics
        println!(
            "{} Could not parse Rust compilation errors. Showing raw output:",
            "Warning:".yellow().bold()
        );
        println!("{}", combined_output);
        return Err(anyhow::anyhow!("Rust compilation failed"));
    }

    // Display Windjammer diagnostics with beautiful formatting
    let error_count = wj_diagnostics
        .iter()
        .filter(|d| matches!(d.level, error_mapper::DiagnosticLevel::Error))
        .count();
    let warning_count = wj_diagnostics
        .iter()
        .filter(|d| matches!(d.level, error_mapper::DiagnosticLevel::Warning))
        .count();

    if error_count > 0 {
        println!(
            "\n{} {} error{} detected:\n",
            "Error:".red().bold(),
            error_count,
            if error_count == 1 { "" } else { "s" }
        );
    }
    if warning_count > 0 {
        println!(
            "{} {} warning{}\n",
            "Warning:".yellow().bold(),
            warning_count,
            if warning_count == 1 { "" } else { "s" }
        );
    }

    for diag in &wj_diagnostics {
        // Use the beautiful formatted output from WindjammerDiagnostic
        let formatted = diag.format();

        // Add colors
        let colored_output = colorize_diagnostic(&formatted, &diag.level);
        println!("{}", colored_output);
    }

    if error_count > 0 {
        Err(anyhow::anyhow!(
            "Compilation failed with {} error{}",
            error_count,
            if error_count == 1 { "" } else { "s" }
        ))
    } else {
        Ok(())
    }
}
