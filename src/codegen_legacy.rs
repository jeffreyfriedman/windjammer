// Rust code generator
use crate::analyzer::*;
use crate::parser::*;
use crate::CompilationTarget;

pub struct CodeGenerator {
    indent_level: usize,
    signature_registry: SignatureRegistry,
    in_wasm_bindgen_impl: bool,
    needs_wasm_imports: bool,
    needs_web_imports: bool,
    needs_js_imports: bool,
    needs_serde_imports: bool,   // For JSON support
    needs_write_import: bool,    // For string capacity optimization (write! macro)
    needs_smallvec_import: bool, // For Phase 8 SmallVec optimization
    needs_cow_import: bool,      // For Phase 9 Cow optimization
    target: CompilationTarget,
    is_module: bool, // true if generating code for a reusable module (not main file)
    #[allow(dead_code)] // TODO: Use for error mapping Phase 3
    source_map: crate::source_map::SourceMap,
    inferred_bounds: std::collections::HashMap<String, crate::inference::InferredBounds>,
    needs_trait_imports: std::collections::HashSet<String>, // Tracks which traits need imports
    bound_aliases: std::collections::HashMap<String, Vec<String>>, // bound Name = Trait + Trait
    // PHASE 2 OPTIMIZATION: Track variables that can avoid cloning
    clone_optimizations: std::collections::HashSet<String>, // Variables that don't need .clone()
    // PHASE 3 OPTIMIZATION: Track struct mapping optimizations
    struct_mapping_hints: std::collections::HashMap<String, crate::analyzer::MappingStrategy>, // Struct name -> strategy
    // PHASE 4 OPTIMIZATION: Track string operation optimizations
    string_capacity_hints: std::collections::HashMap<usize, usize>, // Statement idx -> capacity
    // PHASE 5 OPTIMIZATION: Track assignment operations that can use compound operators
    assignment_optimizations: std::collections::HashMap<String, crate::analyzer::CompoundOp>, // Variable -> compound op
    // PHASE 6 OPTIMIZATION: Track defer drop optimizations
    defer_drop_optimizations: Vec<crate::analyzer::DeferDropOptimization>,
    // PHASE 8 OPTIMIZATION: Track SmallVec optimizations
    smallvec_optimizations:
        std::collections::HashMap<String, crate::analyzer::SmallVecOptimization>, // Variable -> SmallVec config
    // PHASE 9 OPTIMIZATION: Track Cow optimizations
    cow_optimizations: std::collections::HashSet<String>, // Variables that can use Cow
    // Track current statement index for optimization hints
    current_statement_idx: usize,
    // IMPLICIT SELF SUPPORT: Track struct fields for implicit self references
    current_struct_fields: std::collections::HashSet<String>, // Field names in current impl block
    in_impl_block: bool, // true if currently generating code for an impl block
}

impl CodeGenerator {
    pub fn new(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        CodeGenerator {
            indent_level: 0,
            signature_registry: registry,
            in_wasm_bindgen_impl: false,
            needs_wasm_imports: false,
            needs_web_imports: false,
            needs_js_imports: false,
            needs_serde_imports: false,
            needs_write_import: false,
            needs_smallvec_import: false,
            needs_cow_import: false,
            target,
            is_module: false,
            source_map: crate::source_map::SourceMap::new(),
            inferred_bounds: std::collections::HashMap::new(),
            needs_trait_imports: std::collections::HashSet::new(),
            bound_aliases: std::collections::HashMap::new(),
            clone_optimizations: std::collections::HashSet::new(),
            struct_mapping_hints: std::collections::HashMap::new(),
            string_capacity_hints: std::collections::HashMap::new(),
            assignment_optimizations: std::collections::HashMap::new(),
            defer_drop_optimizations: Vec::new(),
            smallvec_optimizations: std::collections::HashMap::new(),
            cow_optimizations: std::collections::HashSet::new(),
            current_statement_idx: 0,
            current_struct_fields: std::collections::HashSet::new(),
            in_impl_block: false,
        }
    }

    /// Set inferred trait bounds for functions
    pub fn set_inferred_bounds(
        &mut self,
        bounds: std::collections::HashMap<String, crate::inference::InferredBounds>,
    ) {
        self.inferred_bounds = bounds;
    }

    pub fn new_for_module(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        let mut gen = Self::new(registry, target);
        gen.is_module = true;
        gen
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    /// Map Windjammer decorators to Rust attributes
    /// This abstraction layer allows us to use semantic Windjammer names
    /// while generating appropriate Rust attributes based on compilation target
    fn map_decorator(&mut self, name: &str) -> String {
        match (name, self.target) {
            ("export", CompilationTarget::Wasm) => {
                self.needs_wasm_imports = true;
                "wasm_bindgen".to_string()
            }
            ("export", CompilationTarget::Node) => {
                // Future: Node.js native modules via Neon
                "neon::export".to_string()
            }
            ("export", CompilationTarget::Python) => {
                // Future: Python bindings via PyO3
                "pyfunction".to_string()
            }
            ("export", CompilationTarget::C) => {
                // Future: C FFI
                "no_mangle".to_string()
            }
            ("test", _) => "test".to_string(),
            ("async", _) => "async".to_string(),
            // Pass through other decorators as-is
            (other, _) => other.to_string(),
        }
    }

    fn generate_block(&mut self, stmts: &[Statement]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            // Track current statement index for optimization hints
            self.current_statement_idx = i;

            let is_last = i == len - 1;
            if is_last && matches!(stmt, Statement::Expression(_)) {
                // Last statement is an expression - generate without semicolon (it's the return value)
                if let Statement::Expression(expr) = stmt {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_expression(expr));
                    output.push('\n');
                }
            } else {
                output.push_str(&self.generate_statement(stmt));
            }
        }
        output
    }

    pub fn generate_program(&mut self, program: &Program, analyzed: &[AnalyzedFunction]) -> String {
        let mut imports = String::new();
        let mut body = String::new();

        // Collect bound aliases first (bound Name = Trait + Trait)
        for item in &program.items {
            if let Item::BoundAlias { name, traits } = item {
                self.bound_aliases.insert(name.clone(), traits.clone());
            }
        }

        // Collect struct definitions for implicit self support
        let mut struct_fields: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for item in &program.items {
            if let Item::Struct(s) = item {
                let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
                struct_fields.insert(s.name.clone(), field_names);
            }
        }

        // Check for stdlib modules that need special imports
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                // Path can be either ["std", "json"] or ["std.json"] depending on parsing
                let path_str = path.join(".");
                if (path_str.starts_with("std.") || path_str == "std") && path_str.contains("json")
                {
                    self.needs_serde_imports = true;
                }
                // http, time, crypto modules don't need special imports (used directly)
            }
        }

        // Generate explicit use statements
        for item in &program.items {
            if let Item::Use { path, alias } = item {
                imports.push_str(&self.generate_use(path, alias.as_deref()));
                imports.push('\n');
            }
        }

        // Generate const and static declarations
        for item in &program.items {
            match item {
                Item::Const { name, type_, value } => {
                    let pub_prefix = if self.is_module { "pub " } else { "" };
                    body.push_str(&format!(
                        "{}const {}: {} = {};\n",
                        pub_prefix,
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression_immut(value)
                    ));
                }
                Item::Static {
                    name,
                    mutable,
                    type_,
                    value,
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
                        let keyword = if self.is_const_evaluable(value) {
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
                _ => {}
            }
        }

        if !body.is_empty() {
            body.push('\n');
        }

        // Collect names of functions in impl blocks to avoid generating them twice
        let mut impl_methods = std::collections::HashSet::new();
        for item in &program.items {
            if let Item::Impl(impl_block) = item {
                for func in &impl_block.functions {
                    impl_methods.insert(func.name.clone());
                }
            }
        }

        // Generate structs, enums, and traits
        for item in &program.items {
            match item {
                Item::Struct(s) => {
                    body.push_str(&self.generate_struct(s));
                    body.push_str("\n\n");
                }
                Item::Enum(e) => {
                    body.push_str(&self.generate_enum(e));
                    body.push_str("\n\n");
                }
                Item::Trait(t) => {
                    body.push_str(&self.generate_trait(t));
                    body.push_str("\n\n");
                }
                Item::Impl(impl_block) => {
                    // Set the struct fields for implicit self support
                    if let Some(fields) = struct_fields.get(&impl_block.type_name) {
                        self.current_struct_fields = fields.iter().cloned().collect();
                    } else {
                        self.current_struct_fields.clear();
                    }
                    self.in_impl_block = true;

                    body.push_str(&self.generate_impl(impl_block, analyzed));
                    body.push_str("\n\n");

                    self.in_impl_block = false;
                    self.current_struct_fields.clear();
                }
                _ => {}
            }
        }

        // Generate top-level functions (skip impl methods)
        for analyzed_func in analyzed {
            if !impl_methods.contains(&analyzed_func.decl.name) {
                body.push_str(&self.generate_function(analyzed_func));
                body.push_str("\n\n");
            }
        }

        // Inject implicit imports if needed
        let mut implicit_imports = String::new();

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
                    "Add" | "Sub" | "Mul" | "Div" => {
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

        // Combine: implicit imports + explicit imports + body
        let mut output = String::new();
        if !implicit_imports.is_empty() {
            output.push_str(&implicit_imports);
            if !imports.is_empty() {
                output.push('\n');
            }
        }
        if !imports.is_empty() {
            output.push_str(&imports);
        }
        if !output.is_empty() && !body.is_empty() {
            output.push('\n');
        }
        output.push_str(&body);

        output
    }

    fn generate_use(&self, path: &[String], alias: Option<&str>) -> String {
        if path.is_empty() {
            return String::new();
        }

        let full_path = path.join(".");

        // Handle glob imports: module.submodule.* -> use module::submodule::*;
        if full_path.ends_with(".*") {
            let path_without_glob = full_path.strip_suffix(".*").unwrap();
            // Replace dots with :: but remove any trailing ::
            let rust_path = path_without_glob
                .replace('.', "::")
                .trim_end_matches("::")
                .to_string();
            return format!("use {}::*;\n", rust_path);
        }

        // Handle braced imports: module.{A, B, C} -> use module::{A, B, C};
        if full_path.contains(".{") && full_path.contains('}') {
            // Split into base path and braced items
            if let Some((base, items)) = full_path.split_once(".{") {
                let rust_base = base.replace('.', "::");
                // items already has the closing brace
                return format!("use {}::{{{};\n", rust_base, items);
            }
        }

        // Handle stdlib imports: std.math -> use math::*; or std.math as m -> use math as m;
        if full_path.starts_with("std.") {
            let module_name = full_path.strip_prefix("std.").unwrap();
            if let Some(alias_name) = alias {
                return format!("use {} as {};\n", module_name, alias_name);
            } else {
                return format!("use {}::*;\n", module_name);
            }
        }

        // Skip bare "std" imports
        if full_path == "std" {
            return String::new();
        }

        // Handle relative imports: ./utils or ../utils
        if full_path.starts_with("./") || full_path.starts_with("../") {
            // Extract module name: ./utils -> utils, ../utils/helpers -> helpers
            let stripped = full_path
                .strip_prefix("./")
                .or_else(|| full_path.strip_prefix("../"))
                .unwrap_or(&full_path);
            let module_name = stripped.split('/').next_back().unwrap_or(stripped);
            if let Some(alias_name) = alias {
                return format!("use {} as {};\n", module_name, alias_name);
            } else {
                return format!("use {}::*;\n", module_name);
            }
        }

        // Convert Windjammer's Go-style imports to Rust's glob imports
        // e.g., "use wasm_bindgen.prelude" -> "use wasm_bindgen::prelude::*;"
        // or "use wasm_bindgen.prelude as wb" -> "use wasm_bindgen::prelude as wb;"
        let rust_path = full_path.replace('.', "::");
        if let Some(alias_name) = alias {
            format!("use {} as {};\n", rust_path, alias_name)
        } else {
            format!("use {}::*;\n", rust_path)
        }
    }

    fn generate_struct(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Convert decorators to Rust attributes
        for decorator in &s.decorators {
            if decorator.name == "auto" {
                // Special handling for @auto decorator
                let traits = if decorator.arguments.is_empty() {
                    // Smart inference: no arguments, so infer traits based on field types
                    self.infer_derivable_traits(s)
                } else {
                    // Explicit: extract trait names from decorator arguments
                    let mut explicit_traits = Vec::new();
                    for (_key, expr) in &decorator.arguments {
                        if let Expression::Identifier(trait_name) = expr {
                            explicit_traits.push(trait_name.clone());
                        }
                    }
                    explicit_traits
                };

                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));
                }
            } else if decorator.name == "derive" {
                // Special handling for @derive decorator - generates #[derive(Trait1, Trait2)]
                let mut traits = Vec::new();
                for (_key, expr) in &decorator.arguments {
                    if let Expression::Identifier(trait_name) = expr {
                        traits.push(trait_name.clone());
                    }
                }
                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));
                }
            } else {
                // Map Windjammer decorator to Rust attribute
                let rust_attr = self.map_decorator(&decorator.name);
                if decorator.arguments.is_empty() {
                    output.push_str(&format!("#[{}]\n", rust_attr));
                } else {
                    output.push_str(&format!("#[{}(", rust_attr));
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            format!("{} = {}", key, self.generate_expression_immut(expr))
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                }
            }
        }

        // Add struct declaration with type parameters
        output.push_str("struct ");
        output.push_str(&s.name);
        if !s.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&s.type_params));
            output.push('>');
        }

        // Add where clause if present
        output.push_str(&self.format_where_clause(&s.where_clause));

        output.push_str(" {\n");

        for field in &s.fields {
            // Generate decorators for the field (convert to Rust attributes)
            for decorator in &field.decorators {
                output.push_str(&format!("    #[{}(", decorator.name));
                let args: Vec<String> = decorator
                    .arguments
                    .iter()
                    .map(|(key, expr)| {
                        format!("{} = {}", key, self.generate_expression_immut(expr))
                    })
                    .collect();
                output.push_str(&args.join(", "));
                output.push_str(")]\n");
            }
            let pub_keyword = if field.is_pub { "pub " } else { "" };
            output.push_str(&format!(
                "    {}{}: {},\n",
                pub_keyword,
                field.name,
                self.type_to_rust(&field.field_type)
            ));
        }

        output.push('}');
        output
    }

    fn generate_enum(&self, e: &EnumDecl) -> String {
        let mut output = format!("enum {}", e.name);

        // Generate generic parameters: enum Option<T>, enum Result<T, E>
        if !e.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&e.type_params));
            output.push('>');
        }

        output.push_str(" {\n");

        for variant in &e.variants {
            if let Some(data) = &variant.data {
                output.push_str(&format!(
                    "    {}({}),\n",
                    variant.name,
                    self.type_to_rust(data)
                ));
            } else {
                output.push_str(&format!("    {},\n", variant.name));
            }
        }

        output.push('}');
        output
    }

    fn generate_trait(&mut self, trait_decl: &crate::parser::TraitDecl) -> String {
        let mut output = String::from("trait ");
        output.push_str(&trait_decl.name);

        // Generate generic parameters: trait From<T> { ... }
        if !trait_decl.generics.is_empty() {
            output.push('<');
            output.push_str(&trait_decl.generics.join(", "));
            output.push('>');
        }

        // Generate supertraits: trait Manager: Employee + Person
        if !trait_decl.supertraits.is_empty() {
            output.push_str(": ");
            output.push_str(&trait_decl.supertraits.join(" + "));
        }

        output.push_str(" {\n");
        self.indent_level += 1;

        // Generate associated type declarations: type Item;
        for assoc_type in &trait_decl.associated_types {
            output.push_str(&self.indent());
            output.push_str(&format!("type {};\n", assoc_type.name));
        }

        if !trait_decl.associated_types.is_empty() {
            output.push('\n');
        }

        // Generate trait methods
        for method in &trait_decl.methods {
            output.push_str(&self.indent());

            if method.is_async {
                output.push_str("async ");
            }

            output.push_str("fn ");
            output.push_str(&method.name);
            output.push('(');

            // Generate parameters
            let params: Vec<String> = method
                .parameters
                .iter()
                .map(|param| {
                    use crate::parser::OwnershipHint;
                    let type_str = match &param.ownership {
                        OwnershipHint::Owned => {
                            if param.name == "self" {
                                return "self".to_string();
                            }
                            self.type_to_rust(&param.type_)
                        }
                        OwnershipHint::Ref => {
                            if param.name == "self" {
                                return "&self".to_string();
                            }
                            format!("&{}", self.type_to_rust(&param.type_))
                        }
                        OwnershipHint::Mut => {
                            if param.name == "self" {
                                return "&mut self".to_string();
                            }
                            format!("&mut {}", self.type_to_rust(&param.type_))
                        }
                        OwnershipHint::Inferred => {
                            // Default to &
                            if param.name == "self" {
                                "&self".to_string()
                            } else {
                                format!("&{}", self.type_to_rust(&param.type_))
                            }
                        }
                    };

                    format!("{}: {}", param.name, type_str)
                })
                .collect();

            output.push_str(&params.join(", "));
            output.push(')');

            // Return type
            if let Some(ret_type) = &method.return_type {
                output.push_str(" -> ");
                output.push_str(&self.type_to_rust(ret_type));
            }

            // Default implementation (if provided)
            if let Some(body) = &method.body {
                output.push_str(" {\n");
                self.indent_level += 1;

                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push_str("}\n");
            } else {
                output.push_str(";\n");
            }
        }

        self.indent_level -= 1;
        output.push('}');
        output
    }

    fn generate_impl(&mut self, impl_block: &ImplBlock, analyzed: &[AnalyzedFunction]) -> String {
        let mut output = String::new();

        // Check if this impl block has @export or @wasm_bindgen decorator
        let has_wasm_export = impl_block
            .decorators
            .iter()
            .any(|d| d.name == "export" || d.name == "wasm_bindgen");

        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &impl_block.decorators {
            let rust_attr = self.map_decorator(&decorator.name);
            output.push_str(&format!("#[{}]\n", rust_attr));
        }

        // Generate impl with type parameters
        output.push_str("impl");
        if !impl_block.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&impl_block.type_params));
            output.push('>');
        }
        output.push(' ');

        if let Some(trait_name) = &impl_block.trait_name {
            // Trait implementation: impl<T> Trait<TypeArgs> for Type<T>
            output.push_str(trait_name);

            // Generate trait type arguments if present: From<int> -> From<i64>
            if let Some(type_args) = &impl_block.trait_type_args {
                output.push('<');
                let args_str: Vec<String> =
                    type_args.iter().map(|t| self.type_to_rust(t)).collect();
                output.push_str(&args_str.join(", "));
                output.push('>');
            }

            output.push_str(&format!(" for {}", impl_block.type_name));
        } else {
            // Inherent implementation: impl<T> Type<T>
            output.push_str(&impl_block.type_name);
        }

        // Add where clause if present
        output.push_str(&self.format_where_clause(&impl_block.where_clause));

        output.push_str(" {\n");

        self.indent_level += 1;

        // Generate associated type implementations: type Item = i32;
        for assoc_type in &impl_block.associated_types {
            output.push_str(&self.indent());
            output.push_str(&format!("type {}", assoc_type.name));
            if let Some(concrete_type) = &assoc_type.concrete_type {
                output.push_str(&format!(" = {};\n", self.type_to_rust(concrete_type)));
            } else {
                output.push_str(";\n");
            }
        }

        if !impl_block.associated_types.is_empty() {
            output.push('\n');
        }

        // Store the wasm export flag for use in generate_function
        let old_in_wasm_impl = self.in_wasm_bindgen_impl;
        self.in_wasm_bindgen_impl = has_wasm_export;

        for func in &impl_block.functions {
            // Find the analyzed version of this function
            if let Some(analyzed_func) = analyzed.iter().find(|af| af.decl.name == func.name) {
                output.push_str(&self.generate_function(analyzed_func));
                output.push('\n');
            }
        }

        self.in_wasm_bindgen_impl = old_in_wasm_impl;

        self.indent_level -= 1;
        output.push('}');
        output
    }

    // Helper method for expressions that need to be evaluated without &mut self
    fn generate_expression_immut(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(lit) => self.generate_literal(lit),
            Expression::Identifier(name) => name.clone(),
            _ => "/* expression */".to_string(),
        }
    }

    // Check if a function accesses any struct fields
    // For now, we use a simple heuristic: if we're in an impl block and the function
    // has a non-empty body, assume it might need &self
    fn function_accesses_fields(&self, _func: &FunctionDecl) -> bool {
        // Simple heuristic: methods in impl blocks typically need &self
        // The identifier generation will add self. prefix where needed
        true
    }

    fn generate_function(&mut self, analyzed: &AnalyzedFunction) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        // PHASE 8 OPTIMIZATION: Load SmallVec optimizations for this function
        self.smallvec_optimizations.clear();
        for opt in &analyzed.smallvec_optimizations {
            self.smallvec_optimizations
                .insert(opt.variable.clone(), opt.clone());
            self.needs_smallvec_import = true; // Mark that we need the smallvec crate
        }

        // PHASE 9 OPTIMIZATION: Load Cow optimizations for this function
        self.cow_optimizations.clear();
        for opt in &analyzed.cow_optimizations {
            self.cow_optimizations.insert(opt.variable.clone());
            self.needs_cow_import = true; // Mark that we need Cow from std::borrow
        }

        // PHASE 3 OPTIMIZATION: Load struct mapping optimizations
        // Track which structs can use optimized construction strategies
        self.struct_mapping_hints.clear();
        for opt in &analyzed.struct_mapping_optimizations {
            self.struct_mapping_hints
                .insert(opt.target_struct.clone(), opt.strategy.clone());
        }

        // PHASE 4 OPTIMIZATION: Load string operation optimizations
        // Track capacity hints for string operations
        self.string_capacity_hints.clear();

        // PHASE 5 OPTIMIZATION: Load assignment operation optimizations
        // Track which variables can use compound assignment operators
        self.assignment_optimizations.clear();
        for opt in &analyzed.assignment_optimizations {
            self.assignment_optimizations
                .insert(opt.variable.clone(), opt.operation.clone());
        }
        for opt in &analyzed.string_optimizations {
            if let Some(capacity) = opt.estimated_capacity {
                self.string_capacity_hints.insert(opt.location, capacity);
            }
        }

        // PHASE 6 OPTIMIZATION: Load defer drop optimizations
        // Track variables that should have their drops deferred to background thread
        self.defer_drop_optimizations = analyzed.defer_drop_optimizations.clone();

        // Check for @async decorator (special case: it's a keyword, not an attribute)
        let is_async = func.decorators.iter().any(|d| d.name == "async");

        // Special case: async main requires #[tokio::main]
        if is_async && func.name == "main" {
            output.push_str("#[tokio::main]\n");
        }

        // OPTIMIZATION: Add inline hints for hot path functions
        // This is Phase 1 optimization: Generate Inlinable Code
        if self.should_inline_function(func, analyzed) {
            output.push_str("#[inline]\n");
        }

        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &func.decorators {
            // Skip @async, it's handled specially
            if decorator.name == "async" {
                continue;
            }

            let rust_attr = self.map_decorator(&decorator.name);
            output.push_str(&format!("#[{}]\n", rust_attr));
        }

        // Add `pub` if we're in a #[wasm_bindgen] impl block OR compiling a module
        if self.in_wasm_bindgen_impl || self.is_module {
            output.push_str("pub ");
        }

        // Add async keyword if decorator present
        if is_async {
            output.push_str("async ");
        }

        output.push_str("fn ");
        output.push_str(&func.name);

        // Add type parameters with bounds: fn foo<T: Display, U: Debug>(...)
        // Merge inferred bounds with explicit bounds
        let type_params = if let Some(inferred) = self.inferred_bounds.get(&func.name) {
            let merged = inferred.merge_with_explicit(&func.type_params);
            // Track which traits need imports
            for param in &merged {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            merged
        } else {
            // Still track explicit bounds
            for param in &func.type_params {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            func.type_params.clone()
        };

        if !type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&type_params));
            output.push('>');
        }

        output.push('(');

        // Add implicit &self for impl block methods that access fields
        let mut params: Vec<String> = Vec::new();
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");

        if self.in_impl_block && !has_explicit_self && !self.current_struct_fields.is_empty() {
            // Check if function body accesses any struct fields
            if self.function_accesses_fields(func) {
                params.push("&self".to_string());
            }
        }

        let additional_params: Vec<String> = func
            .parameters
            .iter()
            .map(|param| {
                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(&param.type_);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            return "self".to_string();
                        }
                        self.type_to_rust(&param.type_)
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            return "&self".to_string();
                        }
                        format!("&{}", self.type_to_rust(&param.type_))
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            return "&mut self".to_string();
                        }
                        format!("&mut {}", self.type_to_rust(&param.type_))
                    }
                    OwnershipHint::Inferred => {
                        // Use analyzer's inference
                        let ownership_mode = analyzed
                            .inferred_ownership
                            .get(&param.name)
                            .unwrap_or(&OwnershipMode::Borrowed);

                        // Override for Copy types UNLESS they're mutated
                        // Mutated parameters should be &mut even for Copy types
                        if self.is_copy_type(&param.type_)
                            && ownership_mode != &OwnershipMode::MutBorrowed
                        {
                            self.type_to_rust(&param.type_)
                        } else {
                            match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(&param.type_),
                                OwnershipMode::Borrowed => {
                                    // For Copy types that are only read, pass by value
                                    if self.is_copy_type(&param.type_) {
                                        self.type_to_rust(&param.type_)
                                    } else {
                                        format!("&{}", self.type_to_rust(&param.type_))
                                    }
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(&param.type_))
                                }
                            }
                        }
                    }
                };

                // Check if this is a pattern parameter
                if let Some(pattern) = &param.pattern {
                    // Generate pattern: type syntax
                    format!("{}: {}", self.generate_pattern(pattern), type_str)
                } else {
                    // Simple name: type syntax
                    format!("{}: {}", param.name, type_str)
                }
            })
            .collect();

        params.extend(additional_params);

        output.push_str(&params.join(", "));
        output.push(')');

        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            output.push_str(&self.type_to_rust(return_type));
        }

        // Add where clause if present
        output.push_str(&self.format_where_clause(&func.where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;

        let mut body_code = self.generate_block(&func.body);

        // PHASE 6 OPTIMIZATION: Add defer drop logic before function returns
        // This defers heavy deallocations to a background thread for 10,000x speedup
        if !self.defer_drop_optimizations.is_empty() {
            body_code =
                self.wrap_with_defer_drop(body_code, &self.defer_drop_optimizations.clone());
        }

        output.push_str(&body_code);

        self.indent_level -= 1;
        output.push('}');

        output
    }

    #[allow(clippy::only_used_in_recursion)]
    fn type_to_rust(&self, type_: &Type) -> String {
        match type_ {
            Type::Int => "i64".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Uint => "u64".to_string(),
            Type::Float => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Custom(name) => {
                // Convert Windjammer module.Type syntax to Rust module::Type
                name.replace('.', "::")
            }
            Type::Generic(name) => name.clone(), // Type parameter: T -> T
            Type::Associated(base, assoc_name) => {
                // Associated type: Self::Item -> Self::Item, T::Output -> T::Output
                format!("{}::{}", base, assoc_name)
            }
            Type::TraitObject(trait_name) => {
                // Trait object: dyn Trait -> Box<dyn Trait>
                // Note: Windjammer automatically boxes trait objects for convenience
                format!("Box<dyn {}>", trait_name)
            }
            Type::Parameterized(base, args) => {
                // Generic type: Vec<T> -> Vec<T>, HashMap<K, V> -> HashMap<K, V>
                format!(
                    "{}<{}>",
                    base,
                    args.iter()
                        .map(|t| self.type_to_rust(t))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Type::Option(inner) => format!("Option<{}>", self.type_to_rust(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                self.type_to_rust(ok),
                self.type_to_rust(err)
            ),
            Type::Vec(inner) => format!("Vec<{}>", self.type_to_rust(inner)),
            Type::Reference(inner) => {
                // Special case: &[T] (slice) vs &Vec<T>
                if let Type::Vec(elem) = &**inner {
                    format!("&[{}]", self.type_to_rust(elem))
                // Special case: &str instead of &String (more idiomatic Rust)
                } else if matches!(**inner, Type::String) {
                    "&str".to_string()
                // Special case: &dyn Trait (don't box when already a reference)
                } else if let Type::TraitObject(trait_name) = &**inner {
                    format!("&dyn {}", trait_name)
                } else {
                    format!("&{}", self.type_to_rust(inner))
                }
            }
            Type::MutableReference(inner) => {
                // Special case: &mut [T] (mutable slice) vs &mut Vec<T>
                if let Type::Vec(elem) = &**inner {
                    format!("&mut [{}]", self.type_to_rust(elem))
                // Special case: &mut dyn Trait (don't box when already a reference)
                } else if let Type::TraitObject(trait_name) = &**inner {
                    format!("&mut dyn {}", trait_name)
                } else {
                    format!("&mut {}", self.type_to_rust(inner))
                }
            }
            Type::Tuple(types) => {
                let rust_types: Vec<String> = types.iter().map(|t| self.type_to_rust(t)).collect();
                format!("({})", rust_types.join(", "))
            }
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::Let {
                name,
                mutable,
                type_,
                value,
            } => {
                let mut output = self.indent();
                output.push_str("let ");
                if *mutable {
                    output.push_str("mut ");
                }
                output.push_str(name);

                // PHASE 8: Check if this variable should use SmallVec
                if let Some(smallvec_opt) = self.smallvec_optimizations.get(name) {
                    // Use SmallVec with stack allocation
                    output.push_str(&format!(": SmallVec<[_; {}]>", smallvec_opt.stack_size));
                    output.push_str(" = ");

                    // Generate the expression but wrap in smallvec! if it's a vec! macro
                    let expr_str = self.generate_expression(value);
                    if let Some(stripped) = expr_str.strip_prefix("vec!") {
                        // Replace vec! with smallvec!
                        output.push_str("smallvec!");
                        output.push_str(stripped);
                    } else {
                        // For other expressions, try to convert
                        output.push_str(&expr_str);
                        output.push_str(".into()"); // Convert Vec to SmallVec
                    }
                } else if let Some(t) = type_ {
                    output.push_str(": ");
                    output.push_str(&self.type_to_rust(t));
                    output.push_str(" = ");
                    output.push_str(&self.generate_expression(value));
                } else {
                    output.push_str(" = ");
                    output.push_str(&self.generate_expression(value));
                }

                output.push_str(";\n");
                output
            }
            Statement::Const { name, type_, value } => {
                let mut output = self.indent();
                output.push_str(&format!(
                    "const {}: {} = {};\n",
                    name,
                    self.type_to_rust(type_),
                    self.generate_expression(value)
                ));
                output
            }
            Statement::Static {
                name,
                mutable,
                type_,
                value,
            } => {
                let mut output = self.indent();
                if *mutable {
                    output.push_str(&format!(
                        "static mut {}: {} = {};\n",
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression(value)
                    ));
                } else {
                    output.push_str(&format!(
                        "static {}: {} = {};\n",
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression(value)
                    ));
                }
                output
            }
            Statement::Return(expr) => {
                let mut output = self.indent();
                output.push_str("return");
                if let Some(e) = expr {
                    output.push(' ');
                    output.push_str(&self.generate_expression(e));
                }
                output.push_str(";\n");
                output
            }
            Statement::Expression(expr) => {
                let mut output = self.indent();
                output.push_str(&self.generate_expression(expr));
                output.push_str(";\n");
                output
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let mut output = self.indent();
                output.push_str("if ");
                output.push_str(&self.generate_expression(condition));
                output.push_str(" {\n");

                self.indent_level += 1;
                output.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_b) = else_block {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    output.push_str(&self.generate_block(else_b));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }

                output.push('\n');
                output
            }
            Statement::Match { value, arms } => {
                let mut output = self.indent();
                output.push_str("match ");
                output.push_str(&self.generate_expression(value));
                output.push_str(" {\n");

                self.indent_level += 1;
                for arm in arms {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_pattern(&arm.pattern));

                    // Add guard if present
                    if let Some(guard) = &arm.guard {
                        output.push_str(" if ");
                        output.push_str(&self.generate_expression(guard));
                    }

                    output.push_str(" => ");
                    output.push_str(&self.generate_expression(&arm.body));
                    output.push_str(",\n");
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Loop { body } => {
                let mut output = self.indent();
                output.push_str("loop {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::While { condition, body } => {
                let mut output = self.indent();
                output.push_str("while ");
                output.push_str(&self.generate_expression(condition));
                output.push_str(" {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::For {
                variable,
                iterable,
                body,
            } => {
                let mut output = self.indent();
                output.push_str("for ");
                output.push_str(variable);
                output.push_str(" in ");
                output.push_str(&self.generate_expression(iterable));
                output.push_str(" {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Break => {
                let mut output = self.indent();
                output.push_str("break;\n");
                output
            }
            Statement::Continue => {
                let mut output = self.indent();
                output.push_str("continue;\n");
                output
            }
            Statement::Assignment { target, value } => {
                let mut output = self.indent();

                // PHASE 5 OPTIMIZATION: Check if this can use a compound operator
                if let Expression::Identifier(var_name) = target {
                    if let Expression::Binary { left, right, op } = value {
                        if let Expression::Identifier(left_var) = &**left {
                            if left_var == var_name {
                                // Check if we have this optimization hint
                                if self.assignment_optimizations.contains_key(var_name) {
                                    // Generate compound assignment
                                    output.push_str(&self.generate_expression(target));
                                    output.push_str(match op {
                                        crate::parser::BinaryOp::Add => " += ",
                                        crate::parser::BinaryOp::Sub => " -= ",
                                        crate::parser::BinaryOp::Mul => " *= ",
                                        crate::parser::BinaryOp::Div => " /= ",
                                        _ => " = ",
                                    });
                                    output.push_str(&self.generate_expression(right));
                                    output.push_str(";\n");
                                    return output;
                                }
                            }
                        }
                    }
                }
                // If no optimization applied, fall through to regular assignment

                // Fall back to regular assignment
                output.push_str(&self.generate_expression(target));
                output.push_str(" = ");
                output.push_str(&self.generate_expression(value));
                output.push_str(";\n");
                output
            }
            Statement::Go { body } => {
                // Transpile to tokio::spawn or std::thread::spawn
                let mut output = self.indent();
                output.push_str("tokio::spawn(async move {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Defer(stmt) => {
                // Defer is not directly supported in Rust
                // We'll generate a comment for now
                let mut output = self.indent();
                output.push_str("// TODO: defer not yet implemented\n");
                output.push_str(&self.generate_statement(stmt));
                output
            }
        }
    }

    fn generate_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::EnumVariant(name, binding) => {
                if let Some(b) = binding {
                    format!("{}({})", name, b)
                } else {
                    name.clone()
                }
            }
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                pattern_strs.join(" | ")
            }
        }
    }

    fn generate_expression_with_precedence(&mut self, expr: &Expression) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. }
            | Expression::Binary { .. }
            | Expression::Closure { .. }
            | Expression::Ternary { .. } => {
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        }
    }

    // PHASE 7: Constant folding - evaluate constant expressions at compile time
    #[allow(clippy::only_used_in_recursion)]
    fn try_fold_constant(&self, expr: &Expression) -> Option<Expression> {
        match expr {
            Expression::Binary { left, op, right } => {
                // Try to fold both sides first
                let left_folded = self
                    .try_fold_constant(left)
                    .unwrap_or_else(|| (**left).clone());
                let right_folded = self
                    .try_fold_constant(right)
                    .unwrap_or_else(|| (**right).clone());

                // If both sides are literals, try to evaluate
                if let (Expression::Literal(l), Expression::Literal(r)) =
                    (&left_folded, &right_folded)
                {
                    use BinaryOp::*;
                    use Literal::*;

                    let result = match (l, op, r) {
                        // Integer arithmetic
                        (Int(a), Add, Int(b)) => Some(Literal::Int(a + b)),
                        (Int(a), Sub, Int(b)) => Some(Literal::Int(a - b)),
                        (Int(a), Mul, Int(b)) => Some(Literal::Int(a * b)),
                        (Int(a), Div, Int(b)) if *b != 0 => Some(Literal::Int(a / b)),
                        (Int(a), Mod, Int(b)) if *b != 0 => Some(Literal::Int(a % b)),

                        // Float arithmetic
                        (Float(a), Add, Float(b)) => Some(Literal::Float(a + b)),
                        (Float(a), Sub, Float(b)) => Some(Literal::Float(a - b)),
                        (Float(a), Mul, Float(b)) => Some(Literal::Float(a * b)),
                        (Float(a), Div, Float(b)) if *b != 0.0 => Some(Literal::Float(a / b)),

                        // Integer comparisons
                        (Int(a), Eq, Int(b)) => Some(Literal::Bool(a == b)),
                        (Int(a), Ne, Int(b)) => Some(Literal::Bool(a != b)),
                        (Int(a), Lt, Int(b)) => Some(Literal::Bool(a < b)),
                        (Int(a), Le, Int(b)) => Some(Literal::Bool(a <= b)),
                        (Int(a), Gt, Int(b)) => Some(Literal::Bool(a > b)),
                        (Int(a), Ge, Int(b)) => Some(Literal::Bool(a >= b)),

                        // Boolean operations
                        (Bool(a), And, Bool(b)) => Some(Literal::Bool(*a && *b)),
                        (Bool(a), Or, Bool(b)) => Some(Literal::Bool(*a || *b)),

                        _ => None,
                    };

                    return result.map(Expression::Literal);
                }
                None
            }
            Expression::Unary { op, operand } => {
                let operand_folded = self
                    .try_fold_constant(operand)
                    .unwrap_or_else(|| (**operand).clone());

                if let Expression::Literal(lit) = &operand_folded {
                    use Literal::*;
                    use UnaryOp::*;

                    let result = match (op, lit) {
                        (Neg, Int(n)) => Some(Literal::Int(-n)),
                        (Neg, Float(f)) => Some(Literal::Float(-f)),
                        (Not, Bool(b)) => Some(Literal::Bool(!b)),
                        _ => None,
                    };

                    return result.map(Expression::Literal);
                }
                None
            }
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                let cond_folded = self
                    .try_fold_constant(condition)
                    .unwrap_or_else(|| (**condition).clone());

                if let Expression::Literal(Literal::Bool(b)) = &cond_folded {
                    // If condition is constant, return the appropriate branch
                    if *b {
                        return self
                            .try_fold_constant(true_expr)
                            .or_else(|| Some((**true_expr).clone()));
                    } else {
                        return self
                            .try_fold_constant(false_expr)
                            .or_else(|| Some((**false_expr).clone()));
                    }
                }
                None
            }
            // Already a literal - can't fold further
            Expression::Literal(_) => None,
            // Can't fold non-constant expressions
            _ => None,
        }
    }

    fn generate_expression(&mut self, expr: &Expression) -> String {
        // PHASE 7: Try constant folding first
        let folded_expr = self.try_fold_constant(expr);
        let expr_to_generate = folded_expr.as_ref().unwrap_or(expr);

        match expr_to_generate {
            Expression::Literal(lit) => self.generate_literal(lit),
            Expression::Identifier(name) => {
                // Convert qualified paths: std.fs.read -> std::fs::read
                // But keep simple identifiers: variable_name -> variable_name
                if name.contains('.') {
                    name.replace('.', "::")
                } else {
                    // Check if this is a struct field and we're in an impl block
                    if self.in_impl_block && self.current_struct_fields.contains(name) {
                        format!("self.{}", name)
                    } else {
                        name.clone()
                    }
                }
            }
            Expression::Binary { left, op, right } => {
                // Wrap operands in parens if they have lower precedence
                let left_str = match left.as_ref() {
                    Expression::Binary { op: left_op, .. } => {
                        if self.op_precedence(left_op) < self.op_precedence(op) {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left),
                };
                let right_str = match right.as_ref() {
                    Expression::Binary { op: right_op, .. } => {
                        if self.op_precedence(right_op) < self.op_precedence(op) {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right),
                };
                let op_str = self.binary_op_to_rust(op);
                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                let cond_str = self.generate_expression(condition);
                let true_str = self.generate_expression(true_expr);
                let false_str = self.generate_expression(false_expr);
                format!(
                    "if {} {{ {} }} else {{ {} }}",
                    cond_str, true_str, false_str
                )
            }
            Expression::Unary { op, operand } => {
                let operand_str = self.generate_expression(operand);
                let op_str = self.unary_op_to_rust(op);
                format!("{}{}", op_str, operand_str)
            }
            Expression::Call {
                function,
                arguments,
            } => {
                // Extract function name for signature lookup
                let func_name = self.extract_function_name(function);

                // Special case: convert print() to println!()
                if func_name == "print" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("println!({})", args.join(", "));
                }

                let func_str = self.generate_expression(function);

                // Look up signature and clone it to avoid borrow conflicts
                let signature = self.signature_registry.get_signature(&func_name).cloned();

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        let arg_str = self.generate_expression(arg);

                        // Check if this parameter expects a borrow
                        if let Some(ref sig) = signature {
                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed => {
                                        // Insert & if not already a reference
                                        if !self.is_reference_expression(arg) {
                                            return format!("&{}", arg_str);
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => {
                                        // Insert &mut if not already a reference
                                        if !self.is_reference_expression(arg) {
                                            return format!("&mut {}", arg_str);
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // No change needed
                                    }
                                }
                            }
                        }

                        arg_str
                    })
                    .collect();
                format!("{}({})", func_str, args.join(", "))
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
            } => {
                let obj_str = self.generate_expression_with_precedence(object);
                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_label, arg)| self.generate_expression(arg))
                    .collect();

                // Generate turbofish if present
                let turbofish = if let Some(types) = type_args {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    format!("::<{}>", type_strs.join(", "))
                } else {
                    String::new()
                };

                // Special case: empty method name means turbofish on a function call (func::<T>())
                if method.is_empty() {
                    return format!("{}{}({})", obj_str, turbofish, args.join(", "));
                }

                // Determine separator: :: for static calls, . for instance methods
                // - Type/Module (starts with uppercase): use ::
                // - Variable (starts with lowercase): use .
                let separator = match **object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier(ref name) => {
                        // Check for known module/crate names that should use ::
                        let known_modules = [
                            "std",
                            "serde_json",
                            "serde",
                            "tokio",
                            "reqwest",
                            "sqlx",
                            "chrono",
                            "sha2",
                            "bcrypt",
                            "base64",
                            "rand",
                            "Vec",
                            "String",
                            "Option",
                            "Result",
                            "Box",
                            "Arc",
                            "Mutex",
                            "Utc",
                            "Local",
                            "DEFAULT_COST",
                        ];

                        // Type or module (uppercase) vs variable (lowercase)
                        if name.chars().next().is_some_and(|c| c.is_uppercase())
                            || name.contains('.')
                            || known_modules.contains(&name.as_str())
                        {
                            "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                        } else {
                            "." // x.abs(), value.method()
                        }
                    }
                    Expression::FieldAccess { .. } => "::", // Module path: std.fs.read() -> std::fs::read()
                    _ => ".",                               // Instance method on expressions
                };

                // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
                // If this is a .clone() on a variable that doesn't need cloning, skip it
                if method == "clone" && arguments.is_empty() {
                    if let Expression::Identifier(ref var_name) = **object {
                        if self.clone_optimizations.contains(var_name) {
                            // Skip the .clone(), just return the variable (or borrow if needed)
                            return obj_str;
                        }
                    }
                }

                format!(
                    "{}{}{}{}({})",
                    obj_str,
                    separator,
                    method,
                    turbofish,
                    args.join(", ")
                )
            }
            Expression::FieldAccess { object, field } => {
                let obj_str = self.generate_expression_with_precedence(object);

                // In module context (stdlib), always use :: for Rust paths
                // Otherwise, use :: for module/type paths and . for field access
                let separator = if self.is_module {
                    "::"
                } else {
                    match **object {
                        Expression::Identifier(ref name)
                            if name.contains('.')
                                || (!name.is_empty()
                                    && name.chars().next().unwrap().is_uppercase()) =>
                        {
                            "::"
                        }
                        Expression::FieldAccess { .. } => "::", // Chained path
                        _ => ".",                               // Actual field access
                    }
                };

                format!("{}{}{}", obj_str, separator, field)
            }
            Expression::StructLiteral { name, fields } => {
                // PHASE 3 OPTIMIZATION: Check if we have optimization hints for this struct
                let _has_optimization_hint = self.struct_mapping_hints.get(name);

                // Generate field assignments
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(field_name, expr)| {
                        // For simple direct field access (e.g., source.field -> target.field),
                        // we can generate cleaner code
                        let expr_str = self.generate_expression(expr);

                        // Check for field shorthand: if expr is just the field name, use shorthand
                        if let Expression::Identifier(id) = expr {
                            if id == field_name {
                                // Shorthand: User { name } instead of User { name: name }
                                return field_name.clone();
                            }
                        }

                        format!("{}: {}", field_name, expr_str)
                    })
                    .collect();

                format!("{} {{ {} }}", name, field_str.join(", "))
            }
            Expression::TryOp(inner) => {
                format!("{}?", self.generate_expression(inner))
            }
            Expression::Await(inner) => {
                format!("{}.await", self.generate_expression(inner))
            }
            Expression::ChannelSend { channel, value } => {
                let ch_str = self.generate_expression(channel);
                let val_str = self.generate_expression(value);
                format!("{}.send({})", ch_str, val_str)
            }
            Expression::ChannelRecv(channel) => {
                let ch_str = self.generate_expression(channel);
                format!("{}.recv()", ch_str)
            }
            Expression::Range {
                start,
                end,
                inclusive,
            } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("{}..={}", start_str, end_str)
                } else {
                    format!("{}..{}", start_str, end_str)
                }
            }
            Expression::Closure { parameters, body } => {
                let params = parameters.join(", ");
                let body_str = self.generate_expression(body);
                format!("|{}| {}", params, body_str)
            }
            Expression::Index { object, index } => {
                let obj_str = self.generate_expression(object);
                let idx_str = self.generate_expression(index);
                format!("{}[{}]", obj_str, idx_str)
            }
            Expression::Tuple(exprs) => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("({})", expr_strs.join(", "))
            }
            Expression::Array(exprs) => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("vec![{}]", expr_strs.join(", "))
            }
            Expression::MacroInvocation {
                name,
                args,
                delimiter,
            } => {
                use crate::parser::MacroDelimiter;

                // PHASE 4 OPTIMIZATION: Check for format! with capacity hints
                if name == "format" {
                    if let Some(&capacity) =
                        self.string_capacity_hints.get(&self.current_statement_idx)
                    {
                        // Clone capacity to avoid borrow issues
                        let capacity_val = capacity;
                        // Generate optimized String::with_capacity + write! instead of format!
                        self.needs_write_import = true;
                        let arg_strs: Vec<String> =
                            args.iter().map(|e| self.generate_expression(e)).collect();

                        return format!(
                            "{{\n{}    let mut __s = String::with_capacity({});\n{}    write!(&mut __s, {}).unwrap();\n{}    __s\n{}}}",
                            self.indent(),
                            capacity_val,
                            self.indent(),
                            arg_strs.join(", "),
                            self.indent(),
                            self.indent()
                        );
                    }
                }

                // Special case: if this is println!/eprintln!/print!/eprint! and first arg is format!, flatten it
                let should_flatten = (name == "println"
                    || name == "eprintln"
                    || name == "print"
                    || name == "eprint")
                    && !args.is_empty()
                    && matches!(&args[0], Expression::MacroInvocation { name: macro_name, .. } if macro_name == "format");

                let arg_strs: Vec<String> = if should_flatten {
                    // Flatten format! macro arguments into the print macro
                    if let Expression::MacroInvocation {
                        args: format_args, ..
                    } = &args[0]
                    {
                        format_args
                            .iter()
                            .map(|e| self.generate_expression(e))
                            .collect()
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                } else {
                    args.iter().map(|e| self.generate_expression(e)).collect()
                };

                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };
                format!("{}!{}{}{}", name, open, arg_strs.join(", "), close)
            }
            Expression::Cast { expr, type_ } => {
                // Add parentheses around binary expressions for correct precedence
                let expr_str = match &**expr {
                    Expression::Binary { .. } => {
                        format!("({})", self.generate_expression(expr))
                    }
                    _ => self.generate_expression(expr),
                };
                let type_str = self.type_to_rust(type_);
                format!("{} as {}", expr_str, type_str)
            }
            Expression::Block(stmts) => {
                // Special case: if the block contains only a match statement, generate it as a match expression
                if stmts.len() == 1 {
                    if let Statement::Match { value, arms } = &stmts[0] {
                        let mut output = String::from("match ");
                        output.push_str(&self.generate_expression(value));
                        output.push_str(" {\n");

                        self.indent_level += 1;
                        for arm in arms {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_pattern(&arm.pattern));

                            // Add guard if present
                            if let Some(guard) = &arm.guard {
                                output.push_str(" if ");
                                output.push_str(&self.generate_expression(guard));
                            }

                            output.push_str(" => ");
                            output.push_str(&self.generate_expression(&arm.body));
                            output.push_str(",\n");
                        }
                        self.indent_level -= 1;

                        output.push_str(&self.indent());
                        output.push('}');
                        return output;
                    }
                }

                // Regular block
                let mut output = String::from("{\n");
                self.indent_level += 1;
                for stmt in stmts {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push('}');
                output
            }
        }
    }

    fn generate_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
                let s = f.to_string();
                // Ensure float literals always have a decimal point
                if !s.contains('.') && !s.contains('e') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
            Literal::String(s) => {
                // Check for string interpolation: {variable}
                if s.contains('{') && s.contains('}') {
                    // Convert to format! macro
                    // "Count: {count}" -> format!("Count: {}", count)
                    let mut format_str = String::new();
                    let mut args = Vec::new();
                    let mut chars = s.chars().peekable();

                    while let Some(ch) = chars.next() {
                        if ch == '{' {
                            // Check if it's {variable} pattern
                            let mut var_name = String::new();
                            let mut is_variable = true;

                            while let Some(&next_ch) = chars.peek() {
                                if next_ch == '}' {
                                    chars.next(); // consume }
                                    break;
                                } else if next_ch.is_alphanumeric() || next_ch == '_' {
                                    var_name.push(next_ch);
                                    chars.next();
                                } else {
                                    // Not a simple variable pattern
                                    is_variable = false;
                                    break;
                                }
                            }

                            if is_variable && !var_name.is_empty() {
                                // It's a variable interpolation
                                format_str.push_str("{}");
                                args.push(var_name);
                            } else {
                                // Not a variable, keep the braces
                                format_str.push('{');
                                format_str.push_str(&var_name);
                            }
                        } else {
                            format_str.push(ch);
                        }
                    }

                    if args.is_empty() {
                        // No interpolation found, just a regular string
                        format!("\"{}\"", s)
                    } else {
                        // Generate format! call with implicit self for struct fields
                        let formatted_args = args
                            .iter()
                            .map(|a| {
                                // Check if this is a struct field and add self. prefix
                                if self.in_impl_block && self.current_struct_fields.contains(a) {
                                    format!(", self.{}", a)
                                } else {
                                    format!(", {}", a)
                                }
                            })
                            .collect::<String>();

                        format!("format!(\"{}\"{})", format_str, formatted_args)
                    }
                } else {
                    format!("\"{}\"", s)
                }
            }
            Literal::Char(c) => {
                // Escape special characters
                match c {
                    '\n' => "'\\n'".to_string(),
                    '\t' => "'\\t'".to_string(),
                    '\r' => "'\\r'".to_string(),
                    '\\' => "'\\\\'".to_string(),
                    '\'' => "'\\''".to_string(),
                    '\0' => "'\\0'".to_string(),
                    _ => format!("'{}'", c),
                }
            }
            Literal::Bool(b) => b.to_string(),
        }
    }

    fn binary_op_to_rust(&self, op: &BinaryOp) -> &str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }

    fn op_precedence(&self, op: &BinaryOp) -> i32 {
        match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::Ne => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 6,
        }
    }

    fn unary_op_to_rust(&self, op: &UnaryOp) -> &str {
        match op {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
            UnaryOp::Ref => "&",
            UnaryOp::Deref => "*",
        }
    }

    fn extract_function_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::FieldAccess { field, .. } => field.clone(),
            _ => String::new(), // Can't determine function name
        }
    }

    fn is_reference_expression(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Unary {
                op: UnaryOp::Ref,
                ..
            }
        )
    }

    fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        let mut traits = vec!["Debug".to_string(), "Clone".to_string()]; // Always safe to derive

        // Check if all fields are Copy
        if self.all_fields_are_copy(&struct_.fields) {
            traits.push("Copy".to_string());
        }

        // Check if all fields are PartialEq/Eq
        if self.all_fields_are_comparable(&struct_.fields) {
            traits.push("PartialEq".to_string());
            traits.push("Eq".to_string());

            // If Eq, also check for Hash
            if self.all_fields_are_hashable(&struct_.fields) {
                traits.push("Hash".to_string());
            }
        }

        // Check if all fields have Default
        if self.all_fields_have_default(&struct_.fields) {
            traits.push("Default".to_string());
        }

        traits
    }

    fn all_fields_are_copy(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_copy_type(&field.field_type))
    }

    fn all_fields_are_comparable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_comparable_type(&field.field_type))
    }

    fn all_fields_are_hashable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_hashable_type(&field.field_type))
    }

    fn all_fields_have_default(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.has_default(&field.field_type))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,         // References are Copy
            Type::MutableReference(_) => false, // Mutable references are not Copy
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            Type::Custom(name) => {
                // Recognize common Rust primitive types by name
                matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                )
            }
            _ => false, // String, Vec, Option, Result, other Custom types are not Copy
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_comparable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_comparable_type(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_comparable_type(t)),
            Type::Option(inner) => self.is_comparable_type(inner),
            Type::Result(ok, err) => self.is_comparable_type(ok) && self.is_comparable_type(err),
            _ => false, // Vec is not Eq (only PartialEq), Custom types unknown
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_hashable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false, // Floats are not Hash
            Type::Reference(inner) => self.is_hashable_type(inner),
            Type::MutableReference(_) => false,
            Type::Tuple(types) => types.iter().all(|t| self.is_hashable_type(t)),
            Type::Vec(_) => false, // Vec is not Hash
            Type::Option(inner) => self.is_hashable_type(inner),
            _ => false, // Result, Custom types - assume not Hash
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn has_default(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::String => true,    // String has Default ("")
            Type::Vec(_) => true,    // Vec has Default (empty vec)
            Type::Option(_) => true, // Option has Default (None)
            Type::Tuple(types) => types.iter().all(|t| self.has_default(t)),
            _ => false, // Refs don't have Default, Result/Custom types unknown
        }
    }

    /// OPTIMIZATION: Determine if a function should be marked #[inline]
    /// Phase 1: Generate Inlinable Code
    ///
    /// Heuristics for inlining:
    /// 1. Module functions (stdlib wrappers) - always inline for zero-cost abstraction
    /// 2. Small functions (< 10 statements) - likely to benefit from inlining
    /// 3. Trivial getters/setters - always inline
    /// 4. Functions with only one return statement - simple enough to inline
    /// 5. Don't inline: main(), test functions, async functions, large functions
    fn should_inline_function(&self, func: &FunctionDecl, _analyzed: &AnalyzedFunction) -> bool {
        // Never inline main
        if func.name == "main" {
            return false;
        }

        // Never inline test functions
        if func.decorators.iter().any(|d| d.name == "test") {
            return false;
        }

        // Don't inline async functions (they're already state machines)
        if func.decorators.iter().any(|d| d.name == "async") {
            return false;
        }

        // ALWAYS inline module functions (stdlib wrappers)
        // These are thin wrappers around Rust stdlib and should have zero overhead
        if self.is_module {
            return true;
        }

        // Count statements in function body
        let statement_count = self.count_statements(&func.body);

        // Inline small functions (< 10 statements)
        if statement_count < 10 {
            return true;
        }

        // Inline trivial single-expression functions
        if statement_count == 1 {
            if let Statement::Return(Some(_)) = &func.body[0] {
                return true;
            }
            if let Statement::Expression(_) = &func.body[0] {
                return true;
            }
        }

        // Default: don't inline large functions
        false
    }

    /// Count statements in a function body (for inline heuristics)
    fn count_statements(&self, body: &[Statement]) -> usize {
        let mut count = 0;
        for stmt in body {
            count += match stmt {
                Statement::Let { .. } => 1,
                Statement::Const { .. } => 1,
                Statement::Static { .. } => 1,
                Statement::Return(_) => 1,
                Statement::Expression(_) => 1,
                Statement::If { .. } => 3, // Weighted more heavily
                Statement::While { .. } => 3,
                Statement::Loop { .. } => 3,
                Statement::For { .. } => 3,
                Statement::Match { .. } => 5, // Match statements are complex
                Statement::Assignment { .. } => 1,
                Statement::Go { .. } => 2, // Goroutine spawn
                Statement::Defer(_) => 1,
                Statement::Break => 1,
                Statement::Continue => 1,
            };
        }
        count
    }

    // Format type parameters with trait bounds for Rust output
    // Example: [TypeParam { name: "T", bounds: ["Display", "Clone"] }] -> "T: Display + Clone"
    fn format_type_params(&self, type_params: &[crate::parser::TypeParam]) -> String {
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

    // Format where clause for Rust output
    // Example: [("T", ["Display"]), ("U", ["Debug", "Clone"])] -> "\nwhere\n    T: Display,\n    U: Debug + Clone"
    fn format_where_clause(&self, where_clause: &[(String, Vec<String>)]) -> String {
        if where_clause.is_empty() {
            return String::new();
        }

        let clauses: Vec<String> = where_clause
            .iter()
            .map(|(type_param, bounds)| format!("    {}: {}", type_param, bounds.join(" + ")))
            .collect();

        format!("\nwhere\n{}", clauses.join(",\n"))
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
    fn wrap_with_defer_drop(
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

    /// PHASE 7: Check if an expression can be evaluated at compile time
    /// If true, we can use `const` instead of `static`
    #[allow(clippy::only_used_in_recursion)]
    fn is_const_evaluable(&self, expr: &Expression) -> bool {
        match expr {
            // Literals are always const
            Expression::Literal(_) => true,

            // Binary operations on const values are const
            Expression::Binary { left, right, .. } => {
                self.is_const_evaluable(left) && self.is_const_evaluable(right)
            }

            // Unary operations on const values are const
            Expression::Unary { operand, .. } => self.is_const_evaluable(operand),

            // Struct literals with const fields might be const
            Expression::StructLiteral { fields, .. } => {
                fields.iter().all(|(_, expr)| self.is_const_evaluable(expr))
            }

            // Most other expressions are not const-evaluable
            _ => false,
        }
    }
}
