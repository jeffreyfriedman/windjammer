//! Build utilities for CLI - generate_mod_file, strip_main_functions.
//!
//! Extracted from main.rs for use by cli/build.rs when the cli feature is enabled.

use anyhow::Result;
use std::path::Path;

/// Generate mod.rs file with pub mod declarations and re-exports
pub fn generate_mod_file(output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    // Find all .rs files (excluding mod.rs itself) and extract their public types
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
        println!(
            "{} No modules found to generate mod.rs",
            "Warning:".yellow().bold()
        );
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

    let has_conflicts = symbol_conflicts
        .values()
        .any(|modules_list| modules_list.len() > 1);

    if has_conflicts {
        use colored::*;
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
            "{} Skipping glob re-exports to prevent ambiguity",
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

    if !has_conflicts {
        content.push_str("\n// Re-export all public items\n");
        for module in &modules {
            let needs_desktop_gate = module.starts_with("desktop_")
                || (module.starts_with("app_") && module != "app_reactive");

            if needs_desktop_gate {
                content.push_str("#[cfg(feature = \"desktop\")]\n");
            }
            content.push_str(&format!("pub use {}::*;\n", module));
        }
    } else {
        content.push_str("\n// Note: Glob re-exports skipped due to symbol conflicts\n");
        content.push_str("// Use explicit imports: use your_crate::module_name::SymbolName;\n");
    }

    let mod_file_path = output_dir.join("mod.rs");
    fs::write(&mod_file_path, content)?;

    println!(
        "{} Generated mod.rs with {} modules",
        "✓".green(),
        modules.len()
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
