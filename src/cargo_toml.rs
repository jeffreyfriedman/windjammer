//! Cargo.toml generation for single-file and multi-file Windjammer builds.
//!
//! TDD FIX (Bug #2): Detect test files and generate [[bin]]/[[test]] targets.
//! Used by compiler for single-file builds (wj CLI uses this path).

use crate::compiler::write_if_changed;
use crate::CompilationTarget;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Find windjammer-runtime path for Cargo.toml dependency
fn find_windjammer_runtime_path() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let original_cwd = current.clone();

    // Try current directory first (if we're in windjammer repo)
    if current
        .join("crates/windjammer-runtime/Cargo.toml")
        .exists()
    {
        return current.join("crates/windjammer-runtime");
    }

    // Also check windjammer/ subdirectory (running from workspace root)
    if current
        .join("windjammer/crates/windjammer-runtime/Cargo.toml")
        .exists()
    {
        return current.join("windjammer/crates/windjammer-runtime");
    }

    // Search upward (up to 5 levels)
    for _ in 0..5 {
        if let Some(parent) = current.parent() {
            if parent
                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
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

    // Try the compiler's own location (the wj binary is in windjammer/target/release/)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut search = exe_dir.to_path_buf();
            for _ in 0..5 {
                if search.join("crates/windjammer-runtime/Cargo.toml").exists() {
                    return search.join("crates/windjammer-runtime");
                }
                if let Some(p) = search.parent() {
                    search = p.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // Fallback: relative paths
    let sibling = original_cwd.join("../windjammer/crates/windjammer-runtime");
    if sibling.join("Cargo.toml").exists() {
        return sibling;
    }
    PathBuf::from("./crates/windjammer-runtime")
}

/// Convert path to string suitable for Cargo.toml (absolute, forward-slash, no Windows \\?\ prefix)
fn path_to_toml_string(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    sanitize_path_for_toml(&abs)
}

/// Sanitize a path for use in TOML string values:
/// - Strip Windows extended-length prefix (\\?\)
/// - Convert backslashes to forward slashes (valid in Cargo.toml on all platforms)
fn sanitize_path_for_toml(path: &Path) -> String {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
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

fn write_cargo_toml(output_dir: &Path, source_dir: &Path, lib_or_bin_section: &str) -> Result<()> {
    let runtime_path = find_windjammer_runtime_path();
    let runtime_path_str = path_to_toml_string(&runtime_path);

    let mut deps = vec![
        format!("windjammer-runtime = {{ path = \"{}\" }}", runtime_path_str),
        "smallvec = \"1.13\"".to_string(),
        "serde = { version = \"1.0\", features = [\"derive\"] }".to_string(),
    ];

    // Detect external crate imports from generated Rust source files
    let external_deps = detect_external_crate_deps(output_dir, source_dir);
    deps.extend(external_deps);

    // Propagate dependencies from source project's Cargo.toml (FFI deps, etc.)
    let propagated = propagate_source_cargo_deps(source_dir, &deps);
    deps.extend(propagated);

    let deps_section = format!("[dependencies]\n{}\n\n", deps.join("\n"));

    let project_name = infer_project_name(source_dir);
    let inferred_snake = project_name.replace('-', "_");
    let package_name = resolve_package_name_with_existing_cargo(output_dir, &inferred_snake);

    let cargo_toml = format!(
        r#"# Auto-generated by Windjammer compiler - do not edit manually
[package]
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
    write_if_changed(&cargo_toml_path, &cargo_toml)?;

    Ok(())
}

/// Relative path to the crate root Rust file for a `cdylib` WASM build.
fn resolve_wasm_lib_path(output_dir: &Path) -> Result<String> {
    if output_dir.join("lib.rs").exists() {
        return Ok("lib.rs".to_string());
    }
    if output_dir.join("mod.rs").exists() {
        return Ok("mod.rs".to_string());
    }

    let mut top_level: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name != "main.rs" {
                    top_level.push(p);
                }
            }
        }
    }
    top_level.sort();
    if let Some(p) = top_level.first() {
        return Ok(p
            .file_name()
            .expect("wasm lib path")
            .to_string_lossy()
            .into_owned());
    }

    let all = walk_rs_files(output_dir)?;
    if all.len() == 1 {
        let rel = all[0].strip_prefix(output_dir)?;
        return Ok(rel.to_string_lossy().replace('\\', "/"));
    }
    for p in &all {
        if p.file_name().and_then(|s| s.to_str()) == Some("lib.rs") {
            let rel = p.strip_prefix(output_dir)?;
            return Ok(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    if all.is_empty() {
        anyhow::bail!("No Rust sources found under output dir for WASM Cargo.toml");
    }
    anyhow::bail!(
        "Cannot pick a single WASM library entry point among {} Rust files",
        all.len()
    );
}

/// Whether generated Rust needs `windjammer-runtime` (WASM feature), mirroring `main.rs` `create_wasm_cargo_toml`.
fn wasm_output_needs_runtime(output_dir: &Path) -> Result<bool> {
    for path in walk_rs_files(output_dir)? {
        let content = fs::read_to_string(&path)?;
        if content.contains("windjammer_runtime") {
            return Ok(true);
        }
        // Platform modules routed through runtime (same heuristic as CLI)
        if content.contains("fs::")
            || content.contains("process::")
            || content.contains("dialog::")
            || content.contains("env::")
            || content.contains("encoding::")
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Generate `Cargo.toml` for `--target wasm` builds produced by `compiler::build_project_ext`.
///
/// WASM output is Rust (`cdylib`); this matches `create_wasm_cargo_toml` in `main.rs` and the
/// `WasmBackend::generate_additional_files` template, but uses the same runtime path discovery
/// as single-file Rust builds.
pub fn generate_wasm_cargo_toml(output_dir: &Path, source_dir: &Path) -> Result<()> {
    let lib_rel = resolve_wasm_lib_path(output_dir)?;
    let needs_runtime = wasm_output_needs_runtime(output_dir)?;
    let runtime_line = if needs_runtime {
        let runtime_path = find_windjammer_runtime_path();
        let runtime_str = path_to_toml_string(&runtime_path);
        format!(
            "windjammer-runtime = {{ path = \"{}\", features = [\"wasm\"] }}\n",
            runtime_str
        )
    } else {
        String::new()
    };

    let extra_deps = detect_external_crate_deps(output_dir, source_dir);

    let smallvec_line = "smallvec = \"1.13\"\n";
    let extra_section = if extra_deps.is_empty() {
        String::new()
    } else {
        format!("{}\n", extra_deps.join("\n"))
    };

    let project_snake = infer_project_name(source_dir).replace('-', "_");
    let package_name =
        resolve_package_name_with_existing_cargo(output_dir, &format!("{project_snake}_wasm"));

    let cargo_toml = format!(
        r#"# Auto-generated by Windjammer compiler - do not edit manually
[package]
name = "{pkg}"
version = "0.1.0"
edition = "2021"

# Prevent this from being treated as part of parent workspace
[workspace]

[lib]
crate-type = ["cdylib"]
path = "{lib_rel}"

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
{smallvec}{runtime}{extra}
[profile.release]
opt-level = "z"
lto = true
"#,
        pkg = package_name,
        lib_rel = lib_rel,
        smallvec = smallvec_line,
        runtime = runtime_line,
        extra = extra_section,
    );

    write_if_changed(&output_dir.join("Cargo.toml"), &cargo_toml)?;
    Ok(())
}

/// Reads `[package] name = "..."` from existing generated `Cargo.toml` in `output_dir`.
fn read_package_name_from_package_section(content: &str) -> Option<String> {
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed != "[package]" {
            in_package = false;
        }
        if in_package && trimmed.starts_with("name") {
            let rest = trimmed.strip_prefix("name")?.trim_start();
            let rest = rest.strip_prefix('=')?.trim();
            let value = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"'))?;
            return Some(value.to_string());
        }
    }
    None
}

/// Prefer a non-placeholder name already present in `output_dir/Cargo.toml` so repeated
/// `wj build` does not reset `name` to `windjammer`. A stale `windjammer` entry is ignored
/// so `wj.toml` (via `inferred_snake`) can replace it.
fn resolve_package_name_with_existing_cargo(output_dir: &Path, inferred_snake: &str) -> String {
    let existing_cargo = output_dir.join("Cargo.toml");
    if !existing_cargo.exists() {
        return inferred_snake.to_string();
    }
    let Ok(content) = fs::read_to_string(&existing_cargo) else {
        return inferred_snake.to_string();
    };
    match read_package_name_from_package_section(&content) {
        Some(name) if name != "windjammer" => name.replace('-', "_"),
        _ => inferred_snake.to_string(),
    }
}

/// Read source project's Cargo.toml and propagate dependencies that aren't already present.
/// This ensures FFI dependencies (wgpu, bytemuck, rapier3d, etc.) are available in the
/// generated build's Cargo.toml without requiring the user to manually copy them.
fn propagate_source_cargo_deps(source_dir: &Path, existing_deps: &[String]) -> Vec<String> {
    use std::collections::HashSet;

    let skip_crates: HashSet<&str> = ["windjammer", "windjammer-runtime", "windjammer_runtime"]
        .into_iter()
        .collect();

    let candidates = [
        source_dir.join("Cargo.toml"),
        source_dir
            .parent()
            .map(|p| p.join("Cargo.toml"))
            .unwrap_or_default(),
    ];

    let cargo_path = candidates.iter().find(|p| p.exists());
    let cargo_path = match cargo_path {
        Some(p) => p,
        None => return Vec::new(),
    };

    let content = match fs::read_to_string(cargo_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut propagated = Vec::new();
    let mut in_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }

        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let dep_name = match trimmed.split(['=', ' ']).next() {
            Some(n) => n.trim(),
            None => continue,
        };

        if dep_name.is_empty() {
            continue;
        }

        if skip_crates.contains(dep_name) {
            continue;
        }

        let already_present = existing_deps
            .iter()
            .any(|d| d.starts_with(dep_name) || d.starts_with(&dep_name.replace('-', "_")));

        if !already_present {
            propagated.push(trimmed.to_string());
        }
    }

    propagated
}

/// Scan generated .rs files for `use <crate>::...` imports and resolve crate paths.
fn detect_external_crate_deps(output_dir: &Path, source_dir: &Path) -> Vec<String> {
    use std::collections::HashSet;

    let builtin_crates: HashSet<&str> = [
        "std",
        "core",
        "alloc",
        "crate",
        "self",
        "super",
        "windjammer_runtime",
        "windjammer",
        "serde",
        "serde_core",
        "smallvec",
        "glob",
        "typenum",
        "bytemuck",
    ]
    .into_iter()
    .collect();

    let mut external_crates: HashSet<String> = HashSet::new();

    if let Ok(entries) = walk_rs_files(output_dir) {
        for path in entries {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("use ") {
                        if let Some(crate_name) = rest.split("::").next() {
                            let crate_name = crate_name.trim().trim_start_matches('{');
                            if !crate_name.is_empty()
                                && !builtin_crates.contains(crate_name)
                                && crate_name.chars().next().is_some_and(|c| c.is_alphabetic())
                            {
                                external_crates.insert(crate_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let mut deps = Vec::new();
    for crate_name in &external_crates {
        if let Some(dep_line) = resolve_crate_path(crate_name, source_dir, output_dir) {
            deps.push(dep_line);
        }
    }
    deps
}

fn walk_rs_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, files);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
    }
    walk(dir, &mut files);
    Ok(files)
}

/// Attempt to find the crate's path on disk for path-based dependency.
///
/// Searches upward from `source_dir` (up to 5 levels) and checks each
/// ancestor's subdirectories (up to 2 levels deep) for a matching
/// `Cargo.toml`. Uses absolute paths so relative `source_dir` values
/// (e.g. `"src"`) don't limit traversal depth. Skips any match that
/// points to `output_dir` itself to prevent cyclic self-dependencies.
fn resolve_crate_path(crate_name: &str, source_dir: &Path, output_dir: &Path) -> Option<String> {
    let hyphenated = crate_name.replace('_', "-");

    let abs_source = source_dir.canonicalize().unwrap_or_else(|_| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(source_dir)
    });

    let mut current = abs_source.clone();
    for _ in 0..5 {
        if let Some(found) = try_find_crate_at(&current, &hyphenated, crate_name, output_dir) {
            return Some(found);
        }
        if !current.pop() {
            break;
        }
    }

    None
}

/// Check `root` and its immediate subdirectories for `<hyphenated>/Cargo.toml`.
fn try_find_crate_at(
    root: &Path,
    hyphenated: &str,
    crate_name: &str,
    output_dir: &Path,
) -> Option<String> {
    // Direct child: <root>/<hyphenated>/Cargo.toml
    if let Some(dep) = check_cargo_toml(&root.join(hyphenated), crate_name, output_dir) {
        return Some(dep);
    }
    // Compiled output variant: <root>/<hyphenated>/src/Cargo.toml
    let src_variant = root.join(hyphenated).join("src");
    if let Some(dep) = check_cargo_toml(&src_variant, crate_name, output_dir) {
        return Some(dep);
    }
    // One level deeper: <root>/*/<hyphenated>/Cargo.toml
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                if let Some(dep) = check_cargo_toml(&child.join(hyphenated), crate_name, output_dir)
                {
                    return Some(dep);
                }
            }
        }
    }
    None
}

/// If `crate_dir/Cargo.toml` exists, produce a dependency line for it.
/// `output_dir` is the directory we're generating into — we must never add
/// a dependency that points back to it (that would create a cyclic package).
fn check_cargo_toml(crate_dir: &Path, crate_name: &str, output_dir: &Path) -> Option<String> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }
    let abs = crate_dir
        .canonicalize()
        .unwrap_or_else(|_| crate_dir.to_path_buf());
    let abs_output = output_dir
        .canonicalize()
        .unwrap_or_else(|_| output_dir.to_path_buf());
    if abs == abs_output {
        return None;
    }
    let actual_pkg = read_package_name(&cargo_toml).unwrap_or_else(|| crate_name.to_string());
    let hyphenated = crate_name.replace('_', "-");
    let path_str = sanitize_path_for_toml(&abs);
    if actual_pkg.replace('-', "_") != crate_name {
        Some(format!(
            "{} = {{ path = \"{}\", package = \"{}\" }}",
            crate_name, path_str, actual_pkg
        ))
    } else {
        Some(format!("{} = {{ path = \"{}\" }}", hyphenated, path_str))
    }
}

fn read_package_name(cargo_toml_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            return trimmed.split('"').nth(1).map(|s| s.to_string());
        }
    }
    None
}

/// Infer the project name from `wj.toml` or `game.toml` (legacy), falling back to directory name.
/// Public so other modules (e.g. `main.rs` WASM Cargo.toml generation) can reuse this logic.
pub fn infer_project_name_from(source_dir: &Path) -> String {
    infer_project_name(source_dir)
}

fn infer_project_name(source_dir: &Path) -> String {
    // Check wj.toml, then game.toml, in source_dir and parent
    let config_files = ["wj.toml", "game.toml"];
    let dirs_to_check: Vec<&Path> = {
        let mut v = vec![source_dir];
        if let Some(parent) = source_dir.parent() {
            v.push(parent);
        }
        v
    };

    for dir in &dirs_to_check {
        for config_name in &config_files {
            let config_path = dir.join(config_name);
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Some(name) = extract_package_name_from_toml(&content) {
                        return name;
                    }
                }
            }
        }
    }

    // Fallback: use directory name instead of hardcoding "windjammer"
    if let Some(dir_name) = source_dir.file_name().and_then(|n| n.to_str()) {
        if dir_name != "src" && dir_name != "src_wj" {
            return dir_name.to_lowercase().replace(' ', "-");
        }
        // If source_dir is "src" or "src_wj", use the parent directory name
        if let Some(parent) = source_dir.parent() {
            if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                return parent_name.to_lowercase().replace(' ', "-");
            }
        }
    }

    "windjammer".to_string()
}

/// Extract `name = "..."` from a TOML file.
/// Supports both `[package]\nname = "..."` (wj.toml) and flat `name = "..."` (game.toml).
fn extract_package_name_from_toml(content: &str) -> Option<String> {
    let mut in_package = false;
    let mut found_any_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            found_any_section = true;
            in_package = trimmed == "[package]";
            continue;
        }
        if trimmed.starts_with("name") {
            // Accept if we're in [package] section, or if there are no sections at all (flat format)
            if in_package || !found_any_section {
                return trimmed
                    .split('"')
                    .nth(1)
                    .map(|s| s.to_lowercase().replace(' ', "-"));
            }
        }
    }
    None
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
        fs::write(temp.join("game.toml"), r#"name = "Breach Protocol""#).unwrap();

        assert_eq!(infer_project_name(&temp), "breach-protocol");

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
name = "breach_protocol"
version = "0.1.0"
"#,
        )
        .unwrap();

        assert_eq!(
            resolve_package_name_with_existing_cargo(&temp, "windjammer"),
            "breach_protocol"
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
            resolve_package_name_with_existing_cargo(&temp, "breach_protocol"),
            "breach_protocol"
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
name = "breach_protocol"
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
            cargo.contains("name = \"breach_protocol\""),
            "package name should be preserved: {}",
            cargo
        );

        fs::remove_dir_all(&out).ok();
        fs::remove_dir_all(&src).ok();
    }
}
