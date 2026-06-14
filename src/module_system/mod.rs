//! Windjammer Module System - The Windjammer Way!
//!
//! Philosophy:
//! - Auto-discover modules from directory structure (compiler does the work)
//! - Respect explicit pub mod / pub use declarations in mod.wj
//! - Generate lib.rs/mod.rs automatically
//! - No boilerplate - developer focuses on logic, not project structure
//!
//! This is NOT just copying Rust's module system!
//! Windjammer infers structure while Rust forces manual declaration.

pub(crate) mod import_resolution;
pub(crate) mod module_graph;
pub(crate) mod module_resolution;

pub use module_graph::{discover_nested_modules, Module, ModuleTree};

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::test_module_gate::is_test_module;
use import_resolution::parse_mod_declarations;
use module_resolution::discover_hand_written_modules;

/// Whether a `.wj` module should be included when mod.wj lists explicit `pub mod` names.
/// Test modules (`*_test`, `test_*`, etc.) are always auto-discovered with `#[cfg(test)]`.
fn include_module_in_explicit_mod_wj(
    name: &str,
    pub_mods: &[String],
    is_subdirectory: bool,
) -> bool {
    if is_subdirectory {
        return pub_mods.is_empty() || pub_mods.iter().any(|m| m == name);
    }
    pub_mods.is_empty() || pub_mods.iter().any(|m| m == name) || is_test_module(name)
}

/// Generate lib.rs content for a Windjammer project
///
/// The Windjammer Way:
/// - Respect explicit pub mod / pub use declarations from mod.wj
/// - Auto-generate pub mod for discovered directories
/// - Use wildcard re-exports ONLY if no explicit pub use exists
///   Discover hand-written Rust modules in the project root
pub fn generate_lib_rs(
    module_tree: &ModuleTree,
    project_root: &Path,
    output_dir: &Path,
) -> Result<String> {
    let mut content = String::from("// Auto-generated lib.rs by Windjammer\n");
    content.push_str("// This file declares all modules in your Windjammer project\n");
    content.push_str(
        "#![allow(unused_imports, unused_mut, unused_assignments, non_camel_case_types)]\n\n",
    );

    // THE WINDJAMMER WAY: Discover hand-written Rust modules (like ffi.rs)
    // These live in the project root alongside src/ and are automatically integrated
    let hand_written_modules =
        discover_hand_written_modules(project_root, module_tree, output_dir)?;

    // Check if mod.wj exists in root
    let root_mod_path = module_tree.root_path.join("mod.wj");
    let has_root_mod = root_mod_path.exists();

    if has_root_mod {
        // Parse mod.wj to extract pub mod and pub use declarations
        let mod_content = fs::read_to_string(&root_mod_path)?;
        let (_pub_mods, pub_uses) = parse_mod_declarations(&mod_content);

        // THE WINDJAMMER WAY: Auto-discover ALL modules (compiler does the work!)
        // mod.wj controls re-exports, but module declarations are auto-discovered
        content.push_str("// Module declarations (auto-discovered)\n");

        // TDD FIX: Filter out "lib" module to prevent E0761 conflict
        for module in &module_tree.root_modules {
            if module.name != "lib" {
                if module.is_public {
                    content.push_str(&format!("pub mod {};\n", module.name));
                } else {
                    content.push_str(&format!("mod {};\n", module.name));
                }
            }
        }

        // Add hand-written modules (like ffi)
        if !hand_written_modules.is_empty() {
            content.push_str("\n// Hand-written Rust modules (FFI/interop)\n");
            for module_name in &hand_written_modules {
                content.push_str(&format!("pub mod {};\n", module_name));
            }
        }

        // Generate re-exports (controlled by mod.wj)
        if !pub_uses.is_empty() {
            content.push_str("\n// Re-exports (from mod.wj)\n");
            for pub_use in pub_uses {
                content.push_str(&format!("pub use {};\n", pub_use));
            }
        } else {
            // No explicit pub use - use wildcard re-exports
            content.push_str("\n// Auto-generated re-exports\n");

            // TDD FIX: Also skip "lib" in re-exports
            for module in &module_tree.root_modules {
                if module.is_public && module.name != "lib" {
                    content.push_str(&format!("pub use {}::*;\n", module.name));
                }
            }
        }
    } else {
        // No mod.wj - auto-generate everything
        content.push_str("// Auto-discovered modules\n");

        // TDD FIX: Filter out "lib" module to prevent E0761 conflict
        // "lib" is a reserved name for the library itself, not a module to import
        // This prevents: error[E0761]: file for module `lib` found at both "lib.rs" and "lib/mod.rs"
        for module in &module_tree.root_modules {
            if module.name != "lib" {
                content.push_str(&format!("pub mod {};\n", module.name));
            }
        }

        // Add hand-written modules (like ffi)
        if !hand_written_modules.is_empty() {
            content.push_str("\n// Hand-written Rust modules (FFI/interop)\n");
            for module_name in &hand_written_modules {
                content.push_str(&format!("pub mod {};\n", module_name));
            }
        }

        content.push_str("\n// Auto-generated re-exports\n");

        // TDD FIX: Also skip "lib" in re-exports
        for module in &module_tree.root_modules {
            if module.name != "lib" {
                content.push_str(&format!("pub use {}::*;\n", module.name));
            }
        }
    }

    Ok(content)
}

/// Generate mod.rs content for a submodule directory
pub fn generate_mod_rs_for_submodule(module: &Module, output_dir: &Path) -> Result<String> {
    let mut content = String::from("// Auto-generated mod.rs by Windjammer\n\n");

    // THE WINDJAMMER WAY: Find all .wj files in the directory (except mod.wj)
    let mut module_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&module.path) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".wj") && filename != "mod.wj" {
                    let module_name = filename.strip_suffix(".wj").unwrap().to_string();
                    module_files.push(module_name);
                }
            }
        }
    }
    module_files.sort();

    // THE WINDJAMMER FIX: Extract type exports from generated .rs files to detect conflicts
    let mut type_exports: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for module_file in &module_files {
        let rs_file = output_dir.join(format!("{}.rs", module_file));
        if rs_file.exists() {
            if let Ok(content) = fs::read_to_string(&rs_file) {
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
                    type_exports.insert(module_file.clone(), exports);
                }
            }
        }
    }

    // Detect symbol conflicts
    let mut symbol_conflicts: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (module_name, exports) in &type_exports {
        for symbol in exports {
            symbol_conflicts
                .entry(symbol.clone())
                .or_default()
                .push(module_name.clone());
        }
    }

    let has_conflicts = symbol_conflicts
        .values()
        .any(|modules_list| modules_list.len() > 1);

    if has_conflicts {
        eprintln!("⚠ Detected conflicting symbol exports in {}:", module.name);
        for (symbol, modules_list) in &symbol_conflicts {
            if modules_list.len() > 1 {
                eprintln!("  • {} exported by: {}", symbol, modules_list.join(", "));
            }
        }
        eprintln!("→ Skipping glob re-exports to prevent ambiguity");
    }

    // Check if this module has a mod.wj
    let mod_wj_path = module.path.join("mod.wj");
    let has_mod_wj = mod_wj_path.exists();

    if has_mod_wj {
        // Parse mod.wj to extract declarations
        let mod_content = fs::read_to_string(&mod_wj_path)?;
        let (pub_mods, pub_uses) = parse_mod_declarations(&mod_content);

        // Generate pub mod declarations for .wj files in this directory (excluding directories)
        content.push_str("// Module file declarations\n");
        let submodule_names: HashSet<&String> = module.submodules.iter().map(|s| &s.name).collect();
        for module_file in &module_files {
            // Only include if it's not a subdirectory
            if !submodule_names.contains(module_file)
                && include_module_in_explicit_mod_wj(module_file, &pub_mods, false)
            {
                // THE WINDJAMMER FIX: Desktop-only modules need feature gates
                let needs_desktop_gate = module_file.starts_with("desktop_")
                    || (module_file.starts_with("app_") && module_file != "app_reactive");

                if is_test_module(module_file) {
                    content.push_str("#[cfg(test)]\n");
                }
                if needs_desktop_gate {
                    content.push_str("#[cfg(feature = \"desktop\")]\n");
                }
                content.push_str(&format!("pub mod {};\n", module_file));
            }
        }

        // Generate pub mod declarations for subdirectories only
        if !module.submodules.is_empty() {
            content.push_str("\n// Submodule declarations\n");
            for submodule in &module.submodules {
                if include_module_in_explicit_mod_wj(&submodule.name, &pub_mods, true)
                    || is_test_module(&submodule.name)
                {
                    let needs_desktop_gate = submodule.name.starts_with("desktop_")
                        || (submodule.name.starts_with("app_") && submodule.name != "app_reactive");

                    if is_test_module(&submodule.name) {
                        content.push_str("#[cfg(test)]\n");
                    }
                    if needs_desktop_gate {
                        content.push_str("#[cfg(feature = \"desktop\")]\n");
                    }

                    if submodule.is_public {
                        content.push_str(&format!("pub mod {};\n", submodule.name));
                    } else {
                        content.push_str(&format!("mod {};\n", submodule.name));
                    }
                }
            }
        }

        // Add re-exports if specified in mod.wj
        if !pub_uses.is_empty() {
            content.push_str("\n// Re-exports (from mod.wj)\n");
            for pub_use in pub_uses {
                content.push_str(&format!("pub use {};\n", pub_use));
            }
        } else if !has_conflicts {
            // THE WINDJAMMER WAY: Auto-generate re-exports when mod.wj has no explicit pub use.
            // This ensures `use crate::module::Type;` works without requiring the user to
            // write `pub use` declarations for every type — the compiler does the work.
            let submodule_names: HashSet<&String> =
                module.submodules.iter().map(|s| &s.name).collect();
            content.push_str("\n// Auto-generated re-exports\n");
            // Re-export .wj file modules (not already covered by submodules)
            for module_file in &module_files {
                if !submodule_names.contains(module_file)
                    && include_module_in_explicit_mod_wj(module_file, &pub_mods, false)
                    && !is_test_module(module_file)
                {
                    content.push_str(&format!("pub use {}::*;\n", module_file));
                }
            }
            // Re-export subdirectory modules
            for submodule in &module.submodules {
                if (include_module_in_explicit_mod_wj(&submodule.name, &pub_mods, true)
                    || is_test_module(&submodule.name))
                    && !is_test_module(&submodule.name)
                {
                    content.push_str(&format!("pub use {}::*;\n", submodule.name));
                }
            }
        }
    } else {
        // No mod.wj - auto-generate declarations for all .wj files and subdirectories
        // Use module.submodules exclusively (contains both files and directories)
        // Separate files from subdirectories based on is_directory flag

        let files: Vec<_> = module
            .submodules
            .iter()
            .filter(|m| !m.is_directory)
            .collect();
        let subdirs: Vec<_> = module
            .submodules
            .iter()
            .filter(|m| m.is_directory)
            .collect();

        if !files.is_empty() {
            content.push_str("// Auto-discovered modules\n");
            for submodule in files {
                let needs_desktop_gate = submodule.name.starts_with("desktop_")
                    || (submodule.name.starts_with("app_") && submodule.name != "app_reactive");

                if needs_desktop_gate {
                    content.push_str("#[cfg(feature = \"desktop\")]\n");
                }
                content.push_str(&format!("pub mod {};\n", submodule.name));
            }
        }

        if !subdirs.is_empty() {
            content.push_str("\n// Auto-discovered submodules\n");
            for submodule in subdirs {
                let needs_desktop_gate = submodule.name.starts_with("desktop_")
                    || (submodule.name.starts_with("app_") && submodule.name != "app_reactive");

                if needs_desktop_gate {
                    content.push_str("#[cfg(feature = \"desktop\")]\n");
                }
                content.push_str(&format!("pub mod {};\n", submodule.name));
            }
        }

        // Only add glob re-exports if no conflicts
        if !has_conflicts {
            content.push_str("\n// Auto-generated re-exports\n");
            // Use module.submodules exclusively (no duplication)
            for submodule in &module.submodules {
                let needs_desktop_gate = submodule.name.starts_with("desktop_")
                    || (submodule.name.starts_with("app_") && submodule.name != "app_reactive");

                if needs_desktop_gate {
                    content.push_str("#[cfg(feature = \"desktop\")]\n");
                }
                content.push_str(&format!("pub use {}::*;\n", submodule.name));
            }
        } else {
            content.push_str("\n// Note: Glob re-exports skipped due to symbol conflicts\n");
            content.push_str(&format!(
                "// Use explicit imports: use parent::{}::SymbolName;\n",
                module.name
            ));
        }
    }

    // Append compiled code from mod.wj (traits, structs, impls, functions).
    // The compiler writes mod.wj output to _mod_items.rs to avoid conflicting
    // with this auto-generated mod.rs. We include that code here.
    let mod_items_path = output_dir.join("_mod_items.rs");
    if mod_items_path.exists() {
        if let Ok(items_content) = fs::read_to_string(&mod_items_path) {
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
        // Clean up the temporary file
        let _ = fs::remove_file(&mod_items_path);
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::import_resolution::parse_mod_declarations;
    use super::module_resolution::discover_hand_written_modules;
    use super::*;
    use tempfile::TempDir;

    fn create_test_dir(files: &[(&str, &str)]) -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        for (path, content) in files {
            let full_path = temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, content).unwrap();
        }

        temp_dir
    }

    #[test]
    fn test_parse_mod_declarations_multiline_pub_use() {
        // TDD Test: Parse multi-line pub use statements
        let content = r#"
// Commands Module
pub mod command

pub use command::{
    EditorCommand,
    MoveObjectCommand,
    SelectObjectCommand,
}
"#;

        let (pub_mods, pub_uses) = parse_mod_declarations(content);

        assert_eq!(pub_mods, vec!["command"]);
        assert_eq!(pub_uses.len(), 1);

        // Should capture the full multi-line pub use statement
        let pub_use = &pub_uses[0];
        assert!(pub_use.contains("command::"), "Should contain module path");
        assert!(
            pub_use.contains("EditorCommand"),
            "Should contain imported items"
        );
        assert!(!pub_use.contains("{;"), "Should not have malformed closing");
    }

    #[test]
    fn test_parse_mod_declarations_single_line_pub_use() {
        let content = "pub use module::*;\n";
        let (_, pub_uses) = parse_mod_declarations(content);
        assert_eq!(pub_uses, vec!["module::*"]);
    }

    #[test]
    fn test_parse_mod_declarations() {
        let content = r#"
pub mod math
pub mod rendering
pub use math::Vec2
pub use math::Vec3
pub use rendering::Color
"#;

        let (pub_mods, pub_uses) = parse_mod_declarations(content);

        assert_eq!(pub_mods, vec!["math", "rendering"]);
        assert_eq!(
            pub_uses,
            vec!["math::Vec2", "math::Vec3", "rendering::Color"]
        );
    }

    #[test]
    fn test_discover_flat_modules() {
        let temp_dir = create_test_dir(&[
            ("vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
            (
                "vec3.wj",
                "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }",
            ),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();

        assert_eq!(tree.root_modules.len(), 2);
        assert!(tree.has_module(&["vec2"]));
        assert!(tree.has_module(&["vec3"]));
    }

    #[test]
    fn test_discover_nested_modules() {
        let temp_dir = create_test_dir(&[
            ("mod.wj", "pub mod math\npub mod rendering"),
            ("math/mod.wj", "pub mod vec2\npub mod vec3"),
            ("math/vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
            (
                "math/vec3.wj",
                "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }",
            ),
            (
                "rendering/color.wj",
                "pub struct Color { pub r: u8, pub g: u8, pub b: u8 }",
            ),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();

        assert_eq!(tree.root_modules.len(), 2);
        assert!(tree.has_module(&["math"]));
        assert!(tree.has_module(&["math", "vec2"]));
        assert!(tree.has_module(&["math", "vec3"]));
        assert!(tree.has_module(&["rendering"]));
        assert!(tree.has_module(&["rendering", "color"]));
    }

    #[test]
    fn test_generate_lib_rs_with_explicit_declarations() {
        let temp_dir = create_test_dir(&[
            (
                "mod.wj",
                "pub mod math\npub use math::Vec2\npub use math::Vec3",
            ),
            ("math/vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
            (
                "math/vec3.wj",
                "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }",
            ),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();
        let output_dir = temp_dir.path().join("out");
        fs::create_dir_all(&output_dir).unwrap();
        let lib_rs = generate_lib_rs(
            &tree,
            temp_dir.path().parent().unwrap_or(temp_dir.path()),
            &output_dir,
        )
        .unwrap();

        assert!(lib_rs.contains("pub mod math;"));
        assert!(lib_rs.contains("pub use math::Vec2;"));
        assert!(lib_rs.contains("pub use math::Vec3;"));
        assert!(
            !lib_rs.contains("pub use math::*;"),
            "Should use explicit re-exports, not wildcard"
        );
    }

    #[test]
    fn test_nested_submodule_not_treated_as_hand_written() {
        // Regression test: when src/game/player/ exists as a submodule of game/,
        // a stale copy at project_root/src/player/ should NOT be picked up as a
        // "hand-written" top-level module.
        let temp_dir = create_test_dir(&[
            ("mod.wj", "pub mod game"),
            ("game/mod.wj", "pub mod player\npub mod combat"),
            ("game/player/mod.wj", "pub mod controller"),
            (
                "game/player/controller.wj",
                "pub struct PlayerController { pub x: f32 }",
            ),
            ("game/combat/mod.wj", "pub mod weapon"),
            (
                "game/combat/weapon.wj",
                "pub struct Weapon { pub damage: f32 }",
            ),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();

        // Simulate stale copies in a "src/" sibling directory
        let project_root = temp_dir.path().parent().unwrap_or(temp_dir.path());
        let stale_src = project_root.join("src");
        fs::create_dir_all(stale_src.join("player")).unwrap();
        fs::write(stale_src.join("player/mod.rs"), "pub mod controller;").unwrap();
        fs::create_dir_all(stale_src.join("combat")).unwrap();
        fs::write(stale_src.join("combat/mod.rs"), "pub mod weapon;").unwrap();

        let output_dir = temp_dir.path().join("out");
        fs::create_dir_all(&output_dir).unwrap();

        let hand_written = discover_hand_written_modules(project_root, &tree, &output_dir).unwrap();

        assert!(
            !hand_written.contains(&"player".to_string()),
            "Nested submodule 'player' should NOT be treated as hand-written. Found: {:?}",
            hand_written
        );
        assert!(
            !hand_written.contains(&"combat".to_string()),
            "Nested submodule 'combat' should NOT be treated as hand-written. Found: {:?}",
            hand_written
        );
    }

    #[test]
    fn test_generate_mod_rs_no_duplicate_pub_use() {
        let temp_dir = create_test_dir(&[
            ("mod.wj", "pub mod item\npub mod item_stack\npub use item::Item\npub use item_stack::ItemStack"),
            ("item.wj", "pub struct Item { pub name: String }"),
            ("item_stack.wj", "pub struct ItemStack { pub quantity: u32 }"),
        ]);

        let tree = discover_nested_modules(temp_dir.path()).unwrap();
        assert!(tree.has_module(&["item"]));
        assert!(tree.has_module(&["item_stack"]));

        let output_dir = temp_dir.path().join("out");
        fs::create_dir_all(&output_dir).unwrap();

        fs::write(
            output_dir.join("_mod_items.rs"),
            "pub use self::item::Item;\npub use self::item_stack::ItemStack;\n",
        )
        .unwrap();

        let parent_module = Module {
            name: "inventory".to_string(),
            path: temp_dir.path().to_path_buf(),
            is_public: true,
            is_directory: true,
            has_mod_wj: true,
            submodules: tree.root_modules.clone(),
        };

        let content = generate_mod_rs_for_submodule(&parent_module, &output_dir).unwrap();

        let item_count = content.matches("pub use self::item::Item;").count()
            + content.matches("pub use item::Item;").count();
        assert_eq!(
            item_count, 1,
            "pub use Item should appear exactly once, but found {} times.\nGenerated content:\n{}",
            item_count, content
        );

        let stack_count = content
            .matches("pub use self::item_stack::ItemStack;")
            .count()
            + content.matches("pub use item_stack::ItemStack;").count();
        assert_eq!(
            stack_count, 1,
            "pub use ItemStack should appear exactly once, but found {} times.\nGenerated content:\n{}",
            stack_count, content
        );
    }
}
