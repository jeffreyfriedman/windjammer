//! Compiling the project under test, FFI wiring, and generating the Rust test harness crate.

use crate::{build_project, build_project_ext, CompilationTarget};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::options::TestRunOptions;
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

/// Read `[lib] name` from a Cargo.toml file, if present.
fn read_cargo_lib_name(cargo_toml_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml_path).ok()?;
    let lib_start = content.find("[lib]")?;
    let name_start = content[lib_start..].find("name = \"")? + lib_start + "name = \"".len();
    let name_end = content[name_start..].find('"')?;
    Some(content[name_start..name_start + name_end].to_string())
}

/// Read `[package] name` from a Cargo.toml file, if present.
fn read_cargo_package_name(cargo_toml_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml_path).ok()?;
    let pkg_start = content.find("[package]")?;
    let name_start = content[pkg_start..].find("name = \"")? + pkg_start + "name = \"".len();
    let name_end = content[name_start..].find('"')?;
    Some(content[name_start..name_start + name_end].to_string())
}

fn resolve_dependency_path(project_root: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    project_root
        .join(p)
        .canonicalize()
        .unwrap_or_else(|_| project_root.join(p))
}

/// Use a pre-built outbound tree (e.g. `build/`) as the library under test.
/// Detect `[lib] path = "build/lib.rs"` dogfood layout and reuse a pre-built tree.
///
/// Returns `(crate_name, package_name, cargo_dep_path, metadata_dir)`.
/// For dogfood, Cargo depends on the project root while signatures load from `build/`.
fn resolve_prebuilt_library(
    build_dir: &Path,
    project_root: &Path,
) -> Result<Option<(String, String, PathBuf, PathBuf)>> {
    use colored::*;

    let build_dir = if build_dir.is_absolute() {
        build_dir.to_path_buf()
    } else {
        project_root.join(build_dir)
    };

    let lib_rs = build_dir.join("lib.rs");
    if !lib_rs.exists() {
        anyhow::bail!(
            "--use-build-dir {} has no lib.rs — run `wj build --library --module-file -o {}` first",
            build_dir.display(),
            build_dir.display()
        );
    }

    let build_cargo = build_dir.join("Cargo.toml");
    let project_cargo = project_root.join("Cargo.toml");

    // Dogfood layout: project root Cargo.toml is source of truth for crate identity;
    // [lib].path often points at `build/lib.rs`.
    let lib_name = read_cargo_lib_name(&project_cargo)
        .or_else(|| read_cargo_lib_name(&build_cargo))
        .or_else(|| {
            project_root
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.replace('-', "_"))
        })
        .unwrap_or_else(|| "lib".to_string());

    let package_name = read_cargo_package_name(&project_cargo)
        .or_else(|| read_cargo_package_name(&build_cargo))
        .unwrap_or_else(|| lib_name.replace('_', "-"));

    // When the project manifest owns the crate, depend from project root so `path = "build/lib.rs"` works.
    let cargo_dep_path = if project_cargo.exists()
        && read_cargo_lib_name(&project_cargo).is_some()
    {
        project_root.to_path_buf()
    } else {
        build_dir.clone()
    };

    // Signatures / `emitted_rust_ref_params` live next to generated `lib.rs`.
    let metadata_dir = build_dir.clone();

    println!(
        "   {} Using pre-built library at {} (lib: {}, dep path: {}, metadata: {})",
        "→".bright_blue().bold(),
        build_dir.display(),
        lib_name,
        cargo_dep_path.display(),
        metadata_dir.display()
    );

    Ok(Some((lib_name, package_name, cargo_dep_path, metadata_dir)))
}

/// Merge `[dependencies]` from the project root Cargo.toml into generated library Cargo.toml.
fn merge_project_cargo_dependencies(project_root: &Path, cargo_toml: &mut String) -> Result<()> {
    let project_cargo = project_root.join("Cargo.toml");
    if !project_cargo.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&project_cargo)?;
    let parsed: toml::Value = toml::from_str(&content)?;
    let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_table()) else {
        return Ok(());
    };

    for (dep_name, dep_spec) in deps {
        if dep_name == "windjammer-runtime" {
            continue;
        }

        // Remove any existing line/table for this dependency.
        remove_dependency_entry(cargo_toml, dep_name);

        let dep_line = format_dependency_line(dep_name, dep_spec, project_root);
        insert_dependency_line(cargo_toml, &dep_line);
    }

    Ok(())
}

fn remove_dependency_entry(cargo_toml: &mut String, dep_name: &str) {
    let prefix = format!("{} =", dep_name);
    let mut out = String::new();
    let mut skip_multiline = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if skip_multiline {
            if trimmed.ends_with('}') {
                skip_multiline = false;
            }
            continue;
        }
        if trimmed.starts_with(&prefix) {
            if trimmed.contains('{') && !trimmed.contains('}') {
                skip_multiline = true;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    *cargo_toml = out;
}

fn format_dependency_line(dep_name: &str, dep_spec: &toml::Value, project_root: &Path) -> String {
    if let Some(version) = dep_spec.as_str() {
        return format!("{} = \"{}\"", dep_name, version);
    }
    if let Some(table) = dep_spec.as_table() {
        let mut parts = Vec::new();
        if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
            parts.push(format!("version = \"{}\"", version));
        }
        if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
            let abs_path = resolve_dependency_path(project_root, path);
            parts.push(format!("path = \"{}\"", path_to_toml_string(&abs_path)));
        }
        if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
            parts.push(format!("git = \"{}\"", git));
        }
        if let Some(branch) = table.get("branch").and_then(|v| v.as_str()) {
            parts.push(format!("branch = \"{}\"", branch));
        }
        if let Some(default_features) = table.get("default-features") {
            parts.push(format!("default-features = {}", default_features));
        }
        if let Some(features) = table.get("features") {
            if let Some(arr) = features.as_array() {
                let feature_list: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| format!("\"{}\"", s))
                    .collect();
                if !feature_list.is_empty() {
                    parts.push(format!("features = [{}]", feature_list.join(", ")));
                }
            } else if let Ok(features_str) = toml::to_string(features) {
                parts.push(format!("features = {}", features_str.trim()));
            }
        }
        if parts.is_empty() {
            return String::new();
        }
        return format!("{} = {{ {} }}", dep_name, parts.join(", "));
    }
    String::new()
}

fn insert_dependency_line(cargo_toml: &mut String, dep_line: &str) {
    if dep_line.is_empty() {
        return;
    }
    if let Some(deps_pos) = cargo_toml.find("[dependencies]") {
        let after = deps_pos + "[dependencies]".len();
        if let Some(next_section) = cargo_toml[after..].find("\n[") {
            cargo_toml.insert_str(after + next_section, &format!("\n{}", dep_line));
        } else {
            cargo_toml.push_str(&format!("\n{}\n", dep_line));
        }
    } else if let Some(lib_pos) = cargo_toml.find("[lib]") {
        cargo_toml.insert_str(
            lib_pos,
            &format!("[dependencies]\n{}\n\n", dep_line),
        );
    }
}

fn project_cargo_has_dep(dep_name: &str, project_root: &Path) -> bool {
    let project_cargo = project_root.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&project_cargo) else {
        return false;
    };
    content.contains(&format!("{} =", dep_name))
}

/// Detect and compile the library being tested (if it exists)
/// Returns `(crate_name, package_name, cargo_dep_path, metadata_dir)`, or None
fn detect_and_compile_library(
    project_root: &Path,
    test_output_dir: &Path,
    opts: &TestRunOptions,
) -> Result<Option<(String, String, PathBuf, PathBuf)>> {
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

    if let Some(ref build_dir) = opts.use_build_dir {
        return resolve_prebuilt_library(build_dir, project_root);
    }

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
    let lib_output_dir = opts
        .output
        .as_ref()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                project_root.join(p)
            }
        })
        .unwrap_or_else(|| test_output_dir.join("lib"));
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
    // Never point at a missing mod.wj (e.g. `--module-file` on a flat lib.wj package).
    let build_entry = if src_dir.join("mod.wj").exists() {
        src_dir.join("mod.wj")
    } else {
        src_dir.clone()
    };

    crate::cargo_toml::set_skip_cargo_toml_generation(opts.no_generate_cargo_toml);
    match build_project_ext(
        &build_entry,
        &lib_output_dir,
        CompilationTarget::Rust,
        true,
        opts.library,
        &[],
    ) {
        Ok(_) => {
            crate::build_utils::apply_library_build_post_steps(
                &build_entry,
                &lib_output_dir,
                opts.library,
                opts.module_file,
            )
            .context("library post-build steps failed")?;

            // Generate lib.rs when module-file layout did not already produce one.
            if !lib_output_dir.join("lib.rs").exists() {
                if let Err(e) = generate_lib_rs_for_library(&lib_output_dir) {
                    eprintln!("WARNING: Failed to generate lib.rs: {}", e);
                }
            }

            if let Err(e) = copy_ffi_files_to_test_library(project_root, &lib_output_dir) {
                eprintln!("WARNING: Failed to copy FFI files: {}", e);
            }

            let project_cargo_toml = project_root.join("Cargo.toml");
            let test_lib_name = read_cargo_lib_name(&project_cargo_toml)
                .unwrap_or_else(|| lib_name.replace('-', "_"));
            let test_lib_package_name = read_cargo_package_name(&project_cargo_toml)
                .unwrap_or_else(|| lib_name.replace('_', "-"));

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
                                    registry: _,
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
                                    // Default desktop only when no features and project Cargo does not already specify this dep.
                                    if dep_name == "windjammer-ui"
                                        && !features.is_some()
                                        && !opts.use_project_cargo
                                        && !project_cargo_has_dep("windjammer-ui", project_root)
                                    {
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

                    if opts.use_project_cargo {
                        merge_project_cargo_dependencies(project_root, &mut cargo_toml)?;
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
            Ok(Some((
                test_lib_name,
                test_lib_package_name,
                lib_output_dir.clone(),
                lib_output_dir,
            )))
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

/// Generate Rust test harness from Windjammer tests
pub(crate) fn generate_test_harness(
    output_dir: &Path,
    tests: &[TestFunction],
    filter: Option<&str>,
    project_root: &Path,
    opts: &TestRunOptions,
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

    // Compile the library first so test codegen can load `emitted_rust_ref_params`
    // from metadata.json (otherwise format! temps miss `&` for demoted `&str` formals).
    let library_dependency = detect_and_compile_library(project_root, output_dir, opts)?;

    // Compile each test file using the existing infrastructure
    for (file, file_tests) in &tests_by_file {
        // Skip if filter doesn't match
        if let Some(filter_str) = filter {
            if !file_tests.iter().any(|t| t.name.contains(filter_str)) {
                continue;
            }
        }

        // Compile the file to Rust — pass library metadata when available.
        if let Some((lib_crate_name, _, _, meta_dir)) = &library_dependency {
            let meta = [(lib_crate_name.as_str(), meta_dir.as_path())];
            build_project_ext(
                file,
                output_dir,
                CompilationTarget::Rust,
                false,
                false,
                &meta,
            )?;
        } else {
            build_project(file, output_dir, CompilationTarget::Rust, false)?;
        }

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

    // Tests compiled above still contain `use crate::...`; point them at the library crate.
    if let Some((lib_crate_name, _, _, _)) = &library_dependency {
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

    let runtime_path = opts
        .runtime_path
        .clone()
        .unwrap_or_else(crate::cargo_toml::find_windjammer_runtime_path);

    let runtime_dep_line = if opts.no_runtime_copy {
        format!(
            "windjammer-runtime = {{ path = \"{}\", default-features = false }}",
            path_to_toml_string(&runtime_path)
        )
    } else {
        let test_runtime_path = output_dir.join("crates").join("windjammer-runtime");

        use colored::*;
        println!(
            "   {} Copying windjammer-runtime to test directory",
            "→".bright_blue().bold()
        );
        fs::create_dir_all(output_dir.join("crates"))
            .map_err(|e| anyhow::anyhow!("Failed to create crates directory: {}", e))?;

        if !runtime_path.exists() {
            anyhow::bail!(
                "windjammer-runtime source path does not exist: {}",
                runtime_path.display()
            );
        }

        copy_dir_recursive(&runtime_path, &test_runtime_path)
            .map_err(|e| anyhow::anyhow!("Failed to copy windjammer-runtime: {}", e))?;

        let runtime_cargo_toml = test_runtime_path.join("Cargo.toml");
        if runtime_cargo_toml.exists() {
            let content = fs::read_to_string(&runtime_cargo_toml)?;
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

        "windjammer-runtime = { path = \"crates/windjammer-runtime\", default-features = false }"
            .to_string()
    };

    let library_dep_str =
        if let Some((lib_crate_name, lib_package_name, lib_path, _)) = library_dependency {
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
{}
smallvec = "1.13"{}

[lib]
name = "windjammer_tests"
path = "lib.rs"
"#,
        runtime_dep_line,
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
        let input = "use crate::domain::item::Item;\nuse crate::application::reports::build_summary_report;";
        let out = rewrite_test_crate_imports(input, "foobar_api");
        assert!(out.contains("use foobar_api::domain::item::Item;"));
        assert!(out.contains("use foobar_api::application::reports::build_summary_report;"));
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
