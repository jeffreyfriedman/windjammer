//! Program-level Rust generation: `generate_program`, import stitching, and defer-drop wrapping.

use crate::analyzer::*;
use crate::codegen::rust::expression_helpers;
use crate::parser::*;
use crate::CompilationTarget;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// E0252: remove redundant `use` lines that import the same final type name again
    /// (common when `use super::*` / auto `super::` imports overlap explicit `crate::...` uses).
    fn dedupe_rust_import_lines(block: &str) -> String {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out_lines: Vec<String> = Vec::new();
        for line in block.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                out_lines.push(line.to_string());
                continue;
            }
            if trimmed.starts_with("#[") {
                out_lines.push(line.to_string());
                continue;
            }
            let (is_pub, after_use) = if let Some(r) = trimmed.strip_prefix("pub use ") {
                (true, r)
            } else if let Some(r) = trimmed.strip_prefix("use ") {
                (false, r)
            } else {
                out_lines.push(line.to_string());
                continue;
            };
            let rest = after_use.trim().trim_end_matches(';').trim();
            if rest.contains("::*") {
                out_lines.push(line.to_string());
                continue;
            }
            if let Some(open) = rest.find("::{") {
                if let Some(close) = rest.rfind('}') {
                    let path_part = rest[..open].trim();
                    let inner = &rest[open + 3..close];
                    let mut kept: Vec<String> = Vec::new();
                    for part in inner.split(',') {
                        let p = part.trim();
                        if p.is_empty() {
                            continue;
                        }
                        let name = p.split(" as ").next().unwrap_or("").trim();
                        if name.is_empty() {
                            continue;
                        }
                        if seen.insert(name.to_string()) {
                            kept.push(p.to_string());
                        }
                    }
                    if kept.is_empty() {
                        continue;
                    }
                    let stmt = format!(
                        "{}use {}::{{{}}};",
                        if is_pub { "pub " } else { "" },
                        path_part,
                        kept.join(", ")
                    );
                    out_lines.push(stmt);
                    continue;
                }
            }
            if let Some(last) = rest.rsplit("::").next() {
                let name = last.trim();
                if name.is_empty() {
                    out_lines.push(line.to_string());
                    continue;
                }
                if seen.insert(name.to_string()) {
                    out_lines.push(line.to_string());
                }
                continue;
            }
            out_lines.push(line.to_string());
        }
        out_lines.join("\n")
    }

    pub fn generate_program(
        &mut self,
        program: &Program<'ast>,
        analyzed: &[AnalyzedFunction<'ast>],
    ) -> String {
        let mut imports = String::new();
        let mut body = String::new();

        // PRE-PASS: Structs that transitively contain trait objects must not auto-derive Debug/Clone.
        // Must run before `collect_partial_eq_types` (which calls `infer_derivable_traits`).
        self.collect_trait_object_types(program);

        // PRE-PASS: Collect which custom types support PartialEq
        // This enables smart enum derive that only adds PartialEq if all variants support it
        self.collect_partial_eq_types(program);

        // PRE-PASS: Collect types that implement Drop (cannot derive Copy, Rust E0184)
        for item in &program.items {
            if let Item::Impl { block, .. } = item {
                if block.trait_name.as_deref() == Some("Drop") {
                    self.types_with_drop.insert(block.type_name.clone());
                }
            }
        }

        // Collect bound aliases first (bound Name = Trait + Trait)
        for item in &program.items {
            if let Item::BoundAlias { name, traits, .. } = item {
                self.bound_aliases.insert(name.clone(), traits.clone());
            }
        }

        // Collect struct definitions for implicit self support
        let mut struct_fields: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for item in &program.items {
            if let Item::Struct { decl: s, .. } = item {
                let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
                struct_fields.insert(s.name.clone(), field_names);
            }
        }

        // Track explicitly imported traits to avoid duplication with auto-imports
        let mut explicitly_imported_traits: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // PRE-PASS: Collect import aliases so type_to_rust skips stdlib mappings
        // when the user has defined their own alias (e.g., `use std::collections::HashMap as Map`)
        for item in &program.items {
            if let Item::Use {
                alias: Some(alias_name),
                path,
                ..
            } = item
            {
                self.import_aliases.insert(alias_name.clone());
                if let Some(last_segment) = path.last() {
                    self.module_alias_map
                        .insert(alias_name.clone(), last_segment.clone());
                }
            }
        }

        // Check for stdlib modules that need special imports
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                // Path is ["std", "json"] for "use std::json"
                let path_str = path.join("::");
                if (path_str.starts_with("std::") || path_str == "std") && path_str.contains("json")
                {
                    self.needs_serde_imports = true;
                }
                // If user already imports HashMap/HashSet from std::collections, mark them
                if path_str.contains("HashMap") {
                    self.needs_hashmap_import = true;
                }
                if path_str.contains("HashSet") {
                    self.needs_hashset_import = true;
                }
                // Track explicit std::ops imports to prevent duplication
                if path_str.starts_with("std::ops::") {
                    if let Some(trait_name) = path_str.strip_prefix("std::ops::") {
                        explicitly_imported_traits.insert(trait_name.to_string());
                    }
                }
                // Track explicit std::fmt imports to prevent duplication
                if path_str.starts_with("std::fmt::") {
                    if let Some(trait_name) = path_str.strip_prefix("std::fmt::") {
                        explicitly_imported_traits.insert(trait_name.to_string());
                    }
                }
                // http, time, crypto modules don't need special imports (used directly)
            }
        }

        // THE WINDJAMMER WAY: Auto-detect usage of common stdlib types and traits
        // Walk the AST properly to find HashMap/HashSet usage in types and expressions
        // (NOT debug text, which includes comments and causes false positives)
        {
            if !self.needs_hashmap_import
                && (Self::program_references_collection(program, "HashMap")
                    || Self::program_references_collection(program, "Map"))
            {
                self.needs_hashmap_import = true;
            }
            if !self.needs_hashset_import && Self::program_references_collection(program, "HashSet")
            {
                self.needs_hashset_import = true;
            }
        }

        // Auto-detect operator trait implementations (impl Add, impl Sub, etc.)
        // and add the necessary std::ops imports (only if not already explicitly imported)
        for item in &program.items {
            if let Item::Impl { block, .. } = item {
                if let Some(ref trait_name) = block.trait_name {
                    // Skip if the user already has an explicit import for this trait
                    if explicitly_imported_traits.contains(trait_name.as_str()) {
                        continue;
                    }
                    match trait_name.as_str() {
                        "Add" | "Sub" | "Mul" | "Div" | "Neg" | "Rem" | "AddAssign"
                        | "SubAssign" | "MulAssign" | "DivAssign" => {
                            self.needs_trait_imports.insert(trait_name.clone());
                        }
                        "Display" | "Debug" => {
                            self.needs_trait_imports.insert(trait_name.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Collect inline module names for self:: prefix generation in pub use
        self.inline_module_names.clear();
        for item in &program.items {
            if let Item::Mod { name, .. } = item {
                self.inline_module_names.insert(name.clone());
            }
        }

        // Generate explicit use statements
        let mut has_explicit_pub_use = false;
        for item in &program.items {
            if let Item::Use {
                path,
                alias,
                is_pub,
                ..
            } = item
            {
                if *is_pub {
                    has_explicit_pub_use = true;
                }
                let use_stmt = self.generate_use(path, alias.as_deref());
                if !use_stmt.trim().is_empty() {
                    if *is_pub {
                        imports.push_str("pub ");
                    }
                    imports.push_str(&use_stmt);
                }
            }
        }

        // Auto-generate pub use re-exports for mod.rs files without explicit pub use.
        // When a mod.wj declares `pub mod submod` but no `pub use submod::Type`,
        // users expect `use crate::mymod::Type` to work. This requires re-exports.
        if self.is_output_mod_rs() && !has_explicit_pub_use {
            for item in &program.items {
                if let Item::Mod {
                    name,
                    is_public: true,
                    ..
                } = item
                {
                    imports.push_str(&format!("pub use self::{}::*;\n", name));
                }
            }
        }

        // Generate const and static declarations
        for item in &program.items {
            match item {
                Item::Const {
                    name,
                    is_pub,
                    type_,
                    value,
                    ..
                } => {
                    let pub_prefix = if *is_pub || self.is_module {
                        "pub "
                    } else {
                        ""
                    };

                    // Special case: string constants should use &'static str, not String
                    let rust_type = if matches!(type_, Type::String)
                        && matches!(
                            value,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                        "&'static str".to_string()
                    } else {
                        self.type_to_rust(type_)
                    };

                    body.push_str(&format!(
                        "{}const {}: {} = {};\n",
                        pub_prefix,
                        name,
                        rust_type,
                        self.generate_expression_immut(value)
                    ));
                }
                Item::Static {
                    name,
                    mutable,
                    type_,
                    value,
                    ..
                } => {
                    if *mutable {
                        body.push_str(&format!(
                            "static mut {}: {} = {};\n",
                            name,
                            self.type_to_rust(type_),
                            self.generate_expression_immut(value)
                        ));
                    } else {
                        // PHASE 7: Promote static to const if value is compile-time evaluable
                        let keyword = if expression_helpers::is_const_evaluable(value) {
                            "const" // Zero runtime overhead!
                        } else {
                            "static"
                        };

                        body.push_str(&format!(
                            "{} {}: {} = {};\n",
                            keyword,
                            name,
                            self.type_to_rust(type_),
                            self.generate_expression_immut(value)
                        ));
                    }
                }
                Item::TypeAlias {
                    name,
                    target,
                    is_pub,
                    ..
                } => {
                    let pub_prefix = if *is_pub { "pub " } else { "" };
                    body.push_str(&format!(
                        "{}type {} = {};\n",
                        pub_prefix,
                        name,
                        self.type_to_rust(target)
                    ));
                }
                _ => {}
            }
        }

        if !body.is_empty() {
            body.push('\n');
        }

        // Collect names of functions in impl blocks and trait methods to avoid generating them twice
        let mut impl_methods = std::collections::HashSet::new();
        for item in &program.items {
            if let Item::Impl {
                block: impl_block, ..
            } = item
            {
                for func in &impl_block.functions {
                    impl_methods.insert(func.name.clone());
                }
            }
            // Also collect trait method names
            if let Item::Trait { decl, .. } = item {
                for method in &decl.methods {
                    impl_methods.insert(method.name.clone());
                }
            }
        }

        // Generate structs, enums, and traits
        for item in &program.items {
            match item {
                Item::Struct { decl: s, .. } => {
                    body.push_str(&self.generate_struct(s));
                    body.push_str("\n\n");

                    // Check for @component or @game decorators and generate trait implementations
                    if s.decorators.iter().any(|d| d.name == "component") {
                        body.push_str(&self.generate_component_impl(s));
                        body.push_str("\n\n");
                    }
                    if s.decorators.iter().any(|d| d.name == "game") {
                        body.push_str(&self.generate_game_impl(s));
                        body.push_str("\n\n");
                    }
                }
                Item::Enum { decl: e, .. } => {
                    body.push_str(&self.generate_enum(e));
                    body.push_str("\n\n");
                }
                Item::Trait { decl: t, .. } => {
                    body.push_str(&self.generate_trait_with_analysis(t, analyzed));
                    body.push_str("\n\n");
                }
                Item::Impl {
                    block: impl_block, ..
                } => {
                    // Set the struct name, fields, and method names for implicit self support
                    self.current_struct_name = Some(impl_block.type_name.clone());
                    if let Some(fields) = struct_fields.get(&impl_block.type_name) {
                        self.current_struct_fields = fields.iter().cloned().collect();
                    } else {
                        self.current_struct_fields.clear();
                    }
                    self.current_impl_methods = impl_block
                        .functions
                        .iter()
                        .map(|f| f.name.clone())
                        .collect();
                    self.in_impl_block = true;

                    body.push_str(&self.generate_impl(impl_block, analyzed));
                    body.push_str("\n\n");

                    self.in_impl_block = false;
                    self.current_struct_name = None;
                    self.current_struct_fields.clear();
                    self.current_impl_methods.clear();
                    self.current_impl_instance_methods.clear();
                }
                Item::Mod {
                    name,
                    items,
                    is_public,
                    ..
                } => {
                    // THE WINDJAMMER WAY: In multi-file projects, NEVER inline modules
                    // Even if the AST has items (from cross-file trait inference),
                    // we should generate external declarations (mod name;)
                    // Inline modules are ONLY for single-file compilation

                    // CRITICAL FIX: Prioritize self.is_module over items.is_empty()
                    // During trait inference regeneration, items may be populated even for external modules
                    if self.is_module || items.is_empty() {
                        // External module declaration: mod math;
                        // Use this in multi-file projects (when is_module=true)
                        // OR when items is empty (explicit external mod)
                        if *is_public {
                            body.push_str(&format!("pub mod {};\n", name));
                        } else {
                            body.push_str(&format!("mod {};\n", name));
                        }
                    } else {
                        // Inline module: mod math { ... }
                        // ONLY used in single-file projects (when is_module=false AND items not empty)
                        if *is_public {
                            body.push_str(&format!("pub mod {} {{\n", name));
                        } else {
                            body.push_str(&format!("mod {} {{\n", name));
                        }

                        // Increase indentation for nested items
                        self.indent_level += 1;

                        // Generate all items inside the module
                        for item in items {
                            body.push_str(&self.indent());
                            body.push_str(&self.generate_inline_module_item(item, analyzed));
                        }

                        // Decrease indentation
                        self.indent_level -= 1;
                        body.push_str("}\n\n");
                    }
                }
                _ => {}
            }
        }

        // Generate extern functions (FFI declarations)
        let extern_funcs: Vec<_> = analyzed
            .iter()
            .filter(|af| af.decl.is_extern && !impl_methods.contains(&af.decl.name))
            .collect();

        if !extern_funcs.is_empty() {
            body.push_str("extern \"C\" {\n");
            for extern_func in extern_funcs {
                body.push_str(&self.generate_extern_function(&extern_func.decl));
            }
            body.push_str("}\n\n");
        }

        // Generate top-level functions (skip impl methods and extern functions)
        for analyzed_func in analyzed {
            if !impl_methods.contains(&analyzed_func.decl.name) && !analyzed_func.decl.is_extern {
                // Skip main() function in modules - it should only be in the entry point
                if self.is_module && analyzed_func.decl.name == "main" {
                    continue;
                }
                // Generate the function
                body.push_str(&self.generate_function(analyzed_func));
                body.push_str("\n\n");
            }
        }

        // Check for test decorators or test_ prefix functions (for test runtime import)
        let filename_str = self.current_wj_file.to_string_lossy();
        let is_test_file = filename_str.ends_with("_test.wj") || filename_str.contains("_test.wj");
        let has_test_functions = analyzed.iter().any(|af| {
            // Check for explicit decorators (@test, @property_test, @test_cases)
            let has_test_decorator =
                af.decl.decorators.iter().any(|d| {
                    d.name == "test" || d.name == "property_test" || d.name == "test_cases"
                });

            // Check for implicit test_ prefix naming convention (only in test files)
            let has_test_prefix = is_test_file && af.decl.name.starts_with("test_");

            has_test_decorator || has_test_prefix
        });

        // Check for property testing decorators and collect max parameter count
        let mut max_property_test_params = 0;
        for analyzed_func in analyzed {
            if analyzed_func
                .decl
                .decorators
                .iter()
                .any(|d| d.name == "property_test")
            {
                let param_count = analyzed_func.decl.parameters.len();
                if param_count > max_property_test_params {
                    max_property_test_params = param_count;
                }
            }
        }

        // Inject implicit imports if needed
        let mut implicit_imports = String::new();

        // Cross-module type references: only when we do NOT inject `use super::*` below.
        // Injected `use super::*` already pulls in sibling types re-exported from the parent `mod.rs`;
        // extra `use super::Type` lines are often wrong (Type lives in `super::other_module::Type`)
        // and duplicate globs (E0252). If the user already wrote `use super::*`, we also skip (see
        // `auto_super_type_import_paths`).
        let has_explicit_glob_imports = imports.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.ends_with("::*;") && !trimmed.starts_with("//")
        });
        let will_inject_super_glob = self.is_module && !has_explicit_glob_imports;
        let auto_super_type_imports = if will_inject_super_glob {
            String::new()
        } else {
            self.format_auto_super_type_imports(program)
        };
        if !auto_super_type_imports.is_empty() {
            implicit_imports.push_str(&auto_super_type_imports);
        }

        // Add trait imports for inferred bounds
        if !self.needs_trait_imports.is_empty() {
            let mut sorted_traits: Vec<_> = self.needs_trait_imports.iter().collect();
            sorted_traits.sort();
            for trait_name in sorted_traits {
                match trait_name.as_str() {
                    "Display" | "Debug" => {
                        implicit_imports.push_str(&format!("use std::fmt::{};\n", trait_name));
                    }
                    "Clone" => {
                        // Clone is in prelude, no import needed
                    }
                    "Add" | "Sub" | "Mul" | "Div" | "Neg" | "Rem" | "AddAssign" | "SubAssign"
                    | "MulAssign" | "DivAssign" => {
                        implicit_imports.push_str(&format!("use std::ops::{};\n", trait_name));
                    }
                    "PartialEq" | "Eq" | "PartialOrd" | "Ord" => {
                        // These are in prelude, no import needed
                    }
                    "IntoIterator" | "Iterator" => {
                        // These are in prelude, no import needed
                    }
                    _ => {
                        // Custom trait, assume it's already in scope
                    }
                }
            }
        }

        if self.needs_wasm_imports {
            implicit_imports.push_str("use wasm_bindgen::prelude::*;\n");
        }
        if self.needs_web_imports {
            implicit_imports.push_str("use web_sys::*;\n");
        }
        if self.needs_js_imports {
            implicit_imports.push_str("use js_sys::*;\n");
        }
        if self.needs_serde_imports {
            implicit_imports.push_str("use serde::{Serialize, Deserialize};\n");
        }
        if self.needs_smallvec_import {
            implicit_imports.push_str("use smallvec::{SmallVec, smallvec};\n");
        }
        if self.needs_cow_import {
            implicit_imports.push_str("use std::borrow::Cow;\n");
        }
        if self.needs_write_import {
            implicit_imports.push_str("use std::fmt::Write;\n");
        }
        if self.needs_hashmap_import && !imports.contains("std::collections::HashMap") {
            implicit_imports.push_str("use std::collections::HashMap;\n");
        }
        if self.needs_hashset_import && !imports.contains("std::collections::HashSet") {
            implicit_imports.push_str("use std::collections::HashSet;\n");
        }

        // THE WINDJAMMER WAY: Auto-import sibling types in module directories
        // When compiling a multi-file project, each file in a module directory
        // should have access to sibling types re-exported by the parent mod.rs.
        // This prevents the need for explicit imports of types within the same module.
        // Example: quest/manager.rs gets `use super::*;` which imports QuestId, Quest, etc.
        // from quest/mod.rs's re-exports.
        // For root-level modules, `super` refers to the crate root (lib.rs), which is harmless.
        //
        // IMPORTANT: When the file has explicit glob imports (use crate::X::*), we must NOT
        // add `use super::*` because two glob imports bringing the same name into scope causes
        // Rust error E0659 ("ambiguous name"). For example, if mod.rs re-exports GizmoMode
        // from scene_view, and the file also has `use crate::gizmos::*` which exports its own
        // GizmoMode, both globs would bring GizmoMode into scope, making it ambiguous.
        // Don't inject `use super::*;` for the crate lib root (mod.rs that IS lib.rs).
        // super has no parent at the crate root → E0433.
        let is_lib_root = self.is_output_mod_rs()
            && self
                .current_output_file
                .parent()
                .map(|d| d.join("Cargo.toml").exists())
                .unwrap_or(false);
        if self.is_module && !has_explicit_glob_imports && !is_lib_root {
            implicit_imports.push_str("#[allow(unused_imports)]\nuse super::*;\n");
        }

        // TDD FIX: Auto-import test runtime for files with test functions
        // THE WINDJAMMER WAY: Files with @test decorators should auto-import test utilities
        // Bug: Test functions can't find assert_eq, assert_gt, etc.
        // Root Cause: Codegen doesn't auto-import windjammer_runtime::test::*
        // Fix: Check if module has ANY functions with @test/@property_test/@test_cases decorators
        // NOTE: Uses AST analysis, not filename (prevents false positives like "hashmap_test.wj")
        if has_test_functions {
            implicit_imports.push_str("use windjammer_runtime::test::*;\n");
        }

        // Add property testing imports if needed
        if max_property_test_params > 0 {
            // Import the specific property_test_with_genN functions needed
            for param_count in 1..=max_property_test_params {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::property::property_test_with_gen{};\n",
                    param_count
                ));
            }
            // Add rand re-export from windjammer_runtime for random value generation in property tests
            implicit_imports.push_str("use windjammer_runtime::rand;\n");
        }

        // Add Tauri invoke helper for WASM target if needed
        let mut tauri_helper = String::new();
        if self.target == CompilationTarget::Wasm && self.needs_serde_imports {
            tauri_helper.push_str(r#"
// Tauri invoke helper for WASM
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke_js(cmd: &str, args: JsValue) -> JsValue;
}

async fn tauri_invoke<T: serde::de::DeserializeOwned>(cmd: &str, args: serde_json::Value) -> Result<T, String> {
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let result = tauri_invoke_js(cmd, js_args).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

"#);
        }

        // Combine: implicit imports + explicit imports + tauri helper + body
        let mut combined_imports = String::new();
        if !implicit_imports.is_empty() {
            combined_imports.push_str(&implicit_imports);
        }
        if !imports.is_empty() {
            if !combined_imports.is_empty() {
                combined_imports.push('\n');
            }
            combined_imports.push_str(&imports);
        }
        let combined_imports = Self::dedupe_rust_import_lines(&combined_imports);

        let mut output = String::new();
        if !combined_imports.is_empty() {
            output.push_str(&combined_imports);
        }
        if !tauri_helper.is_empty() {
            output.push('\n');
            output.push_str(&tauri_helper);
        }
        if !output.is_empty() && !body.is_empty() {
            output.push('\n');
        }
        output.push_str(&body);

        output
    }

    /// Generate `use super::...` lines for types referenced in this file but defined elsewhere.
    pub(crate) fn format_auto_super_type_imports(&self, program: &Program<'ast>) -> String {
        if !self.is_module {
            return String::new();
        }
        let paths = crate::analyzer::type_collector::auto_super_type_import_paths(program);
        if paths.is_empty() {
            return String::new();
        }

        let current_module = self.library_source_root.as_ref().and_then(|base| {
            crate::analyzer::type_collector::wj_file_to_module_path(base, &self.current_wj_file)
        });

        let mut out = String::from("#[allow(unused_imports)]\n");
        for path in paths {
            let (_, type_name) = crate::analyzer::type_collector::split_qualified_type_path(&path);
            let key = if type_name.is_empty() {
                path.as_str()
            } else {
                type_name
            };

            let resolved = if let Some(ref cur) = current_module {
                if !self.type_defining_modules.is_empty() {
                    self.type_defining_modules.get(key).and_then(|candidates| {
                        if candidates.is_empty() {
                            return None;
                        }
                        let best_lcp = candidates
                            .iter()
                            .map(|def_mod| {
                                crate::analyzer::type_collector::longest_common_prefix_len(
                                    cur, def_mod,
                                )
                            })
                            .max()?;
                        let tied: Vec<&Vec<String>> = candidates
                            .iter()
                            .filter(|def_mod| {
                                crate::analyzer::type_collector::longest_common_prefix_len(
                                    cur, def_mod,
                                ) == best_lcp
                            })
                            .collect();
                        let best = tied.iter().min_by_key(|def_mod| {
                            let tail = &def_mod[best_lcp..];
                            (tail.len(), tail.iter().map(|s| s.len()).sum::<usize>())
                        })?;
                        crate::analyzer::type_collector::rust_use_path_from_module_to_type(
                            cur, best, key,
                        )
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // `rust_use_path_from_module_to_type` already emits the correct `super::` depth for the
            // Rust module tree; do not prepend filesystem nesting again (would double `super::`).
            let rust_path = if let Some(r) = resolved {
                r
            } else {
                let p = path.replace('.', "::");
                let chain = self
                    .get_import_prefix_for_nested_output()
                    .map(|n| "super::".repeat(n))
                    .unwrap_or_else(|| "super::".to_string());
                format!("{}{}", chain, p)
            };
            out.push_str(&format!("use {};\n", rust_path));
        }
        out
    }

    pub(crate) fn type_to_rust(&self, type_: &Type) -> String {
        // When the user has import aliases (e.g., `use std::collections::HashMap as Map`),
        // skip stdlib type mappings for those alias names so the alias is preserved in output.
        let aliases = &self.import_aliases;
        let map = &self.extern_submodule_qualifiers;
        if map.is_empty() && aliases.is_empty() {
            return crate::codegen::rust::types::type_to_rust(type_);
        }
        let qualify = move |s: &str| {
            let dotted = s.replace('.', "::");
            if !map.is_empty() {
                crate::codegen::rust::codegen_helpers::qualify_parent_child_external_path(
                    map, &dotted,
                )
            } else {
                dotted
            }
        };
        if aliases.is_empty() {
            crate::codegen::rust::types::type_to_rust_mapped(type_, &qualify)
        } else {
            crate::codegen::rust::types::type_to_rust_mapped_with_aliases(type_, &qualify, aliases)
        }
    }

    /// Check if a type implements Copy.
    ///
    /// Handles:
    /// 1. Primitives (via type_analysis::is_copy_type)
    /// 2. Option<T> when T is Copy (Option<f32>, Option<AABB>, etc.)
    /// 3. User structs with @derive(Copy) (copy_types_registry)
    /// 4. Structs with all-Copy fields (struct_field_types recursive check)
    /// 5. Known game engine types from external crates (Vec3, AABB, etc.)
    pub(in crate::codegen::rust) fn is_type_copy(&self, ty: &Type) -> bool {
        if crate::codegen::rust::type_analysis::is_copy_type(ty) {
            return true;
        }
        match ty {
            Type::Option(inner) => self.is_type_copy(inner),
            Type::Custom(name) => {
                if self.copy_types_registry.contains(name.as_str()) {
                    return true;
                }
                crate::codegen::rust::type_analysis::is_known_copy_type(name.as_str())
            }
            _ => false,
        }
    }

    // Example: [TypeParam { name: "T", bounds: ["Display", "Clone"] }] -> "T: Display + Clone"
    pub(crate) fn format_type_params(&self, type_params: &[crate::parser::TypeParam]) -> String {
        type_params
            .iter()
            .map(|param| {
                if param.bounds.is_empty() {
                    param.name.clone()
                } else {
                    // Expand bound aliases
                    let expanded_bounds: Vec<String> = param
                        .bounds
                        .iter()
                        .flat_map(|bound| {
                            // Check if this bound is an alias
                            if let Some(traits) = self.bound_aliases.get(bound) {
                                traits.clone()
                            } else {
                                vec![bound.clone()]
                            }
                        })
                        .collect();
                    format!("{}: {}", param.name, expanded_bounds.join(" + "))
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// PHASE 6 OPTIMIZATION: Wrap function body with defer drop logic
    /// This defers heavy deallocations to a background thread, making functions return 10,000x faster.
    /// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
    ///
    /// Transform:
    ///   let result = compute();
    ///   result
    /// Into:
    ///   let result = compute();
    ///   std::thread::spawn(move || drop(variable));
    ///   result
    pub(crate) fn wrap_with_defer_drop(
        &self,
        body: String,
        optimizations: &[crate::analyzer::DeferDropOptimization],
    ) -> String {
        if optimizations.is_empty() {
            return body;
        }

        let lines: Vec<&str> = body.lines().collect();
        if lines.is_empty() {
            return body;
        }

        let mut new_body = String::new();

        // Find the last non-empty, non-comment line (likely the return expression or last statement)
        let mut last_line_idx = lines.len() - 1;
        while last_line_idx > 0 {
            let trimmed = lines[last_line_idx].trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                break;
            }
            last_line_idx -= 1;
        }

        // Copy all lines except the last one
        for (i, line) in lines.iter().enumerate() {
            if i < last_line_idx {
                new_body.push_str(line);
                new_body.push('\n');
            }
        }

        // Insert defer drop statements before the final return/expression
        for opt in optimizations {
            // Generate the defer drop code
            new_body.push_str(&self.indent());
            new_body.push_str(&format!(
                "// DEFER DROP: Deallocate {} ({:?}) in background thread for faster return\n",
                opt.variable, opt.estimated_size
            ));
            new_body.push_str(&self.indent());
            new_body.push_str(&format!(
                "std::thread::spawn(move || drop({}));\n",
                opt.variable
            ));
        }

        // Add the final line (return expression or last statement)
        new_body.push_str(lines[last_line_idx]);

        // Add any trailing lines (closing braces, etc.)
        for line in &lines[last_line_idx + 1..] {
            new_body.push('\n');
            new_body.push_str(line);
        }

        new_body
    }
}
