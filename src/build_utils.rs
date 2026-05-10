//! Build utilities for CLI - generate_mod_file, strip_main_functions.
//!
//! Extracted from main.rs for use by cli/build.rs when the cli feature is enabled.

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

/// True when a module name represents a test module that should be gated with `#[cfg(test)]`.
fn is_test_module(name: &str) -> bool {
    name == "tests"
        || name == "test_runtime"
        || name == "test_output"
        || name == "test_plugins"
        || name == "tests_build"
        || name.ends_with("_test")
        || name.ends_with("_tests")
        || name.starts_with("test_")
}

/// True when the output directory is a Rust module subtree under `src/...` (not the crate root).
/// In that case we only emit `mod.rs` for Cargo, not `lib.rs` (which would be invalid as a nested crate root).
fn is_submodule_output_dir(output_dir: &Path) -> bool {
    let comps: Vec<_> = output_dir.components().collect();
    for i in 0..comps.len() {
        if comps[i].as_os_str() == "src" && i + 1 < comps.len() {
            return true;
        }
    }
    false
}

/// Copy top-level `*.rs` files and `*/mod.rs` module trees from `<project>/src/` into the output
/// directory when they are not already present. This keeps hand-written Rust under `<project>/src/`
/// discoverable in `out/` without pulling in unrelated trees (output is usually `out/`, not under
/// `src/`).
fn copy_project_src_tree_into_output(output_dir: &Path) -> std::io::Result<()> {
    use std::fs;
    let Some(root) = output_dir.parent() else {
        return Ok(());
    };
    let root = if root.as_os_str().is_empty() {
        Path::new(".")
    } else {
        root
    };
    let src_dir = root.join("src");
    if !src_dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&src_dir)? {
        let entry = entry?;
        let p = entry.path();
        let name = entry.file_name();
        let dest = output_dir.join(&name);
        if p.is_file() {
            if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            // Skip `src/foo.rs` when `src/foo/` exists (Rust's split module layout: parent file +
            // subfolder is one logical module; copying only the `.rs` breaks `out/`).
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                if src_dir.join(stem).is_dir() {
                    continue;
                }
            }
            if !dest.exists() {
                fs::copy(&p, &dest)?;
            }
        } else if p.is_dir() && p.join("mod.rs").exists() {
            copy_dir_merge_shallow(&p, &dest)?;
        }
    }
    Ok(())
}

fn copy_dir_merge_shallow(src: &Path, dst: &Path) -> std::io::Result<()> {
    use std::fs;
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_merge_shallow(&from, &to)?;
        } else if !to.exists() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Copy hand-written sibling `*.rs` from the parent directory into `output_dir` when the file
/// does not already exist in the output (Windjammer-emitted files take precedence).
///
/// This picks up:
/// - `components/platform.rs` next to `components/generated/` (same parent as `generated/`)
/// - `ffi.rs` (or other root modules) next to `out/` when building into a crate output folder
fn copy_sibling_rs_from_parent(output_dir: &Path) -> std::io::Result<()> {
    use std::fs;
    let Some(parent) = output_dir.parent() else {
        return Ok(());
    };
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Avoid hijacking Cargo's build script or entrypoints from the parent folder.
        if matches!(stem, "build" | "main" | "lib") {
            continue;
        }
        let dest = output_dir.join(p.file_name().unwrap());
        if !dest.exists() {
            fs::copy(&p, &dest)?;
        }
    }
    Ok(())
}

/// Parse `pub mod name;` lines from mod.wj (subset of module_system parsing; keeps build_utils free of `module_system`).
fn pub_mod_names_from_mod_wj(content: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("pub mod ") {
            if let Some(name) = trimmed
                .strip_prefix("pub mod ")
                .and_then(|s| s.split_whitespace().next())
            {
                names.insert(name.trim_end_matches(';').to_string());
            }
        }
    }
    names
}

fn mod_declared_in(content: &str, name: &str) -> bool {
    let pub_mod = format!("pub mod {};", name);
    let plain = format!("mod {};", name);
    content.contains(&pub_mod)
        || content.contains(&plain)
        || content.contains(&format!("pub mod {} {{", name))
}

/// When `mod.wj` exists, stub-merge must not re-add stale `.rs` files for modules no longer
/// declared in `mod.wj` (the compiler already regenerated `mod.rs` without them). Hand-written
/// `foo.rs` without `foo.wj` is still merged when `mod.wj` exists but omits `foo` (legacy FFI pattern).
fn should_merge_extra_module(name: &str, sibling_source_dir: Option<&Path>) -> bool {
    let Some(sdir) = sibling_source_dir else {
        return true;
    };
    let mod_wj = sdir.join("mod.wj");
    if !mod_wj.exists() {
        return true;
    }
    let Ok(text) = std::fs::read_to_string(&mod_wj) else {
        return true;
    };
    let declared = pub_mod_names_from_mod_wj(&text);
    // If mod.wj has no explicit pub mod declarations, auto-discover all sibling modules.
    // An empty mod.wj is a directory marker, not an exclusion list.
    if declared.is_empty() {
        return true;
    }
    let wj_for_module = sdir.join(format!("{}.wj", name));
    declared.contains(name) || !wj_for_module.exists()
}

fn source_dir_for_output(
    output_subdir: &Path,
    layout: Option<(&Path, &Path)>,
) -> Option<std::path::PathBuf> {
    let (out_root, src_root) = layout?;
    let rel = output_subdir.strip_prefix(out_root).ok()?;
    Some(src_root.join(rel))
}

/// Generate mod.rs file with pub mod declarations and re-exports.
/// Recursively generates mod.rs for nested subdirectories (e.g., ui/mod.rs, components/mod.rs).
/// For the root directory, generates `lib.rs` if no `lib.rs` exists (to serve as crate root
/// without invalid `use super::*`).
pub fn generate_mod_file(output_dir: &Path) -> Result<()> {
    generate_mod_file_with_layout(output_dir, None)
}

/// Same as [`generate_mod_file`], but when `layout` is `Some((output_root, source_root))`, stub-merge
/// of extra `*.rs` siblings respects `mod.wj` (avoids re-adding removed modules).
pub(crate) fn generate_mod_file_with_layout(
    output_dir: &Path,
    layout: Option<(&Path, &Path)>,
) -> Result<()> {
    copy_project_src_tree_into_output(output_dir)?;
    copy_sibling_rs_from_parent(output_dir)?;
    generate_mod_file_recursive(output_dir, layout)?;

    let mod_rs = output_dir.join("mod.rs");
    let lib_rs = output_dir.join("lib.rs");

    if mod_rs.exists() {
        if is_submodule_output_dir(output_dir) {
            if lib_rs.exists() {
                std::fs::remove_file(&lib_rs)?;
            }
            let cargo_toml_path = output_dir.join("Cargo.toml");
            if cargo_toml_path.exists() {
                if let Ok(toml_content) = std::fs::read_to_string(&cargo_toml_path) {
                    if toml_content.contains("path = \"lib.rs\"") {
                        let updated =
                            toml_content.replace("path = \"lib.rs\"", "path = \"mod.rs\"");
                        std::fs::write(&cargo_toml_path, updated)?;
                    }
                }
            }
        } else {
            // Crate root: derive lib.rs from mod.rs (strip `use super::*` only used for nested modules).
            let content = std::fs::read_to_string(&mod_rs)?;
            let cleaned: String = content
                .lines()
                .filter(|line| {
                    let t = line.trim();
                    t != "use super::*;" && t != "#[allow(unused_imports)]"
                })
                .collect::<Vec<&str>>()
                .join("\n");
            std::fs::write(&lib_rs, cleaned + "\n")?;

            if lib_rs.exists() {
                let cargo_toml_path = output_dir.join("Cargo.toml");
                if cargo_toml_path.exists() {
                    if let Ok(toml_content) = std::fs::read_to_string(&cargo_toml_path) {
                        if toml_content.contains("path = \"mod.rs\"") {
                            let updated =
                                toml_content.replace("path = \"mod.rs\"", "path = \"lib.rs\"");
                            std::fs::write(&cargo_toml_path, updated)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Recursively generate mod.rs for a directory and all its subdirectories.
/// Processes subdirectories first (depth-first) so parent mod.rs can reference child modules.
fn generate_mod_file_recursive(output_dir: &Path, layout: Option<(&Path, &Path)>) -> Result<()> {
    use colored::*;
    use std::fs;

    // Step 1: Recursively generate mod.rs for subdirectories that have .rs files
    let subdirs: Vec<_> = fs::read_dir(output_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n != "target" && n != ".git")
                    .unwrap_or(true)
        })
        .collect();

    for subdir in &subdirs {
        let has_rs_files = fs::read_dir(subdir)
            .ok()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    let p = e.path();
                    p.is_file()
                        && p.extension().and_then(|s| s.to_str()) == Some("rs")
                        && p.file_name().and_then(|n| n.to_str()) != Some("mod.rs")
                })
            })
            .unwrap_or(false);
        let has_rs_subdirs = fs::read_dir(subdir)
            .ok()
            .map(|entries| entries.filter_map(|e| e.ok()).any(|e| e.path().is_dir()))
            .unwrap_or(false);
        if has_rs_files || has_rs_subdirs {
            generate_mod_file_recursive(subdir, layout)?;
        }
    }

    let sibling_src = source_dir_for_output(output_dir, layout);

    // Step 2: If a compiler-generated mod.rs exists (from compiling mod.wj),
    // preserve it but also discover hand-written modules not already declared.
    let existing_mod_rs = output_dir.join("mod.rs");
    let mod_items_path_check = output_dir.join("_mod_items.rs");
    // When _mod_items.rs exists, the compiler just freshly produced mod.wj output.
    // Delete the stale mod.rs so it gets regenerated from scratch below, preventing
    // incremental builds from duplicating trait/struct/impl definitions.
    if mod_items_path_check.exists() && existing_mod_rs.exists() {
        let _ = fs::remove_file(&existing_mod_rs);
    }
    if existing_mod_rs.exists() {
        if let Ok(content) = fs::read_to_string(&existing_mod_rs) {
            let is_auto_generated = content.starts_with("// Auto-generated mod.rs")
                    || content.starts_with("// Module declarations");
            if !is_auto_generated {
                // Clean up stale _mod_items references AND previously-merged
                // mod.wj content from prior builds. Without this, incremental
                // builds would re-append _mod_items.rs content each time,
                // producing duplicate struct/trait/impl definitions.
                let mut filtered_lines = Vec::new();
                let mut in_mod_wj_section = false;
                for l in content.lines() {
                    let t = l.trim();
                    if t == "pub mod _mod_items;"
                        || t == "pub use _mod_items::*;"
                        || t == "mod _mod_items;"
                    {
                        continue;
                    }
                    if t == "// Code from mod.wj (traits, structs, impls)" {
                        in_mod_wj_section = true;
                        continue;
                    }
                    if in_mod_wj_section {
                        continue;
                    }
                    filtered_lines.push(l);
                }
                let content = filtered_lines.join("\n") + "\n";
                // Stub / hand-merged mod.rs: add sibling `foo.rs` modules and `foo/mod.rs` dirs.
                let mut extra_modules = Vec::new();
                for entry in fs::read_dir(output_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            if path.join("mod.rs").exists()
                                && !mod_declared_in(&content, dir_name)
                                && should_merge_extra_module(dir_name, sibling_src.as_deref())
                            {
                                extra_modules.push(dir_name.to_string());
                            }
                        }
                    } else if path.is_file() {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            if file_name.ends_with(".rs")
                                && file_name != "mod.rs"
                                && file_name != "main.rs"
                                && file_name != "lib.rs"
                                && file_name != "_mod_items.rs"
                            {
                                if let Some(module_name) = file_name.strip_suffix(".rs") {
                                    if !mod_declared_in(&content, module_name)
                                        && should_merge_extra_module(
                                            module_name,
                                            sibling_src.as_deref(),
                                        )
                                    {
                                        extra_modules.push(module_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                let mut updated = content.clone();
                if !extra_modules.is_empty() {
                    extra_modules.sort();
                    for m in &extra_modules {
                        if is_test_module(m) {
                            updated.push_str("\n#[cfg(test)]");
                            updated.push_str(&format!("\npub mod {};", m));
                        } else {
                            updated.push_str(&format!("\npub mod {};", m));
                            updated.push_str(&format!("\npub use {}::*;", m));
                        }
                    }
                    updated.push('\n');
                }
                // Merge _mod_items.rs (compiled mod.wj code) into this mod.rs
                let mod_items_path = output_dir.join("_mod_items.rs");
                if mod_items_path.exists() {
                    if let Ok(items_content) = fs::read_to_string(&mod_items_path) {
                        let has_code = items_content.lines().any(|line| {
                            let t = line.trim();
                            !t.is_empty()
                                && !t.starts_with("//")
                                && !t.starts_with("#[")
                                && !t.starts_with("use ")
                                && !(t.starts_with("pub mod ") && t.ends_with(';'))
                                && !(t.starts_with("mod ") && t.ends_with(';'))
                                && !(t.starts_with("pub use ") && t.ends_with("::*;"))
                        });
                        if has_code {
                            updated.push_str("\n// Code from mod.wj (traits, structs, impls)\n");
                            for line in items_content.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("pub mod ") && trimmed.ends_with(';') {
                                    continue;
                                }
                                if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
                                    continue;
                                }
                                if trimmed.starts_with("pub use ") && trimmed.ends_with(';') {
                                    continue;
                                }
                                if trimmed == "#[allow(unused_imports)]" {
                                    continue;
                                }
                                if trimmed == "use super::*;" {
                                    continue;
                                }
                                updated.push_str(line);
                                updated.push('\n');
                            }
                        }
                    }
                    let _ = fs::remove_file(&mod_items_path);
                }
                if updated != content {
                    fs::write(&existing_mod_rs, updated)?;
                }
                return Ok(());
            }
        }
    }

    // Step 3: Generate mod.rs for this directory
    let mut modules = Vec::new();
    let mut type_exports: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs")
                    && file_name != "mod.rs"
                    && file_name != "main.rs"
                    && file_name != "lib.rs"
                    && file_name != "_mod_items.rs"
                {
                    if let Some(module_name) = file_name.strip_suffix(".rs") {
                        modules.push(module_name.to_string());

                        let content = fs::read_to_string(&path)?;
                        let mut exports = Vec::new();

                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("pub struct ") {
                                if let Some(name) = trimmed
                                    .strip_prefix("pub struct ")
                                    .and_then(|s| s.split_whitespace().next())
                                {
                                    exports.push(name.to_string());
                                }
                            } else if trimmed.starts_with("pub enum ") {
                                if let Some(name) = trimmed
                                    .strip_prefix("pub enum ")
                                    .and_then(|s| s.split_whitespace().next())
                                {
                                    exports.push(name.to_string());
                                }
                            }
                        }

                        if !exports.is_empty() {
                            type_exports.insert(module_name.to_string(), exports);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let mod_rs_path = path.join("mod.rs");
                if mod_rs_path.exists() {
                    modules.push(dir_name.to_string());
                }
            }
        }
    }

    if modules.is_empty() {
        return Ok(());
    }

    modules.sort();

    // When mod.wj declares explicit modules, filter out stale .rs files for modules
    // that are no longer declared. Without this, a removed module (e.g. `beta`) stays
    // in mod.rs because its .rs file persists from the previous build.
    modules.retain(|m| should_merge_extra_module(m, sibling_src.as_deref()));

    if modules.is_empty() {
        return Ok(());
    }

    let mut symbol_conflicts: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (module, exports) in &type_exports {
        for symbol in exports {
            symbol_conflicts
                .entry(symbol.clone())
                .or_default()
                .push(module.clone());
        }
    }

    let conflicting_symbols: std::collections::HashSet<String> = symbol_conflicts
        .iter()
        .filter(|(_, modules_list)| modules_list.len() > 1)
        .map(|(symbol, _)| symbol.clone())
        .collect();

    let modules_with_conflicts: std::collections::HashSet<String> = symbol_conflicts
        .iter()
        .filter(|(_, modules_list)| modules_list.len() > 1)
        .flat_map(|(_, modules_list)| modules_list.clone())
        .collect();

    if !conflicting_symbols.is_empty() {
        println!("{} Detected conflicting symbol exports:", "⚠".yellow());
        for (symbol, modules_list) in &symbol_conflicts {
            if modules_list.len() > 1 {
                println!(
                    "  • {} exported by: {}",
                    symbol.cyan(),
                    modules_list.join(", ")
                );
            }
        }
        println!(
            "{} Using selective re-exports for conflicting modules",
            "→".yellow()
        );
    }

    // Pre-read _mod_items.rs to extract selective re-exports from mod.wj.
    // When the user writes `pub use camera2d::Camera2D` in mod.wj, the codegen
    // produces `pub use self::camera2d::Camera2D;`. We use these instead of
    // wildcard re-exports to respect the user's intent.
    let mod_items_path = output_dir.join("_mod_items.rs");
    let mod_items_content = mod_items_path
        .exists()
        .then(|| fs::read_to_string(&mod_items_path).ok())
        .flatten();

    // Collect user-defined re-exports from _mod_items.rs (both wildcard and selective).
    // These come from explicit `pub use` declarations in the user's mod.wj.
    // Skip re-exports of test modules -- they are gated with #[cfg(test)] and
    // would cause unresolved import errors in non-test builds.
    let mut user_reexports: Vec<String> = Vec::new();
    let mut has_real_code = false;
    if let Some(ref items_content) = mod_items_content {
        for line in items_content.lines() {
            let t = line.trim();
            if t.starts_with("pub use ") && t.ends_with(';') {
                let reexport_module = t
                    .trim_start_matches("pub use ")
                    .trim_start_matches("self::")
                    .split("::")
                    .next()
                    .unwrap_or("");
                if is_test_module(reexport_module) {
                    continue;
                }
                user_reexports.push(t.to_string());
            }
            if !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with("#[")
                && !t.starts_with("use ")
                && !(t.starts_with("pub mod ") && t.ends_with(';'))
                && !(t.starts_with("mod ") && t.ends_with(';'))
                && !(t.starts_with("pub use ") && t.ends_with(';'))
            {
                has_real_code = true;
            }
        }
    }
    let has_user_reexports = !user_reexports.is_empty();

    let mut content = String::from("// Auto-generated mod.rs by Windjammer CLI\n");
    content.push_str("// This file declares all generated Windjammer modules\n\n");

    for module in &modules {
        let needs_desktop_gate = module.starts_with("desktop_")
            || (module.starts_with("app_") && module != "app_reactive");
        let needs_test_gate = is_test_module(module);

        if needs_test_gate {
            content.push_str("#[cfg(test)]\n");
        }
        if needs_desktop_gate {
            content.push_str("#[cfg(feature = \"desktop\")]\n");
        }
        content.push_str(&format!("pub mod {};\n", module));
    }

    content.push_str("\n// Re-export public items\n");

    if has_user_reexports {
        // Use the user's re-exports from mod.wj
        for reexport in &user_reexports {
            content.push_str(reexport);
            content.push('\n');
        }
    } else {
        // No explicit re-exports in mod.wj; generate wildcards
        for module in &modules {
            if is_test_module(module) {
                continue;
            }

            let needs_desktop_gate = module.starts_with("desktop_")
                || (module.starts_with("app_") && module != "app_reactive");

            if modules_with_conflicts.contains(module) {
                if let Some(exports) = type_exports.get(module) {
                    for symbol in exports {
                        if !conflicting_symbols.contains(symbol) {
                            if needs_desktop_gate {
                                content.push_str("#[cfg(feature = \"desktop\")]\n");
                            }
                            content.push_str(&format!("pub use {}::{};\n", module, symbol));
                        }
                    }
                }
            } else {
                if needs_desktop_gate {
                    content.push_str("#[cfg(feature = \"desktop\")]\n");
                }
                content.push_str(&format!("pub use {}::*;\n", module));
            }
        }
    }

    // Append real code from mod.wj (traits, structs, impls, functions).
    if has_real_code {
        if let Some(ref items_content) = mod_items_content {
            content.push_str("\n// Code from mod.wj (traits, structs, impls)\n");
            for line in items_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub mod ") && trimmed.ends_with(';') {
                    continue;
                }
                if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
                    continue;
                }
                if trimmed.starts_with("pub use ") && trimmed.ends_with(';') {
                    continue;
                }
                if trimmed == "#[allow(unused_imports)]" {
                    continue;
                }
                if trimmed == "use super::*;" {
                    continue;
                }
                content.push_str(line);
                content.push('\n');
            }
        }
    }

    if mod_items_path.exists() {
        let _ = fs::remove_file(&mod_items_path);
    }

    let mod_file_path = output_dir.join("mod.rs");
    fs::write(&mod_file_path, content)?;

    println!(
        "{} Generated mod.rs with {} modules in {}",
        "✓".green(),
        modules.len(),
        output_dir.display()
    );

    Ok(())
}

/// Strip main() functions from generated Rust files (library mode)
pub fn strip_main_functions(output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    let mut stripped_count = 0;

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "mod.rs" {
                    let content = fs::read_to_string(&path)?;

                    let mut new_lines = Vec::new();
                    let mut found_main = false;

                    for line in content.lines() {
                        let trimmed = line.trim();

                        if trimmed.starts_with("fn main()") || trimmed.starts_with("pub fn main()")
                        {
                            found_main = true;
                            stripped_count += 1;
                            break;
                        }

                        new_lines.push(line);
                    }

                    if found_main {
                        let new_content = new_lines.join("\n") + "\n";
                        fs::write(&path, new_content)?;
                    }
                }
            }
        }
    }

    if stripped_count > 0 {
        println!(
            "{} Stripped {} main() functions (library mode)",
            "✓".green(),
            stripped_count
        );
    }

    Ok(())
}

// Module generation functions from main.rs
pub fn generate_nested_module_structure(source_dir: &Path, output_dir: &Path) -> Result<()> {
    use crate::module_system::{
        discover_nested_modules, generate_lib_rs, generate_mod_rs_for_submodule,
    };
    use anyhow::Context;
    use colored::*;

    // Discover all modules in the source directory
    let module_tree =
        discover_nested_modules(source_dir).context("Failed to discover module structure")?;

    // Build set of ALL generated module names (including nested submodules)
    // Used to prevent stale copies in src/ from being treated as hand-written modules
    fn collect_all_names(
        modules: &[crate::module_system::Module],
        names: &mut std::collections::HashSet<String>,
    ) {
        for m in modules {
            names.insert(m.name.clone());
            if !m.submodules.is_empty() {
                collect_all_names(&m.submodules, names);
            }
        }
    }
    let mut all_generated_names = std::collections::HashSet::new();
    collect_all_names(&module_tree.root_modules, &mut all_generated_names);

    // Generate lib.rs (for crate root) or mod.rs (for subdirectory)
    // THE WINDJAMMER WAY: Auto-discover hand-written Rust modules (FFI/interop)
    // Look for hand-written .rs files in the project root (parent of src)
    let project_root = if let Some(parent) = source_dir.parent() {
        if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        }
    } else {
        source_dir
    };

    // Determine if we should generate lib.rs or mod.rs
    // Generate mod.rs when output is a subdirectory (e.g., src/components/generated)
    // Generate lib.rs when output is a crate root (e.g., out/, target/generated)
    //
    // Detection heuristic:
    // - If path contains ".../src/..." with more components after src, generate mod.rs
    // - Otherwise, generate lib.rs
    let components: Vec<_> = output_dir.components().collect();
    let mut is_subdirectory = false;
    for (i, component) in components.iter().enumerate() {
        if let std::path::Component::Normal(name) = component {
            if name.to_string_lossy() == "src" && i + 1 < components.len() {
                // Path contains ".../src/..." with more components after src
                is_subdirectory = true;
                break;
            }
        }
    }

    let (module_file_name, module_file_path) = if is_subdirectory {
        ("mod.rs", output_dir.join("mod.rs"))
    } else {
        ("lib.rs", output_dir.join("lib.rs"))
    };

    let module_content = generate_lib_rs(&module_tree, project_root, output_dir)?;
    eprintln!("DEBUG generate_nested: writing {} at {:?}", module_file_name, module_file_path);
    eprintln!("DEBUG generate_nested: project_root={:?}, output_dir={:?}, source_dir={:?}", project_root, output_dir, source_dir);
    std::fs::write(&module_file_path, module_content)?;

    // Copy hand-written modules to output directory
    // THE WINDJAMMER WAY: Seamless FFI integration!
    // BUT: Don't overwrite generated .rs files from .wj sources!
    // Check both project_root/ and project_root/src/ (Rust convention)
    let copy_dirs = vec![project_root.to_path_buf(), project_root.join("src")];

    for copy_dir in &copy_dirs {
        if !copy_dir.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(copy_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        // Copy .rs files that aren't lib.rs, mod.rs, or build.rs
                        // build.rs is a Cargo build script and should not be copied to output
                        if name_str.ends_with(".rs")
                            && name_str != "lib.rs"
                            && name_str != "mod.rs"
                            && name_str != "build.rs"
                        {
                            let stem = path.file_stem().unwrap().to_string_lossy();

                            // Don't copy main.rs (not a module)
                            if stem == "main" {
                                continue;
                            }

                            // Don't copy .rs files that match a generated module name
                            if all_generated_names.contains(stem.as_ref()) {
                                continue;
                            }

                            // Don't copy auto-generated files (stale from previous builds)
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if content.starts_with("#[allow(unused_imports)]")
                                    || content.starts_with("// Auto-generated")
                                {
                                    continue;
                                }
                            }

                            // Don't copy .rs files with a corresponding .wj source
                            let corresponding_wj = source_dir.join(format!("{}.wj", stem));
                            if corresponding_wj.exists() {
                                continue;
                            }

                            // CRITICAL FIX: Don't copy .rs files that have corresponding subdirectories
                            // Example: Don't copy events.rs if events/ directory exists
                            // (events.rs declares submodules which won't work in generated context)
                            let corresponding_dir = path.parent().unwrap().join(stem.as_ref());
                            if corresponding_dir.exists() && corresponding_dir.is_dir() {
                                continue; // Skip copying - this is a module parent file
                            }

                            // BUG #12 FIX: Don't copy out-of-scope .rs files
                            // -----------------------------------------------
                            // When output is a subdirectory like src/components/generated/,
                            // we should NOT copy .rs files from the parent src/ directory
                            // (like src/app.rs, src/examples_wasm.rs).
                            //
                            // These are top-level modules declared in src/lib.rs, not part
                            // of the generated components.
                            //
                            // Only copy .rs files that are within the same subdirectory tree
                            // as the output directory.
                            let should_copy = if copy_dir.ends_with("src") {
                                // When scanning src/, check if output is also in src/
                                if let Ok(rel_output) = output_dir.strip_prefix(copy_dir) {
                                    // Output is within src/ (e.g., src/components/generated/)
                                    // Check if this .rs file is within the output tree
                                    if let Ok(rel_path) = path.strip_prefix(copy_dir) {
                                        // Get the first component of the output relative path
                                        // e.g., for src/components/generated/ -> "components"
                                        let output_first_component = rel_output.components().next();
                                        let path_first_component = rel_path.components().next();

                                        // Only copy if they share the same first component
                                        // (i.e., they're in the same subdirectory tree)
                                        // OR if the file is in the immediate directory (no subdirectory)
                                        if let (Some(output_comp), Some(path_comp)) =
                                            (output_first_component, path_first_component)
                                        {
                                            output_comp == path_comp
                                        } else {
                                            // File is directly in src/ (e.g., src/app.rs)
                                            // Don't copy - these are top-level modules
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    // Output is NOT within src/, so we're at project root scope
                                    // Copy all .rs files from src/ (they're in scope)
                                    true
                                }
                            } else {
                                // Not scanning src/, so copy (project root FFI files)
                                true
                            };

                            if should_copy {
                                // Only copy hand-written .rs files (like ffi.rs)
                                let dest = output_dir.join(name);
                                if let Err(e) = std::fs::copy(&path, &dest) {
                                    eprintln!("Warning: Failed to copy {}: {}", name_str, e);
                                }
                            } else {
                                eprintln!(
                                    "⏭️  Skipping out-of-scope file: {} (not within output tree)",
                                    path.display()
                                );
                            }
                        }
                    }
                } else if path.is_dir() {
                    // THE WINDJAMMER WAY: Copy directories with hand-written Rust (like ffi/)
                    if let Some(dir_name) = path.file_name() {
                        let dir_name_str = dir_name.to_string_lossy();
                        let skip_dirs = [
                            "target",
                            "build",
                            "generated",
                            "dist",
                            "node_modules",
                            ".git",
                            "tests_build",
                            "test_output",
                            "test_scenarios",
                            "examples",
                            "benches",
                            "lib",
                        ];

                        if !skip_dirs.contains(&dir_name_str.as_ref()) {
                            // Don't copy directories that correspond to ANY generated module
                            // (including nested submodules like sundering/player → player)
                            if all_generated_names.contains(dir_name_str.as_ref()) {
                                continue;
                            }

                            // Also check for a corresponding .wj directory in src
                            let corresponding_wj_dir = source_dir.join(dir_name_str.as_ref());
                            if corresponding_wj_dir.exists() && corresponding_wj_dir.is_dir() {
                                continue;
                            }

                            // CRITICAL FIX: Don't copy directories that contain the output directory!
                            // This prevents infinite recursion when output is inside the source tree
                            // Example: Don't copy src/components/ when output is src/components/generated/
                            if let Ok(canonical_dir) = path.canonicalize() {
                                if let Ok(canonical_output) = output_dir.canonicalize() {
                                    if canonical_output.starts_with(&canonical_dir) {
                                        continue; // Skip copying - this dir contains the output directory
                                    }
                                }
                            }

                            // Check if this directory has a mod.rs (it's a Rust module)
                            let mod_rs = path.join("mod.rs");
                            if mod_rs.exists() {
                                // Bug #9B FIX: Don't copy out-of-scope hand-written modules
                                // --------------------------------------------------------
                                // When output is a subdirectory like src/components/generated/,
                                // we should NOT copy modules from src/ (like src/events/).
                                //
                                // Only copy modules that are:
                                // 1. In the project root (for FFI interop), OR
                                // 2. Would be copied by the module system itself
                                //
                                // Heuristic: If the module's parent directory is NOT the same as
                                // copy_dir (the directory we're scanning), it's out of scope.
                                //
                                // Example:
                                // - copy_dir: project_root/src/
                                // - path: project_root/src/events/
                                // - output_dir: project_root/src/components/generated/
                                // - Skip because events/ is NOT within components/generated/ tree

                                let should_copy = if copy_dir.ends_with("src") {
                                    // When scanning src/, check if output is also in src/
                                    // If output is src/components/generated/, only copy modules
                                    // that are within the components/ tree
                                    if let Ok(rel_output) = output_dir.strip_prefix(copy_dir) {
                                        // Output is within src/ (e.g., src/components/generated/)
                                        // Check if this module is within the output tree
                                        if let Ok(rel_path) = path.strip_prefix(copy_dir) {
                                            // Get the first component of the output relative path
                                            // e.g., for src/components/generated/ -> "components"
                                            let output_first_component =
                                                rel_output.components().next();
                                            let path_first_component = rel_path.components().next();

                                            // Only copy if they share the same first component
                                            // (i.e., they're in the same subdirectory tree)
                                            output_first_component == path_first_component
                                        } else {
                                            false
                                        }
                                    } else {
                                        // Output is NOT within src/, so we're at project root scope
                                        // Copy all modules from src/ (they're in scope)
                                        true
                                    }
                                } else {
                                    // Not scanning src/, so copy (project root FFI modules)
                                    true
                                };

                                if should_copy {
                                    let dest_dir = output_dir.join(dir_name);
                                    if let Err(e) = crate::test_runner::copy_dir_recursive(&path, &dest_dir) {
                                        eprintln!(
                                            "Warning: Failed to copy directory {}: {}",
                                            dir_name_str, e
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "⏭️  Skipping out-of-scope module: {} (not within output tree)",
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!(
        "{} Generated {} with {} top-level modules",
        "✓".green(),
        module_file_name,
        module_tree.root_modules.len()
    );

    // Recursively generate mod.rs for each directory module
    fn generate_mod_rs_recursive(
        module: &crate::module_system::Module,
        output_dir: &Path,
    ) -> Result<()> {
        if module.is_directory && !module.submodules.is_empty() {
            let module_output_dir = output_dir.join(&module.name);
            std::fs::create_dir_all(&module_output_dir)?;
            let mod_rs_content = generate_mod_rs_for_submodule(module, &module_output_dir)?;
            let mod_rs_path = module_output_dir.join("mod.rs");
            std::fs::write(&mod_rs_path, mod_rs_content)?;

            // Recursively generate for submodules
            for submodule in &module.submodules {
                generate_mod_rs_recursive(submodule, &module_output_dir)?;
            }
        }
        Ok(())
    }

    // Generate mod.rs for all directory modules
    for module in &module_tree.root_modules {
        generate_mod_rs_recursive(module, output_dir)?;
    }

    Ok(())
}

/// Cleanup stale .rs files that conflict with generated directory modules.
/// When a hand-written lighting.rs exists but we've generated a lighting/ directory
/// with mod.rs, Rust complains about finding the module at two locations.
/// This function recursively walks the output directory and removes such stale files.
pub fn cleanup_stale_module_files(output_dir: &Path) -> Result<()> {
    cleanup_stale_module_files_recursive(output_dir)
}

pub fn cleanup_stale_module_files_recursive(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if a sibling .rs file exists with the same name as this directory
                let dir_name = path.file_name().unwrap().to_string_lossy();

                // NEVER remove crate root files (lib.rs, main.rs)
                if dir_name == "lib" || dir_name == "main" {
                    cleanup_stale_module_files_recursive(&path)?;
                    continue;
                }

                let sibling_rs = dir.join(format!("{}.rs", dir_name));

                if sibling_rs.exists() {
                    // Check that the directory has a mod.rs (confirming it's a module directory)
                    let mod_rs = path.join("mod.rs");
                    if mod_rs.exists() {
                        eprintln!(
                            "  Removing stale {}.rs (conflicts with {}/mod.rs)",
                            dir_name, dir_name
                        );
                        std::fs::remove_file(&sibling_rs)?;
                        // Also remove the .rs.map file if it exists
                        let sibling_map = dir.join(format!("{}.rs.map", dir_name));
                        if sibling_map.exists() {
                            std::fs::remove_file(&sibling_map)?;
                        }
                    }
                }

                // Recurse into subdirectories
                cleanup_stale_module_files_recursive(&path)?;
            }
        }
    }

    Ok(())
}

