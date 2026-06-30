//! Compiling the project under test, FFI wiring, and generating the Rust test harness crate.

use crate::{build_project, build_project_ext, CompilationTarget};
use anyhow::Result;
use std::path::{Path, PathBuf};

use super::test_discovery::TestFunction;
use super::util::copy_dir_recursive;

/// Windjammer test sources use `use crate::module::...` (same as in-crate unit tests).
/// When the harness compiles tests into a separate `windjammer-tests` crate, rewrite those
/// paths to the library under test (e.g. `foobar_api::domain::...`).
pub fn rewrite_test_crate_imports(rust_code: &str, lib_crate_name: &str) -> String {
    let mut out = rust_code.to_string();
    // `pub use crate::` must be rewritten before bare `use crate::`.
    out = out.replace("pub use crate::", &format!("pub use {}::", lib_crate_name));
    out = out.replace("use crate::", &format!("use {}::", lib_crate_name));
    out
}

/// True when `dir` contains at least one `.wj` source file (recursively).
fn directory_has_wj_sources(dir: &Path) -> bool {
    use std::fs;
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if directory_has_wj_sources(&path) {
                return true;
            }
        } else if path.extension().is_some_and(|e| e == "wj") {
            return true;
        }
    }
    false
}

/// Detect and compile the library being tested (if it exists)
/// Returns (library_name, library_path) for Cargo dependency, or None
fn detect_and_compile_library(
    project_root: &Path,
    test_output_dir: &Path,
) -> Result<Option<(String, String, PathBuf)>> {
    use std::fs;

    // Look for wj.toml or windjammer.toml
    let config_path = if project_root.join("wj.toml").exists() {
        Some(project_root.join("wj.toml"))
    } else if project_root.join("windjammer.toml").exists() {
        Some(project_root.join("windjammer.toml"))
    } else {
        None
    };

    let config = if let Some(path) = config_path {
        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str::<crate::config::WjConfig>(&content).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    // Check if there's a library to compile
    let src_dir = project_root.join("src");
    if !src_dir.exists() || !src_dir.is_dir() {
        return Ok(None); // No library to compile
    }

    // Rust-only `src/` (e.g. windjammer compiler) is not a Windjammer library project.
    if !directory_has_wj_sources(&src_dir) {
        return Ok(None);
    }

    // Get library name from config or infer from directory
    let lib_name = config
        .as_ref()
        .and_then(|c| {
            if !c.package.name.is_empty() {
                Some(c.package.name.clone())
            } else {
                c.project.as_ref().map(|p| p.name.clone())
            }
        })
        .or_else(|| {
            project_root
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.replace('-', "_"))
        })
        .unwrap_or_else(|| "lib".to_string());

    // Create library output directory (clean it first to avoid stale files)
    let lib_output_dir = test_output_dir.join("lib");
    if lib_output_dir.exists() {
        fs::remove_dir_all(&lib_output_dir)?;
    }
    fs::create_dir_all(&lib_output_dir)?;

    // Compile the library
    use colored::*;
    println!(
        "   {} Compiling library: {}",
        "→".bright_blue().bold(),
        lib_name
    );

    // Prefer mod.wj entry (excludes main.wj binary) when present — matches `wj build src/mod.wj`.
    let build_entry = if src_dir.join("mod.wj").exists() {
        src_dir.join("mod.wj")
    } else {
        src_dir.clone()
    };

    eprintln!("DEBUG: About to call build_project_ext for library");
    match build_project_ext(
        &build_entry,
        &lib_output_dir,
        CompilationTarget::Rust,
        true,
        true,
        &[],
    ) {
        Ok(_) => {
            eprintln!("DEBUG: build_project returned Ok");
            // Generate lib.rs entry point for the compiled library
            // build_project generates Rust files but doesn't create lib.rs
            if let Err(e) = generate_lib_rs_for_library(&lib_output_dir) {
                eprintln!("WARNING: Failed to generate lib.rs: {}", e);
                // Continue anyway - the library might still work
            } else {
                eprintln!("DEBUG: generate_lib_rs_for_library succeeded");
            }

            // TDD FIX: Copy FFI files from src/ffi to test library
            // THE WINDJAMMER WAY: Dynamic, robust FFI integration
            // This enables tests to work with full FFI functionality
            if let Err(e) = copy_ffi_files_to_test_library(project_root, &lib_output_dir) {
                eprintln!("WARNING: Failed to copy FFI files: {}", e);
                // Continue anyway - tests might not need FFI
            } else {
                eprintln!("DEBUG: FFI files copied successfully");
            }

            eprintln!("DEBUG: About to fix Cargo.toml");

            // TDD FIX: Use the project's actual lib name, not a _testlib suffix
            // THE WINDJAMMER WAY: Test library name must match project lib name so imports work
            // Bug: Tests were failing with E0433: unresolved module windjammer_game_core
            // Root Cause: Test library was using *_testlib suffix, breaking imports
            // Fix: Read the actual [lib] name from project's Cargo.toml

            // Read project's Cargo.toml to get the actual lib name
            let project_cargo_toml = project_root.join("Cargo.toml");
            let actual_lib_name = if project_cargo_toml.exists() {
                match fs::read_to_string(&project_cargo_toml) {
                    Ok(content) => {
                        // Parse [lib] name from Cargo.toml
                        if let Some(lib_section_start) = content.find("[lib]") {
                            if let Some(name_start) = content[lib_section_start..].find("name = \"")
                            {
                                let abs_start = lib_section_start + name_start + "name = \"".len();
                                content[abs_start..].find('"').map(|name_end| {
                                    content[abs_start..abs_start + name_end].to_string()
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            // TDD FIX: Use actual library package name from project's Cargo.toml
            // THE WINDJAMMER WAY: Test dependencies must use real library names
            // Bug: Tests failing with E0433: unresolved module windjammer_game_core
            // Root Cause: Test Cargo.toml uses windjammer-game-core-testlib, but tests import windjammer_game_core
            // Fix: Read actual [package] name from project's Cargo.toml and use that for test dependency

            let actual_package_name = if project_cargo_toml.exists() {
                match fs::read_to_string(&project_cargo_toml) {
                    Ok(content) => {
                        // Parse [package] name from Cargo.toml
                        content.find("[package]").and_then(|pkg_start| {
                            content[pkg_start..]
                                .find("name = \"")
                                .and_then(|name_start| {
                                    let abs_start = pkg_start + name_start + "name = \"".len();
                                    content[abs_start..].find('"').map(|name_end| {
                                        content[abs_start..abs_start + name_end].to_string()
                                    })
                                })
                        })
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            // Use actual lib name from Cargo.toml, or infer from package name
            let test_lib_name = actual_lib_name.unwrap_or_else(|| lib_name.replace('-', "_"));
            // Use actual package name from Cargo.toml, or infer from directory (NO -testlib suffix!)
            let test_lib_package_name =
                actual_package_name.unwrap_or_else(|| lib_name.replace('_', "-"));

            // Fix the generated Cargo.toml to use the correct library name and add user dependencies
            let cargo_toml_path = lib_output_dir.join("Cargo.toml");

            println!(
                "   {} Reading Cargo.toml from: {}",
                "→".blue().bold(),
                cargo_toml_path.display()
            );

            match fs::read_to_string(&cargo_toml_path) {
                Ok(mut cargo_toml) => {
                    // Replace [package] name with unique test library name
                    if let Some(pkg_start) = cargo_toml.find("[package]") {
                        if let Some(name_start) = cargo_toml[pkg_start..].find("name = \"") {
                            let abs_name_start = pkg_start + name_start + "name = \"".len();
                            if let Some(name_end) = cargo_toml[abs_name_start..].find('"') {
                                let abs_name_end = abs_name_start + name_end;
                                cargo_toml.replace_range(
                                    abs_name_start..abs_name_end,
                                    &test_lib_package_name,
                                );
                            }
                        }
                    }

                    // Replace [lib] name with unique test library name
                    if let Some(lib_start) = cargo_toml.find("[lib]") {
                        if let Some(name_start) = cargo_toml[lib_start..].find("name = \"") {
                            let abs_name_start = lib_start + name_start + "name = \"".len();
                            if let Some(name_end) = cargo_toml[abs_name_start..].find('"') {
                                let abs_name_end = abs_name_start + name_end;
                                cargo_toml
                                    .replace_range(abs_name_start..abs_name_end, &test_lib_name);
                            }
                        }
                    }

                    // Remove self-referential dependency (library importing itself)
                    let self_dep_pattern = format!("{} = {{", lib_name);
                    if let Some(start) = cargo_toml.find(&self_dep_pattern) {
                        // Find the end of this dependency line
                        if let Some(end) = cargo_toml[start..].find('\n') {
                            let line_end = start + end + 1;
                            cargo_toml.replace_range(start..line_end, "");
                        }
                    }

                    // Add user dependencies from wj.toml (replace wildcards with proper specs)
                    if let Some(cfg) = &config {
                        let mut deps_section = String::new();
                        for (dep_name, dep_spec) in &cfg.dependencies {
                            // If dependency exists with wildcard version, replace it
                            let wildcard_pattern = format!("{} = \"*\"", dep_name);
                            if cargo_toml.contains(&wildcard_pattern) {
                                // Remove the wildcard line
                                cargo_toml =
                                    cargo_toml.replace(&format!("{}\n", wildcard_pattern), "");
                            }
                            // If dependency already exists with a proper spec, skip it
                            else if cargo_toml.contains(&format!("{} =", dep_name)) {
                                continue;
                            }

                            use crate::config::DependencySpec;
                            match dep_spec {
                                DependencySpec::Simple(version) => {
                                    deps_section
                                        .push_str(&format!("{} = \"{}\"\n", dep_name, version));
                                }
                                DependencySpec::Detailed {
                                    version,
                                    features,
                                    path,
                                    git,
                                    branch,
                                } => {
                                    deps_section.push_str(&format!("{} = {{ ", dep_name));
                                    if let Some(v) = version {
                                        deps_section.push_str(&format!("version = \"{}\", ", v));
                                    }
                                    if let Some(p) = path {
                                        // Make path relative to project root
                                        let abs_path = project_root.join(p);
                                        deps_section.push_str(&format!(
                                            "path = \"{}\", ",
                                            path_to_toml_string(&abs_path)
                                        ));
                                    }
                                    // Add desktop feature for windjammer-ui
                                    if dep_name == "windjammer-ui" && !features.is_some() {
                                        deps_section.push_str("features = [\"desktop\"], ");
                                    }
                                    if let Some(g) = git {
                                        deps_section.push_str(&format!("git = \"{}\", ", g));
                                    }
                                    if let Some(b) = branch {
                                        deps_section.push_str(&format!("branch = \"{}\", ", b));
                                    }
                                    if let Some(f) = features {
                                        deps_section.push_str(&format!("features = {:?}, ", f));
                                    }
                                    // Remove trailing comma and space
                                    if deps_section.ends_with(", ") {
                                        deps_section.truncate(deps_section.len() - 2);
                                    }
                                    deps_section.push_str(" }\n");
                                }
                            }
                        }

                        // Insert dependencies before [lib] section
                        if !deps_section.is_empty() {
                            if let Some(lib_pos) = cargo_toml.find("[lib]") {
                                cargo_toml.insert_str(lib_pos, &deps_section);
                            }
                        }
                    }

                    if let Err(e) = fs::write(&cargo_toml_path, &cargo_toml) {
                        eprintln!("WARNING: Failed to write Cargo.toml: {}", e);
                    } else {
                        println!(
                            "   {} Updated library Cargo.toml (package: {}, lib: {})",
                            "✓".green().bold(),
                            test_lib_package_name,
                            test_lib_name
                        );
                    }
                }
                Err(e) => {
                    eprintln!("WARNING: Failed to read Cargo.toml: {}", e);
                }
            }

            let cargo_toml_path = lib_output_dir.join("Cargo.toml");
            if !cargo_toml_path.exists() {
                eprintln!(
                    "WARNING: Library compile produced no Cargo.toml at {} — skipping library dependency",
                    cargo_toml_path.display()
                );
                return Ok(None);
            }

            println!("   {} Library compiled successfully", "✓".green().bold());
            Ok(Some((test_lib_name, test_lib_package_name, lib_output_dir)))
        }
        Err(e) => {
            println!("   {} Library compilation failed: {}", "✗".red().bold(), e);
            // Don't fail the entire test run - just continue without library dependency
            Ok(None)
        }
    }
}

/// Generate lib.rs entry point for a compiled library
/// This creates a proper Rust library crate structure that tests can import from
fn generate_lib_rs_for_library(lib_output_dir: &Path) -> Result<()> {
    use std::collections::HashSet;
    use std::fs;

    // Find all modules (directories with mod.rs AND top-level .rs files)
    let mut dir_modules = HashSet::new();
    let mut file_modules = Vec::new();

    for entry in fs::read_dir(lib_output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Directory modules (must have mod.rs)
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if path.join("mod.rs").exists() {
                    dir_modules.insert(dir_name.to_string());
                }
            }
        } else if path.is_file() {
            // Top-level .rs files (but not lib.rs or Cargo.toml)
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "lib.rs" {
                    // Extract module name (remove .rs extension)
                    let module_name = file_name.trim_end_matches(".rs");
                    file_modules.push(module_name.to_string());
                }
            }
        }
    }

    // TDD FIX: When both window.rs and window/mod.rs exist, prefer directory module
    // This prevents E0761: file for module `window` found at both locations
    // Directory modules take precedence (standard Rust convention)
    let mut modules: Vec<String> = dir_modules.iter().cloned().collect();

    // Add file modules that don't conflict with directory modules
    for file_module in file_modules {
        if !dir_modules.contains(&file_module) {
            modules.push(file_module);
        }
    }

    if modules.is_empty() {
        return Ok(()); // No modules to export
    }

    // TDD FIX: Filter out "lib" module to prevent E0761 conflict
    // "lib" is a reserved name for the library itself, not a module to import
    // This prevents: error[E0761]: file for module `lib` found at both "lib.rs" and "lib/mod.rs"
    // Also exclude "mod" — `pub mod mod;` is invalid (keyword); mod.rs is the barrel, not a child module
    // `main` is the binary entry — not part of the library surface under test.
    modules.retain(|m| m != "lib" && m != "mod" && m != "main");

    if modules.is_empty() {
        return Ok(()); // No modules to export after filtering
    }

    modules.sort();

    // Generate lib.rs content
    let mut lib_rs = String::from("// Auto-generated library entry point\n\n");

    // Declare all modules
    for module in &modules {
        lib_rs.push_str(&format!("pub mod {};\n", module));
    }

    lib_rs.push_str("\n// Re-export for convenience\n");
    for module in &modules {
        lib_rs.push_str(&format!("pub use {}::*;\n", module));
    }

    // Write lib.rs
    fs::write(lib_output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

/// TDD FIX: Copy FFI files from project src/ffi to test library output
/// THE WINDJAMMER WAY: Dynamic, robust FFI integration for tests
///
/// This function:
/// 1. Checks if project has src/ffi directory
/// 2. Recursively copies all .rs files
/// 3. Copies shader files (.wgsl) if they exist
/// 4. Updates lib.rs to include pub mod ffi
/// 5. Returns Ok even if no FFI files exist (optional feature)
fn copy_ffi_files_to_test_library(project_root: &Path, lib_output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    // TDD FIX: Check for FFI directory in multiple locations
    // THE WINDJAMMER WAY: Support both src/ffi (library) and ffi/ (game engine) layouts
    // 1. Check ffi/ at project root (game engine layout)
    // 2. Check src/ffi/ (library layout)
    let ffi_locations = [
        project_root.join("ffi"),
        project_root.join("src").join("ffi"),
    ];

    let src_ffi_dir = ffi_locations
        .iter()
        .find(|path| path.exists() && path.is_dir())
        .cloned();

    let src_ffi_dir = match src_ffi_dir {
        Some(dir) => dir,
        None => {
            // No FFI directory - this is fine, not all projects need FFI
            return Ok(());
        }
    };

    println!(
        "   {} Copying FFI files from {}",
        "→".bright_blue().bold(),
        src_ffi_dir.display()
    );

    // Create ffi directory in lib output
    let dest_ffi_dir = lib_output_dir.join("ffi");
    fs::create_dir_all(&dest_ffi_dir)?;

    // Recursively copy all .rs files from src/ffi
    copy_ffi_files_recursive(&src_ffi_dir, &dest_ffi_dir)?;

    // Update lib.rs to include ffi module
    let lib_rs_path = lib_output_dir.join("lib.rs");
    if lib_rs_path.exists() {
        let mut lib_rs_content = fs::read_to_string(&lib_rs_path)?;

        // Check if ffi module is already declared
        if !lib_rs_content.contains("pub mod ffi") {
            // Insert ffi module declaration after other module declarations
            // but before re-exports
            if let Some(reexport_pos) = lib_rs_content.find("// Re-export for convenience") {
                lib_rs_content
                    .insert_str(reexport_pos, "pub mod ffi; // FFI Rust implementations\n\n");
            } else {
                // No re-export comment found, append at end
                lib_rs_content.push_str("\npub mod ffi; // FFI Rust implementations\n");
            }

            fs::write(&lib_rs_path, lib_rs_content)?;
            println!(
                "   {} Updated lib.rs to include FFI module",
                "✓".green().bold()
            );
        }
    }

    // TDD FIX: Copy FFI dependencies from project Cargo.toml to test library Cargo.toml
    // This ensures that FFI code that uses external crates (like wgpu) can compile
    copy_ffi_dependencies_to_test_library(project_root, lib_output_dir)?;

    println!("   {} FFI files copied successfully", "✓".green().bold());

    Ok(())
}

/// TDD FIX: Copy FFI dependencies from project Cargo.toml to test library Cargo.toml
/// THE WINDJAMMER WAY: Dynamic, robust dependency management for FFI
fn copy_ffi_dependencies_to_test_library(project_root: &Path, lib_output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    // Read project's Cargo.toml
    let project_cargo_toml = project_root.join("Cargo.toml");
    if !project_cargo_toml.exists() {
        // No Cargo.toml - no FFI dependencies to copy
        return Ok(());
    }

    let cargo_toml_content = fs::read_to_string(&project_cargo_toml)?;
    let cargo_toml: toml::Value = toml::from_str(&cargo_toml_content)?;

    // Extract dependencies
    let mut ffi_deps = Vec::new();
    if let Some(deps) = cargo_toml.get("dependencies").and_then(|v| v.as_table()) {
        for (dep_name, dep_spec) in deps {
            // Skip windjammer-runtime (already added by test framework)
            // Skip dependencies that are paths to local crates
            if dep_name == "windjammer-runtime" {
                continue;
            }

            // Add all other dependencies (these are typically external crates needed by FFI)
            ffi_deps.push((dep_name.clone(), dep_spec.clone()));
        }
    }

    if ffi_deps.is_empty() {
        // No FFI dependencies to copy
        return Ok(());
    }

    println!(
        "   {} Copying {} FFI dependencies to test library",
        "→".bright_blue().bold(),
        ffi_deps.len()
    );

    // Read test library's Cargo.toml
    let test_cargo_toml_path = lib_output_dir.join("Cargo.toml");
    let mut test_cargo_toml_content = fs::read_to_string(&test_cargo_toml_path)?;

    // Add FFI dependencies to the [dependencies] section
    for (dep_name, dep_spec) in ffi_deps {
        // Skip if dependency already exists
        if test_cargo_toml_content.contains(&format!("{} =", dep_name)) {
            continue;
        }

        // Format dependency spec as TOML
        let dep_line = if let Some(version_str) = dep_spec.as_str() {
            // Simple version string: dep = "1.0"
            format!("{} = \"{}\"", dep_name, version_str)
        } else if let Some(table) = dep_spec.as_table() {
            // Complex dependency with features, etc.
            let mut parts = Vec::new();
            if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
                parts.push(format!("version = \"{}\"", version));
            }
            if let Some(features) = table.get("features") {
                // Handle features array more carefully
                if let Ok(features_str) = toml::to_string(features) {
                    parts.push(format!("features = {}", features_str.trim()));
                } else if let Some(arr) = features.as_array() {
                    // Manual array formatting as fallback
                    let feature_list: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| format!("\"{}\"", s))
                        .collect();
                    parts.push(format!("features = [{}]", feature_list.join(", ")));
                }
            }
            if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                // Make path absolute for test library
                let abs_path = project_root.join(path);
                parts.push(format!("path = \"{}\"", path_to_toml_string(&abs_path)));
            }
            if parts.is_empty() {
                // No useful parts extracted, skip this dependency
                continue;
            }
            format!("{} = {{ {} }}", dep_name, parts.join(", "))
        } else {
            continue;
        };

        // Find [dependencies] section and add the dependency
        if let Some(deps_pos) = test_cargo_toml_content.find("[dependencies]") {
            // Find the end of the [dependencies] section (next [section] or EOF)
            let after_deps = &test_cargo_toml_content[deps_pos + "[dependencies]".len()..];
            if let Some(next_section_pos) = after_deps.find("\n[") {
                // Insert before next section
                let insert_pos = deps_pos + "[dependencies]".len() + next_section_pos;
                test_cargo_toml_content.insert_str(insert_pos, &format!("\n{}", dep_line));
            } else {
                // Append at end of file
                test_cargo_toml_content.push_str(&format!("\n{}\n", dep_line));
            }
        }
    }

    // Write updated Cargo.toml
    fs::write(&test_cargo_toml_path, test_cargo_toml_content)?;

    println!(
        "   {} FFI dependencies added to test library Cargo.toml",
        "✓".green().bold()
    );

    Ok(())
}

/// Recursively copy .rs and .wgsl files from source to destination
fn copy_ffi_files_recursive(src: &Path, dest: &Path) -> Result<()> {
    use std::fs;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = match path.file_name() {
            Some(name) => name,
            None => continue,
        };

        if path.is_dir() {
            // Recursively copy subdirectories (e.g., shaders/)
            let dest_subdir = dest.join(file_name);
            fs::create_dir_all(&dest_subdir)?;
            copy_ffi_files_recursive(&path, &dest_subdir)?;
        } else if path.is_file() {
            // Copy .rs files and .wgsl shader files
            if let Some(file_name_str) = file_name.to_str() {
                if file_name_str.ends_with(".rs") || file_name_str.ends_with(".wgsl") {
                    let dest_file = dest.join(file_name);
                    fs::copy(&path, &dest_file)?;
                }
            }
        }
    }

    Ok(())
}

/// Convert a path to TOML-safe format (forward slashes, no Windows \\?\ prefix)
/// Windows canonicalize() adds \\?\ prefix; backslashes cause TOML parse errors
pub fn path_to_toml_string(path: &Path) -> String {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}

/// Find windjammer-runtime path using robust search logic
fn find_windjammer_runtime_path() -> Result<PathBuf> {
    use std::env;

    // Compiled into the `wj` binary: always valid when built from the windjammer repo.
    let compiler_runtime =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime");
    if compiler_runtime.join("Cargo.toml").exists() {
        return Ok(compiler_runtime);
    }

    // Installed `wj` binary: walk up from the executable location.
    if let Ok(exe_path) = env::current_exe() {
        let mut search = exe_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        for _ in 0..8 {
            let candidate = search.join("crates/windjammer-runtime");
            if candidate.join("Cargo.toml").exists() {
                return Ok(candidate);
            }
            if let Some(parent) = search.parent() {
                search = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    // Runtime env var (e.g. `cargo test` subprocess with CARGO_MANIFEST_DIR set).
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let runtime_path = PathBuf::from(manifest_dir).join("crates/windjammer-runtime");
        if runtime_path.join("Cargo.toml").exists() {
            return Ok(runtime_path);
        }
    }

    // Start from current directory and search upward
    let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Try current directory first (if we're in windjammer repo)
    if current
        .join("crates/windjammer-runtime/Cargo.toml")
        .exists()
    {
        return Ok(current.join("crates/windjammer-runtime"));
    }

    // Search up to 10 levels (increased from 5 for deeper project structures)
    for _ in 0..10 {
        if let Some(parent) = current.parent() {
            // Check for windjammer/crates/windjammer-runtime (nested structure)
            if parent
                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(parent.join("windjammer/crates/windjammer-runtime"));
            }

            // Check for crates/windjammer-runtime (flat structure)
            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                return Ok(parent.join("crates/windjammer-runtime"));
            }

            // Check current level for windjammer/crates (sibling search)
            if current
                .join("../windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(current.join("../windjammer/crates/windjammer-runtime"));
            }

            // Check for ../../windjammer/crates (deeper sibling)
            if current
                .join("../../windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(current.join("../../windjammer/crates/windjammer-runtime"));
            }

            // Check for ../../../windjammer/crates (even deeper)
            if current
                .join("../../../windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                return Ok(current.join("../../../windjammer/crates/windjammer-runtime"));
            }

            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback: try relative paths and canonicalize them
    let candidates = vec![
        PathBuf::from("../windjammer/crates/windjammer-runtime"),
        PathBuf::from("../../windjammer/crates/windjammer-runtime"),
        PathBuf::from("../../../windjammer/crates/windjammer-runtime"),
        PathBuf::from("./crates/windjammer-runtime"),
    ];

    for candidate in candidates {
        if candidate.join("Cargo.toml").exists() {
            // Canonicalize to get absolute path
            if let Ok(canonical) = candidate.canonicalize() {
                return Ok(canonical);
            }
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "could not locate windjammer-runtime (searched compiler manifest, executable path, and cwd ancestors)"
    )
}

/// Generate Rust test harness from Windjammer tests
pub(crate) fn generate_test_harness(
    output_dir: &Path,
    tests: &[TestFunction],
    filter: Option<&str>,
    project_root: &Path,
) -> Result<()> {
    use std::collections::HashMap;
    use std::fs;

    // Group tests by file
    let mut tests_by_file: HashMap<PathBuf, Vec<&TestFunction>> = HashMap::new();
    for test in tests {
        tests_by_file
            .entry(test.file.clone())
            .or_default()
            .push(test);
    }

    // Compile each test file using the existing infrastructure
    for (file, file_tests) in &tests_by_file {
        // Skip if filter doesn't match
        if let Some(filter_str) = filter {
            if !file_tests.iter().any(|t| t.name.contains(filter_str)) {
                continue;
            }
        }

        // Compile the file to Rust
        build_project(file, output_dir, CompilationTarget::Rust, false)?;

        // Read the generated Rust code
        let output_file = output_dir.join(format!(
            "{}.rs",
            file.file_stem().unwrap().to_string_lossy()
        ));
        let mut rust_code = fs::read_to_string(&output_file)?;

        // Add #[test] when codegen did not; skip when auto-test attribute already emitted.
        for test in file_tests.iter() {
            let already_marked = [
                format!("#[test]\npub fn {}()", test.name),
                format!("#[test]\nfn {}()", test.name),
                format!("#[test]\n#[inline]\npub fn {}()", test.name),
                format!("#[test]\n#[inline]\nfn {}()", test.name),
            ]
            .iter()
            .any(|pat| rust_code.contains(pat));
            if already_marked {
                continue;
            }
            for sig in [
                format!("pub fn {}()", test.name),
                format!("fn {}()", test.name),
            ] {
                if rust_code.contains(&sig) {
                    rust_code = rust_code.replace(&sig, &format!("#[test]\n{}", sig));
                    break;
                }
            }
        }

        // Write back
        fs::write(&output_file, rust_code)?;
    }

    // Detect and compile the library if it exists
    let library_dependency = detect_and_compile_library(project_root, output_dir)?;

    // Tests compiled above still contain `use crate::...`; point them at the library crate.
    if let Some((lib_crate_name, _, _)) = &library_dependency {
        for file in tests_by_file.keys() {
            let output_file = output_dir.join(format!(
                "{}.rs",
                file.file_stem().unwrap().to_string_lossy()
            ));
            if output_file.exists() {
                let rust_code = fs::read_to_string(&output_file)?;
                let rewritten = rewrite_test_crate_imports(&rust_code, lib_crate_name);
                fs::write(&output_file, rewritten)?;
            }
        }
    }

    let _ = crate::rust_integration_tests::sync_rust_integration_tests(project_root);

    // TDD FIX: Copy windjammer-runtime to test directory so tests can find it
    // THE WINDJAMMER WAY: Self-contained test environments
    let windjammer_runtime_path = find_windjammer_runtime_path()?;
    let test_runtime_path = output_dir.join("crates").join("windjammer-runtime");

    // Create crates directory and copy windjammer-runtime
    use colored::*;
    println!(
        "   {} Copying windjammer-runtime to test directory",
        "→".bright_blue().bold()
    );
    println!(
        "   {} Source: {}",
        "→".bright_blue().bold(),
        windjammer_runtime_path.display()
    );
    println!(
        "   {} Dest: {}",
        "→".bright_blue().bold(),
        test_runtime_path.display()
    );
    fs::create_dir_all(output_dir.join("crates"))
        .map_err(|e| anyhow::anyhow!("Failed to create crates directory: {}", e))?;

    if !windjammer_runtime_path.exists() {
        anyhow::bail!(
            "windjammer-runtime source path does not exist: {}",
            windjammer_runtime_path.display()
        );
    }

    copy_dir_recursive(&windjammer_runtime_path, &test_runtime_path)
        .map_err(|e| anyhow::anyhow!("Failed to copy windjammer-runtime: {}", e))?;

    // TDD FIX: Patch windjammer-runtime's Cargo.toml to remove workspace inheritance
    // When copied to a temp directory, there's no workspace root, so all fields must be explicit
    let runtime_cargo_toml = test_runtime_path.join("Cargo.toml");
    if runtime_cargo_toml.exists() {
        let content = fs::read_to_string(&runtime_cargo_toml)?;
        // Replace all workspace-inherited fields with explicit values
        let patched = content
            .replace("version.workspace = true", "version = \"0.1.0\"")
            .replace("version = { workspace = true }", "version = \"0.1.0\"")
            .replace("edition.workspace = true", "edition = \"2021\"")
            .replace("edition = { workspace = true }", "edition = \"2021\"")
            .replace("authors.workspace = true", "authors = []")
            .replace("authors = { workspace = true }", "authors = []")
            .replace("license.workspace = true", "license = \"MIT\"")
            .replace("license = { workspace = true }", "license = \"MIT\"");
        fs::write(&runtime_cargo_toml, patched)?;
    }

    println!(
        "   {} windjammer-runtime copied successfully",
        "✓".green().bold()
    );

    let library_dep_str =
        if let Some((lib_crate_name, lib_package_name, lib_path)) = library_dependency {
            format!(
                "\n{} = {{ path = \"{}\", package = \"{}\" }}",
                lib_crate_name,
                path_to_toml_string(&lib_path),
                lib_package_name
            )
        } else {
            String::new()
        };

    let cargo_toml = format!(
        r#"[package]
name = "windjammer-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
windjammer-runtime = {{ path = "crates/windjammer-runtime", default-features = false }}
smallvec = "1.13"{}

[lib]
name = "windjammer_tests"
path = "lib.rs"
"#,
        library_dep_str
    );
    fs::write(output_dir.join("Cargo.toml"), cargo_toml)?;

    // Create lib.rs that includes all test modules
    let mut lib_rs = String::from("// Auto-generated test harness\n\n");
    for (file, _) in tests_by_file {
        let module_name = file.file_stem().unwrap().to_string_lossy();
        lib_rs.push_str(&format!("pub mod {};\n", module_name));
    }
    fs::write(output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

#[cfg(test)]
mod rewrite_import_tests {
    use super::rewrite_test_crate_imports;

    #[test]
    fn rewrites_use_crate_paths_to_library_crate() {
        let input = "use crate::domain::account::Account;\nuse crate::application::reports::build_trial_balance_report;";
        let out = rewrite_test_crate_imports(input, "foobar_api");
        assert!(out.contains("use foobar_api::domain::account::Account;"));
        assert!(out.contains("use foobar_api::application::reports::build_trial_balance_report;"));
        assert!(!out.contains("use crate::"));
    }

    #[test]
    fn rewrites_pub_use_crate_paths() {
        let input = "pub use crate::domain::*;";
        let out = rewrite_test_crate_imports(input, "test_lib");
        assert_eq!(out, "pub use test_lib::domain::*;");
    }

    #[test]
    fn leaves_non_crate_imports_unchanged() {
        let input = "use windjammer_runtime::test;\nuse std::collections::HashMap;";
        let out = rewrite_test_crate_imports(input, "foobar_api");
        assert_eq!(out, input);
    }
}
