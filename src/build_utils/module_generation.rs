//! `mod.rs` / `lib.rs` generation and stale module cleanup.

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use super::file_operations::{copy_project_src_tree_into_output, copy_sibling_rs_from_parent};
use super::path_utilities::{is_submodule_output_dir, source_dir_for_output};

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
                            !(t.is_empty()
                                || t.starts_with("//")
                                || t.starts_with("#[")
                                || t.starts_with("use ")
                                || (t.starts_with("pub mod ") && t.ends_with(';'))
                                || (t.starts_with("mod ") && t.ends_with(';'))
                                || (t.starts_with("pub use ") && t.ends_with("::*;")))
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
            if !(t.is_empty()
                || t.starts_with("//")
                || t.starts_with("#[")
                || t.starts_with("use ")
                || (t.starts_with("pub mod ") && t.ends_with(';'))
                || (t.starts_with("mod ") && t.ends_with(';'))
                || (t.starts_with("pub use ") && t.ends_with(';')))
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
