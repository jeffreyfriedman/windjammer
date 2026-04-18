//! Build utilities for CLI - generate_mod_file, strip_main_functions.
//!
//! Extracted from main.rs for use by cli/build.rs when the cli feature is enabled.

use anyhow::Result;
use std::path::Path;

/// Generate mod.rs file with pub mod declarations and re-exports.
/// Recursively generates mod.rs for nested subdirectories (e.g., ui/mod.rs, components/mod.rs).
/// For the root directory, generates `lib.rs` if no `lib.rs` exists (to serve as crate root
/// without invalid `use super::*`).
pub fn generate_mod_file(output_dir: &Path) -> Result<()> {
    generate_mod_file_recursive(output_dir)?;

    let mod_rs = output_dir.join("mod.rs");
    let lib_rs = output_dir.join("lib.rs");

    // Always regenerate lib.rs from mod.rs when mod.rs exists.
    // lib.rs is derived from mod.rs (with `use super::*` stripped),
    // so it must stay in sync.
    if mod_rs.exists() {
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
    }

    // If lib.rs now exists, patch Cargo.toml to point to it instead of mod.rs
    if lib_rs.exists() {
        let cargo_toml_path = output_dir.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(toml_content) = std::fs::read_to_string(&cargo_toml_path) {
                if toml_content.contains("path = \"mod.rs\"") {
                    let updated = toml_content.replace(
                        "path = \"mod.rs\"",
                        "path = \"lib.rs\"",
                    );
                    std::fs::write(&cargo_toml_path, updated)?;
                }
            }
        }
    }

    Ok(())
}

/// Recursively generate mod.rs for a directory and all its subdirectories.
/// Processes subdirectories first (depth-first) so parent mod.rs can reference child modules.
fn generate_mod_file_recursive(output_dir: &Path) -> Result<()> {
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
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| {
                        let p = e.path();
                        p.is_file()
                            && p.extension().and_then(|s| s.to_str()) == Some("rs")
                            && p.file_name().and_then(|n| n.to_str()) != Some("mod.rs")
                    })
            })
            .unwrap_or(false);
        let has_rs_subdirs = fs::read_dir(subdir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().is_dir())
            })
            .unwrap_or(false);
        if has_rs_files || has_rs_subdirs {
            generate_mod_file_recursive(subdir)?;
        }
    }

    // Step 2: If a compiler-generated mod.rs exists (from compiling mod.wj),
    // preserve it - it contains type definitions that we must not overwrite.
    let existing_mod_rs = output_dir.join("mod.rs");
    if existing_mod_rs.exists() {
        if let Ok(content) = fs::read_to_string(&existing_mod_rs) {
            let is_auto_generated = content.starts_with("// Auto-generated mod.rs");
            if !is_auto_generated {
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
