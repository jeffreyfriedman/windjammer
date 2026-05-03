//! Build utilities for CLI - generate_mod_file, strip_main_functions.
//!
//! Extracted from main.rs for use by cli/build.rs when the cli feature is enabled.

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

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
/// directory when they are not already present. This keeps hand-written Rust next to `src_wj/`
/// discoverable in `out/` without pulling in unrelated trees (output is usually `out/`, not under
/// `src/`).
fn copy_project_src_tree_into_output(output_dir: &Path) -> std::io::Result<()> {
    use std::fs;
    let Some(root) = output_dir.parent() else {
        return Ok(());
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
    if existing_mod_rs.exists() {
        if let Ok(content) = fs::read_to_string(&existing_mod_rs) {
            let is_auto_generated = content.starts_with("// Auto-generated mod.rs");
            if !is_auto_generated {
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
                if !extra_modules.is_empty() {
                    extra_modules.sort();
                    let mut updated = content.clone();
                    for m in &extra_modules {
                        updated.push_str(&format!("\npub mod {};", m));
                        updated.push_str(&format!("\npub use {}::*;", m));
                    }
                    updated.push('\n');
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
                // THE WINDJAMMER FIX: Exclude lib.rs, mod.rs, and main.rs from module declarations
                if file_name.ends_with(".rs")
                    && file_name != "mod.rs"
                    && file_name != "main.rs"
                    && file_name != "lib.rs"
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

    let mut content = String::from("// Auto-generated mod.rs by Windjammer CLI\n");
    content.push_str("// This file declares all generated Windjammer modules\n\n");

    for module in &modules {
        let needs_desktop_gate = module.starts_with("desktop_")
            || (module.starts_with("app_") && module != "app_reactive");

        if needs_desktop_gate {
            content.push_str("#[cfg(feature = \"desktop\")]\n");
        }
        content.push_str(&format!("pub mod {};\n", module));
    }

    content.push_str("\n// Re-export public items\n");
    for module in &modules {
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
