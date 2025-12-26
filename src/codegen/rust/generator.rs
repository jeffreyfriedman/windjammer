// Rust code generator
use crate::analyzer::*;
use crate::codegen::rust::{
    arm_string_analysis, ast_utilities, codegen_helpers, constant_folding, expression_helpers,
    operators, pattern_analysis, self_analysis, string_analysis, type_analysis,
};
use crate::parser::ast::CompoundOp;
use crate::parser::*;
use crate::CompilationTarget;

// DATA FLOW ANALYSIS: Track how a variable is used
#[derive(Debug, Clone, Copy, PartialEq)]
enum VariableUsage {
    NotUsed,         // Variable not referenced
    FieldAccessOnly, // Variable only used for field access (frame.x)
    Moved,           // Variable moved (returned, passed to function, used by itself)
}

pub struct CodeGenerator {
    indent_level: usize,
    signature_registry: SignatureRegistry,
    in_wasm_bindgen_impl: bool,
    in_trait_impl: bool, // true if currently generating code for a trait implementation
    needs_wasm_imports: bool,
    needs_web_imports: bool,
    needs_js_imports: bool,
    needs_serde_imports: bool,   // For JSON support
    needs_write_import: bool,    // For string capacity optimization (write! macro)
    needs_smallvec_import: bool, // For Phase 8 SmallVec optimization
    needs_cow_import: bool,      // For Phase 9 Cow optimization
    target: CompilationTarget,
    is_module: bool, // true if generating code for a reusable module (not main file)
    source_map: crate::source_map::SourceMap,
    current_output_file: std::path::PathBuf, // Path to the Rust file being generated
    current_rust_line: usize, // Current line number in generated Rust code (1-indexed)
    current_wj_file: std::path::PathBuf, // Path to the Windjammer file being compiled
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
    // AUTO-CLONE: Track where to automatically insert clones
    auto_clone_analysis: Option<crate::auto_clone::AutoCloneAnalysis>,
    // Track current statement index for optimization hints
    current_statement_idx: usize,
    // IMPLICIT SELF SUPPORT: Track struct fields for implicit self references
    current_struct_fields: std::collections::HashSet<String>, // Field names in current impl block
    current_struct_name: Option<String>, // Name of struct in current impl block
    in_impl_block: bool,                 // true if currently generating code for an impl block
    // USIZE DETECTION: Track which struct fields have type usize (for auto-casting)
    usize_struct_fields: std::collections::HashMap<String, std::collections::HashSet<String>>, // Struct name -> usize field names
    // FUNCTION CONTEXT: Track current function parameters for compound assignment optimization
    current_function_params: Vec<crate::parser::Parameter>, // Parameters of the current function
    // FUNCTION CONTEXT: Track current function return type for string literal conversion
    current_function_return_type: Option<Type>,
    // WINDJAMMER TRAIT INFERENCE: Analyzed trait methods with inferred signatures from ALL impls
    analyzed_trait_methods: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::analyzer::AnalyzedFunction>,
    >,
    // FUNCTION CONTEXT: Track current function body for data flow analysis
    current_function_body: Vec<Statement>, // Body of the current function being generated
    // Workspace root for source maps
    workspace_root: Option<std::path::PathBuf>,
    // BRANCH TYPE CONSISTENCY: Suppress auto string conversion when any branch uses .as_str()
    suppress_string_conversion: bool,
    // LOCAL VARIABLE TRACKING: Stack of scopes, each scope contains local variable names
    // Enables proper variable shadowing of field names
    local_variable_scopes: Vec<std::collections::HashSet<String>>,
    // EXPRESSION CONTEXT: Track if we're generating code whose value will be used
    // Prevents adding semicolons to final expressions in if-else/match when used as values
    in_expression_context: bool,
    // TRAIT TRACKING: Track which custom types support PartialEq
    partial_eq_types: std::collections::HashSet<String>,
    // MATCH ARM CONTEXT: Force string conversion in match arm blocks
    in_match_arm_needing_string: bool,
    // BORROWED ITERATOR VARIABLES: Track variables that are iterating over borrowed collections
    // These variables are references, so accessing their fields requires .clone()
    borrowed_iterator_vars: std::collections::HashSet<String>,
    // USIZE VARIABLES: Track variables assigned from .len() for auto-casting
    usize_variables: std::collections::HashSet<String>,
    // INFERRED BORROWED PARAMS: Parameters inferred to be borrowed (for field access cloning)
    inferred_borrowed_params: std::collections::HashSet<String>,
    // ASSIGNMENT TARGET: Flag to suppress auto-clone when generating assignment targets
    generating_assignment_target: bool,
}

impl CodeGenerator {
    pub fn new(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        CodeGenerator {
            indent_level: 0,
            signature_registry: registry,
            in_wasm_bindgen_impl: false,
            in_trait_impl: false,
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
            current_output_file: std::path::PathBuf::new(),
            current_rust_line: 1,
            current_wj_file: std::path::PathBuf::new(),
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
            auto_clone_analysis: None,
            current_statement_idx: 0,
            current_struct_fields: std::collections::HashSet::new(),
            current_struct_name: None,
            in_impl_block: false,
            usize_struct_fields: std::collections::HashMap::new(),
            current_function_params: Vec::new(),
            current_function_return_type: None,
            current_function_body: Vec::new(),
            workspace_root: None,
            suppress_string_conversion: false,
            borrowed_iterator_vars: std::collections::HashSet::new(),
            usize_variables: std::collections::HashSet::new(),
            inferred_borrowed_params: std::collections::HashSet::new(),
            generating_assignment_target: false,
            partial_eq_types: std::collections::HashSet::new(),
            in_match_arm_needing_string: false,
            local_variable_scopes: Vec::new(),
            in_expression_context: false,
            analyzed_trait_methods: std::collections::HashMap::new(),
        }
    }

    /// Set analyzed trait methods (used for trait signature inference from impls)
    pub fn set_analyzed_trait_methods(
        &mut self,
        methods: std::collections::HashMap<
            String,
            std::collections::HashMap<String, crate::analyzer::AnalyzedFunction>,
        >,
    ) {
        self.analyzed_trait_methods = methods;
    }

    /// Set the workspace root for relative paths in source maps
    pub fn set_workspace_root(&mut self, path: std::path::PathBuf) {
        self.workspace_root = Some(path.clone());
        // CRITICAL: Also set workspace root on the source_map for relative path conversion
        self.source_map.set_workspace_root(path);
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

    /// Generate an item inside an inline module
    fn generate_inline_module_item(
        &mut self,
        item: &Item,
        analyzed: &[AnalyzedFunction],
    ) -> String {
        match item {
            Item::Function { decl, .. } => {
                // Find the analyzed version
                if let Some(analyzed_func) = analyzed.iter().find(|f| f.decl.name == decl.name) {
                    self.generate_function(analyzed_func)
                } else {
                    // Shouldn't happen, but generate basic signature
                    String::new()
                }
            }
            Item::Struct { decl, .. } => self.generate_struct(decl),
            Item::Enum { decl, .. } => self.generate_enum(decl),
            Item::Trait { decl, .. } => self.generate_trait_with_analysis(decl, analyzed),
            Item::Impl { block, .. } => self.generate_impl(block, analyzed),
            Item::Mod {
                name,
                items,
                is_public,
                ..
            } => {
                // Nested inline module
                let mut output = String::new();
                if *is_public {
                    output.push_str(&format!("pub mod {} {{\n", name));
                } else {
                    output.push_str(&format!("mod {} {{\n", name));
                }

                self.indent_level += 1;
                for nested_item in items {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_inline_module_item(nested_item, analyzed));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            _ => String::new(), // Ignore other items for now
        }
    }

    // ============================================================================
    // SOURCE MAP TRACKING
    // ============================================================================

    /// Set the output file path for source mapping
    pub fn set_output_file(&mut self, path: impl Into<std::path::PathBuf>) {
        self.current_output_file = path.into();
    }

    /// Set the Windjammer source file path for source mapping
    pub fn set_source_file(&mut self, path: impl Into<std::path::PathBuf>) {
        self.current_wj_file = path.into();
    }

    /// Get the current line number in the generated Rust code
    #[allow(dead_code)]
    fn current_rust_line(&self) -> usize {
        self.current_rust_line
    }

    /// Increment the Rust line counter (call after generating each line)
    #[allow(dead_code)]
    fn increment_rust_line(&mut self) {
        self.current_rust_line += 1;
    }

    /// Increment the Rust line counter by N lines
    #[allow(dead_code)]
    fn increment_rust_lines(&mut self, count: usize) {
        self.current_rust_line += count;
    }

    /// Record a mapping from current Rust location to Windjammer location
    fn record_mapping(&mut self, wj_location: &crate::source_map::Location) {
        if !self.current_output_file.as_os_str().is_empty() {
            self.source_map.add_mapping(
                self.current_output_file.clone(),
                self.current_rust_line,
                0, // column (simplified for now)
                wj_location.file.clone(),
                wj_location.line,
                wj_location.column,
            );
        }
    }

    /// Get the source map (for saving after code generation)
    pub fn get_source_map(&self) -> &crate::source_map::SourceMap {
        &self.source_map
    }

    /// Count newlines in a string and increment the Rust line counter
    #[allow(dead_code)]
    fn track_generated_lines(&mut self, code: &str) {
        let newline_count = code.matches('\n').count();
        if newline_count > 0 {
            self.increment_rust_lines(newline_count);
        }
    }

    /// Generate a statement with automatic source tracking
    #[allow(dead_code)]
    fn generate_statement_tracked(&mut self, stmt: &Statement) -> String {
        let code = self.generate_statement(stmt);
        self.track_generated_lines(&code);
        code
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
            // HTTP method decorators for Axum
            ("get", _) => "axum::routing::get".to_string(),
            ("post", _) => "axum::routing::post".to_string(),
            ("put", _) => "axum::routing::put".to_string(),
            ("delete", _) => "axum::routing::delete".to_string(),
            ("patch", _) => "axum::routing::patch".to_string(),
            // Pass through other decorators as-is
            (other, _) => other.to_string(),
        }
    }

    // ============================================================================
    // UI FRAMEWORK SUPPORT
    // ============================================================================

    /// Check if an expression is a UI component that needs .to_vnode()
    #[allow(dead_code, clippy::only_used_in_recursion)]
    /// Check if a method is a builder method that returns Self (for chaining)
    #[allow(dead_code)]
    fn generate_block(&mut self, stmts: &[Statement]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            // Track current statement index for optimization hints
            self.current_statement_idx = i;

            let is_last = i == len - 1;
            if is_last
                && matches!(
                    stmt,
                    Statement::Expression { .. }
                        | Statement::Thread { .. }
                        | Statement::Async { .. }
                )
            {
                // Last statement is an expression or thread/async block - generate without discard (it's the return value)
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        let mut expr_str = self.generate_expression(expr);

                        // WINDJAMMER PHILOSOPHY: Auto-convert implicit returns when function returns String
                        // BUT: Don't convert if:
                        // 1. The expression explicitly uses .as_str() (user wants &str)
                        // 2. A sibling branch in an if-else uses .as_str() (type consistency)
                        let returns_string = match &self.current_function_return_type {
                            Some(Type::String) => true,
                            Some(Type::Custom(name)) if name == "String" => true,
                            _ => false,
                        };

                        // Also check if we're in a match arm that needs string conversion
                        let in_match_needing_string = self.in_match_arm_needing_string;

                        // Check if the expression explicitly returns &str via .as_str()
                        let expr_uses_as_str = expr_str.contains(".as_str()");

                        // Check if we should suppress conversion (sibling branch has .as_str())
                        let should_suppress = self.suppress_string_conversion;

                        if (returns_string || in_match_needing_string)
                            && !expr_uses_as_str
                            && !should_suppress
                        {
                            // String literal needs .to_string()
                            if matches!(
                                expr,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) && !expr_str.ends_with(".to_string()")
                            {
                                expr_str = format!("{}.to_string()", expr_str);
                            }
                            // self.field needs .clone() when self is borrowed
                            else if let Expression::FieldAccess { object, .. } = expr {
                                if let Expression::Identifier { name: obj_name, .. } = &**object {
                                    if obj_name == "self" && !expr_str.ends_with(".clone()") {
                                        let self_is_borrowed =
                                            self.current_function_params.iter().any(|p| {
                                                p.name == "self"
                                                    && matches!(
                                                        p.ownership,
                                                        crate::parser::OwnershipHint::Ref
                                                    )
                                            });
                                        if self_is_borrowed {
                                            expr_str = format!("{}.clone()", expr_str);
                                        }
                                    }
                                }
                            }
                        }

                        // FIXED: Auto-cast usize to i64 for implicit returns
                        let returns_int = match &self.current_function_return_type {
                            Some(Type::Int) => true,
                            Some(Type::Custom(name)) if name == "i64" || name == "int" => true,
                            _ => false,
                        };

                        if returns_int && self.expression_produces_usize(expr) {
                            // Implicit return of .len() - auto-cast!
                            expr_str = format!("{} as i64", expr_str);
                        }

                        // WINDJAMMER PHILOSOPHY: Auto-add .cloned() for HashMap.get() and similar methods
                        // When returning Option<T> but method returns Option<&T>, add .cloned()
                        let returns_option_owned = self.returns_option_owned_type();
                        if returns_option_owned
                            && self.is_method_returning_option_ref(expr)
                            && !expr_str.ends_with(".cloned()")
                            && !expr_str.ends_with(".clone()")
                        {
                            expr_str = format!("{}.cloned()", expr_str);
                        }

                        output.push_str(&expr_str);

                        // BUGFIX: Only add semicolon if:
                        // 1. Function returns ()
                        // 2. AND we're not in an expression context (value is not being used)
                        // This prevents adding semicolons to if-else branches when used as values
                        let returns_unit = self.current_function_return_type.is_none()
                            || matches!(self.current_function_return_type, Some(Type::Tuple(ref types)) if types.is_empty());
                        if returns_unit && !self.in_expression_context {
                            output.push(';');
                        }
                        output.push('\n');
                    }
                    Statement::Thread { body, .. } => {
                        // Generate as expression (returns JoinHandle)
                        output.push_str(&self.indent());
                        output.push_str("std::thread::spawn(move || {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    Statement::Async { body, .. } => {
                        // Generate as expression (returns JoinHandle)
                        output.push_str(&self.indent());
                        output.push_str("tokio::spawn(async move {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    _ => unreachable!(),
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

        // PRE-PASS: Collect which custom types support PartialEq
        // This enables smart enum derive that only adds PartialEq if all variants support it
        self.collect_partial_eq_types(program);

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

        // Check for stdlib modules that need special imports
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                // Path is ["std", "json"] for "use std::json"
                let path_str = path.join("::");
                if (path_str.starts_with("std::") || path_str == "std") && path_str.contains("json")
                {
                    self.needs_serde_imports = true;
                }
                // http, time, crypto modules don't need special imports (used directly)
            }
        }

        // Generate explicit use statements
        for item in &program.items {
            if let Item::Use {
                path,
                alias,
                is_pub,
                ..
            } = item
            {
                let use_stmt = self.generate_use(path, alias.as_deref());
                if !use_stmt.trim().is_empty() {
                    if *is_pub {
                        imports.push_str("pub ");
                    }
                    imports.push_str(&use_stmt);
                }
                // Don't add extra newline - generate_use already includes it
            }
        }

        // Generate const and static declarations
        for item in &program.items {
            match item {
                Item::Const {
                    name, type_, value, ..
                } => {
                    let pub_prefix = if self.is_module { "pub " } else { "" };

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
                    // Set the struct name and fields for implicit self support
                    self.current_struct_name = Some(impl_block.type_name.clone());
                    if let Some(fields) = struct_fields.get(&impl_block.type_name) {
                        self.current_struct_fields = fields.iter().cloned().collect();
                    } else {
                        self.current_struct_fields.clear();
                    }
                    self.in_impl_block = true;

                    body.push_str(&self.generate_impl(impl_block, analyzed));
                    body.push_str("\n\n");

                    self.in_impl_block = false;
                    self.current_struct_name = None;
                    self.current_struct_fields.clear();
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

    fn generate_use(&self, path: &[String], alias: Option<&str>) -> String {
        if path.is_empty() {
            return String::new();
        }

        let full_path = path.join(".");

        // Handle stdlib imports FIRST (before glob handling)
        // This ensures std::ui::*, std::fs::*, etc. are properly skipped
        if full_path.starts_with("std::") || full_path.starts_with("std.") {
            // Normalize to use :: separator
            let normalized = full_path.replace('.', "::");
            let module_name = normalized.strip_prefix("std::").unwrap();

            // Strip glob suffix if present for checking
            let module_base = module_name.strip_suffix("::*").unwrap_or(module_name);

            // Handle Rust stdlib modules that should NOT be mapped to windjammer_runtime
            // These are native Rust modules that should be used directly
            if module_base.starts_with("collections")
                || module_base.starts_with("cmp")
                || module_base.starts_with("ops")
                || module_base == "ops"
            {
                // Pass through to Rust's std library
                return format!("use std::{};\n", module_name);
            }

            // Handle UI framework - skip explicit import (handled by implicit imports)
            if module_base == "ui" || module_base.starts_with("ui::") {
                // UI framework is handled by implicit imports from windjammer-ui crate
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle Game framework - skip explicit import (handled by implicit imports)
            if module_base == "game" || module_base.starts_with("game::") {
                // Game framework is handled by implicit imports from windjammer-game-framework crate
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle Tauri framework - skip explicit import (functions are generated inline)
            if module_base == "tauri" || module_base.starts_with("tauri::") {
                // Tauri functions are handled by compiler codegen (generate_tauri_invoke)
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle platform APIs - skip explicit import (handled by implicit imports)
            if module_base == "fs"
                || module_base.starts_with("fs::")
                || module_base == "process"
                || module_base.starts_with("process::")
                || module_base == "dialog"
                || module_base.starts_with("dialog::")
                || module_base == "env"
                || module_base.starts_with("env::")
                || module_base == "encoding"
                || module_base.starts_with("encoding::")
                || module_base == "compute"
                || module_base.starts_with("compute::")
                || module_base == "net"
                || module_base.starts_with("net::")
                || module_base == "http"
                || module_base.starts_with("http::")
                || module_base == "storage"
                || module_base.starts_with("storage::")
            {
                // Platform APIs are handled by implicit imports (platform-specific)
                // Don't generate an explicit import here
                return String::new();
            }

            // Map to windjammer_runtime (all stdlib modules are now implemented!)
            let rust_import = match module_name {
                // Core modules
                "http" => "windjammer_runtime::http",
                "mime" => "windjammer_runtime::mime",
                "json" => "windjammer_runtime::json",

                // Additional modules
                "async" => "windjammer_runtime::async_runtime",
                "cli" => "windjammer_runtime::cli",
                "crypto" => "windjammer_runtime::crypto",
                "csv" => "windjammer_runtime::csv_mod",
                "db" => "windjammer_runtime::db",
                "log" => "windjammer_runtime::log_mod",
                "math" => "windjammer_runtime::math",
                "random" => "windjammer_runtime::random",
                "regex" => "windjammer_runtime::regex_mod",
                "strings" => "windjammer_runtime::strings",
                "testing" => "windjammer_runtime::testing",
                "time" => "windjammer_runtime::time",
                // "ui" is handled by implicit imports (windjammer-ui crate), not runtime
                "game" => "windjammer_runtime::game",

                _ => {
                    // Unknown module - try windjammer_runtime
                    return format!("use windjammer_runtime::{};\n", module_name);
                }
            };

            if let Some(alias_name) = alias {
                return format!("use {} as {};\n", rust_import, alias_name);
            } else {
                // For _mod suffixed modules (log_mod, regex_mod), alias back to the original name
                // AND import any public types they export
                if rust_import.ends_with("_mod") {
                    let original_name = rust_import
                        .strip_suffix("_mod")
                        .and_then(|s| s.split("::").last())
                        .unwrap_or(rust_import);

                    let mut result = format!("use {} as {};\n", rust_import, original_name);

                    // Import types for modules that export them
                    match original_name {
                        "regex" => {
                            result.push_str(&format!("use {}::Regex;\n", rust_import));
                        }
                        "time" => {
                            result.push_str(&format!(
                                "use {}::{{Duration, Instant}};\n",
                                rust_import
                            ));
                        }
                        _ => {}
                    }

                    return result;
                }
                // Import the module itself (not glob) to keep module-qualified paths
                // For types like Duration, we'll need explicit imports or full paths
                return format!("use {};\n", rust_import);
            }
        }

        // Skip bare "std" imports
        if full_path == "std" {
            return String::new();
        }

        // Handle glob imports for non-stdlib modules: module.submodule.* -> use module::submodule::*;
        if full_path.ends_with(".*") {
            let path_without_glob = full_path.strip_suffix(".*").unwrap();
            // Replace dots with :: but remove any trailing ::
            let rust_path = path_without_glob
                .replace('.', "::")
                .trim_end_matches("::")
                .to_string();
            return format!("use {}::*;\n", rust_path);
        }

        // Handle braced imports: module::{A, B, C} or module.{A, B, C}
        if (full_path.contains("::{") || full_path.contains(".{")) && full_path.contains('}') {
            // Try :: separator first, then . separator
            if let Some((base, items)) = full_path.split_once("::{") {
                return format!("use {}::{{{};\n", base, items);
            } else if let Some((base, items)) = full_path.split_once(".{") {
                let rust_base = base.replace('.', "::");
                return format!("use {}::{{{};\n", rust_base, items);
            }
        }

        // Handle relative imports: ./utils or ../utils or ./config::Config
        if full_path.starts_with("./") || full_path.starts_with("../") {
            // Strip the leading ./ or ../
            let stripped = full_path
                .strip_prefix("./")
                .or_else(|| full_path.strip_prefix("../"))
                .unwrap_or(&full_path);

            // Check if this is importing a specific item (e.g., ./config::Config)
            if stripped.contains("::") {
                // Split into module path and item
                let rust_path = stripped.replace('/', "::");
                // Check if the last segment looks like a type (uppercase)
                let segments: Vec<&str> = rust_path.split("::").collect();
                if let Some(last) = segments.last() {
                    if last.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // Importing a specific type: ./config::Config -> use crate::config::Config;
                        return format!("use crate::{};\n", rust_path);
                    }
                }
                // For crate::module imports, just import the module (not ::*)
                // This allows qualified usage like module::func() in the code
                return format!("use crate::{};\n", rust_path);
            } else {
                // Module import: ./config
                // In the main entry point (is_module=false), modules are already in scope via pub mod declarations
                // In submodules (is_module=true), we need to explicitly use sibling modules
                let module_name = stripped.split('/').next_back().unwrap_or(stripped);
                if let Some(alias_name) = alias {
                    return format!("use crate::{} as {};\n", module_name, alias_name);
                } else if self.is_module {
                    // In a module, we need to explicitly use sibling modules
                    return format!("use crate::{};\n", module_name);
                } else {
                    // In main entry point, modules are already in scope
                    return String::new();
                }
            }
        }

        // Convert Windjammer's Go-style imports to Rust imports
        // Heuristic: If the last segment starts with an uppercase letter, it's likely a type/struct
        // Otherwise, it's a module and we should add ::*
        let rust_path = full_path.replace('.', "::");

        // BUGFIX: Handle imports from sibling modules (flat directory structure)
        // When importing from common module names like math, rendering, collision2d, etc.,
        // these are sibling files in src/generated/, so use super:: instead of absolute paths
        //
        // IMPORTANT: Distinguish between:
        // 1. Directory prefixes (math, rendering, physics) - should be stripped
        // 2. Actual module files (texture_atlas, sprite_region) - should be preserved
        // THE WINDJAMMER WAY: With nested module system (lib.rs), use crate:: for cross-directory imports
        // Only use super:: for same-directory imports
        let directory_prefixes = [
            "math",
            "rendering",
            "physics",
            "ecs",
            "audio",
            "effects",
            "input",
            "game_loop",
            "world",
            "debug",
            "assets",
        ];
        let common_sibling_modules = [
            "vec2",
            "vec3",
            "vec4",
            "mat4",
            "quat",
            "collision2d",
            "rigidbody2d",
            "physics_world",
            "entity",
            "components",
            "query",
            "world",
            "ecs",
            "texture",
            "texture_atlas",
            "sprite",
            "sprite_region",
            "camera2d",
            "camera3d",
            "color",
            "render_context",
            "render_api",
        ];

        // Handle super::super::math::vec3::Vec3 -> super::Vec3
        // This handles cases where Windjammer source uses "use super.super.math.vec3::Vec3"
        if rust_path.starts_with("super::super::") {
            // Extract the path after super::super::
            if let Some(rest_path) = rust_path.strip_prefix("super::super::") {
                // Find the actual type name (last segment)
                let segments: Vec<&str> = rest_path.split("::").collect();
                if let Some(type_name) = segments.last() {
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // It's a type, use just super::TypeName
                        return format!("use super::{};\n", type_name);
                    }
                }
            }
        }

        let first_segment = rust_path.split("::").next().unwrap_or("");
        let is_directory_prefix = directory_prefixes.contains(&first_segment);
        let is_actual_module_file =
            common_sibling_modules.contains(&first_segment) && !is_directory_prefix;
        let _is_sibling_module =
            is_directory_prefix || is_actual_module_file || first_segment == "super";

        if let Some(alias_name) = alias {
            if is_directory_prefix {
                // THE WINDJAMMER WAY: Use crate:: for cross-directory imports
                // math::Vec2 as V -> use crate::math::Vec2 as V;
                format!("use crate::{} as {};\n", rust_path, alias_name)
            } else if is_actual_module_file {
                // Keep module path for actual module files: texture_atlas::TextureAtlas as TA -> use super::texture_atlas::TextureAtlas as TA;
                format!("use super::{} as {};\n", rust_path, alias_name)
            } else {
                format!("use {} as {};\n", rust_path, alias_name)
            }
        } else {
            // Check if already a glob import (ends with ::*)
            if rust_path.ends_with("::*") {
                format!("use {};\n", rust_path)
            } else if is_directory_prefix {
                // THE WINDJAMMER WAY: Use crate:: for cross-directory imports
                // math::Vec2 -> use crate::math::Vec2;
                format!("use crate::{};\n", rust_path)
            } else if is_actual_module_file {
                // Keep full path for actual module files to avoid ambiguity
                // texture_atlas::TextureAtlas -> use super::texture_atlas::TextureAtlas;
                format!("use super::{};\n", rust_path)
            } else {
                // Check if the last segment looks like a type (starts with uppercase)
                let last_segment = rust_path.split("::").last().unwrap_or("");
                if last_segment
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase())
                {
                    // Likely a type, don't add ::*
                    format!("use {};\n", rust_path)
                } else if rust_path.starts_with("crate::") {
                    // For crate::module imports, don't add ::*
                    // This allows qualified usage like module::func() in the code
                    format!("use {};\n", rust_path)
                } else {
                    // Likely a module, add ::*
                    format!("use {}::*;\n", rust_path)
                }
            }
        }
    }

    fn generate_struct(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Track which fields have type usize (for auto-casting in comparisons)
        let mut usize_fields = std::collections::HashSet::new();
        for field in &s.fields {
            if matches!(field.field_type, Type::Custom(ref name) if name == "usize") {
                usize_fields.insert(field.name.clone());
            }
        }
        self.usize_struct_fields
            .insert(s.name.clone(), usize_fields);

        // Convert decorators to Rust attributes
        for decorator in &s.decorators {
            // Skip framework decorators - they're handled separately
            if decorator.name == "component" || decorator.name == "game" {
                continue;
            }

            if decorator.name == "command" {
                // Special handling for @command decorator - generates clap attributes
                // @command(name: "app", about: "Description") -> #[derive(Parser)] + #[command(...)]
                output.push_str("#[derive(Parser)]\n");

                if !decorator.arguments.is_empty() {
                    output.push_str("#[command(");
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
                continue;
            } else if decorator.name == "auto" {
                // Special handling for @auto decorator
                let traits = if decorator.arguments.is_empty() {
                    // Smart inference: no arguments, so infer traits based on field types
                    self.infer_derivable_traits(s)
                } else {
                    // Explicit: extract trait names from decorator arguments
                    let mut explicit_traits = Vec::new();
                    for (_key, expr) in &decorator.arguments {
                        if let Expression::Identifier {
                            name: trait_name, ..
                        } = expr
                        {
                            explicit_traits.push(trait_name.clone());
                        }
                    }
                    explicit_traits
                };

                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));

                    // Track if this struct has PartialEq for enum derive inference
                    if traits.iter().any(|t| t == "PartialEq") {
                        // Note: partial_eq_types is already populated in pre-pass, no need to insert here
                    }
                }
            } else if decorator.name == "derive" {
                // Special handling for @derive decorator - generates #[derive(Trait1, Trait2)]
                let mut traits = Vec::new();
                for (_key, expr) in &decorator.arguments {
                    if let Expression::Identifier {
                        name: trait_name, ..
                    } = expr
                    {
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

        // WINDJAMMER PHILOSOPHY: Auto-derive common traits for simple structs
        // If a struct has no @auto or @derive decorator, but all fields are primitive/Copy types,
        // automatically add Clone, Copy, Debug, PartialEq - this is what the user would want 90% of the time
        let has_derive_decorator = s
            .decorators
            .iter()
            .any(|d| d.name == "auto" || d.name == "derive");
        if !has_derive_decorator {
            let inferred_traits = self.infer_derivable_traits(s);
            if !inferred_traits.is_empty() {
                output.push_str(&format!("#[derive({})]\n", inferred_traits.join(", ")));
            }
        }

        // Add struct declaration with type parameters
        let pub_prefix = if s.is_pub || self.is_module {
            "pub "
        } else {
            ""
        };
        output.push_str(&format!("{}struct ", pub_prefix));
        output.push_str(&s.name);
        if !s.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&s.type_params));
            output.push('>');
        }

        // Add where clause if present
        output.push_str(&codegen_helpers::format_where_clause(&s.where_clause));

        // Check if this is a unit struct (no fields)
        if s.fields.is_empty() {
            // Unit struct - end with semicolon
            output.push(';');
            return output;
        }

        output.push_str(" {\n");

        for field in &s.fields {
            // Emit doc comment for field if present
            if let Some(doc) = &field.doc_comment {
                output.push_str(&format!("    /// {}\n", doc));
            }

            // Generate decorators for the field (convert to Rust attributes)
            for decorator in &field.decorators {
                // Handle @arg decorator specially - it's a clap field attribute
                if decorator.name == "arg" {
                    output.push_str("    #[arg(");
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            // Handle special cases for clap arguments
                            match key.as_str() {
                                "short" => {
                                    // short takes a character literal
                                    format!("short = {}", self.generate_expression_immut(expr))
                                }
                                "long" => {
                                    // long takes a string literal
                                    format!("long = {}", self.generate_expression_immut(expr))
                                }
                                "default_value" => {
                                    format!(
                                        "default_value = {}",
                                        self.generate_expression_immut(expr)
                                    )
                                }
                                "help" => {
                                    format!("help = {}", self.generate_expression_immut(expr))
                                }
                                _ => format!("{} = {}", key, self.generate_expression_immut(expr)),
                            }
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                } else {
                    // Generic decorator handling
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
            }
            // In modules, all fields should be pub for cross-module access
            let pub_keyword = if self.is_module || field.is_pub {
                "pub "
            } else {
                ""
            };
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
        let mut output = String::new();

        // WINDJAMMER PHILOSOPHY: Auto-derive common traits for enums
        // All enums get Clone, Debug by default
        // Only add PartialEq if ALL variants support it
        // Unit-only enums (no data) also get Copy
        let mut traits = vec!["Clone".to_string(), "Debug".to_string()];

        // Check if all variants support PartialEq
        let all_variants_partial_eq = self.all_enum_variants_are_partial_eq(&e.variants);
        if all_variants_partial_eq {
            traits.push("PartialEq".to_string());
        }

        let all_unit = e
            .variants
            .iter()
            .all(|v| matches!(v.data, crate::parser::EnumVariantData::Unit));
        if all_unit {
            traits.push("Copy".to_string());
        }
        output.push_str(&format!("#[derive({})]\n", traits.join(", ")));

        let pub_prefix = if e.is_pub || self.is_module {
            "pub "
        } else {
            ""
        };
        output.push_str(&format!("{}enum {}", pub_prefix, e.name));

        // Generate generic parameters: enum Option<T>, enum Result<T, E>
        if !e.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&e.type_params));
            output.push('>');
        }

        output.push_str(" {\n");

        for variant in &e.variants {
            // Emit doc comment for variant if present
            if let Some(doc) = &variant.doc_comment {
                output.push_str(&format!("    /// {}\n", doc));
            }

            use crate::parser::EnumVariantData;
            match &variant.data {
                EnumVariantData::Unit => {
                    output.push_str(&format!("    {},\n", variant.name));
                }
                EnumVariantData::Tuple(types) => {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    output.push_str(&format!(
                        "    {}({}),\n",
                        variant.name,
                        type_strs.join(", ")
                    ));
                }
                EnumVariantData::Struct(fields) => {
                    let field_strs: Vec<String> = fields
                        .iter()
                        .map(|(name, ty)| format!("{}: {}", name, self.type_to_rust(ty)))
                        .collect();
                    output.push_str(&format!(
                        "    {} {{ {} }},\n",
                        variant.name,
                        field_strs.join(", ")
                    ));
                }
            }
        }

        output.push('}');
        output
    }

    fn generate_trait_with_analysis(
        &mut self,
        trait_decl: &crate::parser::TraitDecl,
        analyzed: &[AnalyzedFunction],
    ) -> String {
        let mut output = String::new();

        // TODO: Add is_pub field to TraitDecl and check it properly
        // For now, always emit pub for traits (the common case)
        output.push_str("pub trait ");
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
            // THE WINDJAMMER WAY: Look up analyzed data for this method
            // Priority: 1) Global cross-file inferred (analyzed_trait_methods)
            //           2) Local analyzed (for default implementations)
            eprintln!(
                "DEBUG CODEGEN TRAIT: Looking for {}.{}",
                trait_decl.name, method.name
            );
            eprintln!(
                "DEBUG CODEGEN TRAIT:   analyzed_trait_methods has trait? {}",
                self.analyzed_trait_methods.contains_key(&trait_decl.name)
            );
            let analyzed_method =
                if let Some(trait_methods) = self.analyzed_trait_methods.get(&trait_decl.name) {
                    eprintln!(
                        "DEBUG CODEGEN TRAIT:   Trait methods has method? {}",
                        trait_methods.contains_key(&method.name)
                    );
                    if let Some(global_analysis) = trait_methods.get(&method.name) {
                        eprintln!(
                            "DEBUG CODEGEN TRAIT:   Using GLOBAL analysis with self={:?}",
                            global_analysis.inferred_ownership.get("self")
                        );
                        // Use global cross-file inferred analysis
                        Some(global_analysis)
                    } else if method.body.is_some() {
                        eprintln!("DEBUG CODEGEN TRAIT:   Using LOCAL analyzed (default impl)");
                        // Fallback to local analysis for default impl
                        analyzed.iter().find(|f| f.decl.name == method.name)
                    } else {
                        eprintln!("DEBUG CODEGEN TRAIT:   No analysis found");
                        None
                    }
                } else if method.body.is_some() {
                    eprintln!("DEBUG CODEGEN TRAIT:   No global, using LOCAL for default impl");
                    // No global analysis available, use local for default impl
                    analyzed.iter().find(|f| f.decl.name == method.name)
                } else {
                    eprintln!("DEBUG CODEGEN TRAIT:   No analysis at all");
                    None
                };
            output.push_str(&self.indent());

            if method.is_async {
                output.push_str("async ");
            }

            output.push_str("fn ");
            output.push_str(&method.name);
            output.push('(');

            // Generate parameters
            // NOTE: Trait method signatures cannot have 'mut' keyword in Rust
            // Only implementations can have 'mut self' or 'mut param'
            eprintln!(
                "DEBUG CODEGEN: Generating params for {}.{}, analyzed_method is_some: {}",
                trait_decl.name,
                method.name,
                analyzed_method.is_some()
            );
            if let Some(am) = analyzed_method {
                eprintln!(
                    "DEBUG CODEGEN:   analyzed_method.inferred_ownership keys: {:?}",
                    am.inferred_ownership.keys().collect::<Vec<_>>()
                );
                for (k, v) in &am.inferred_ownership {
                    eprintln!("DEBUG CODEGEN:     {} = {:?}", k, v);
                }
            }
            let params: Vec<String> = method
                .parameters
                .iter()
                .map(|param| {
                    eprintln!("DEBUG CODEGEN:   Processing param: {}", param.name);
                    use crate::parser::OwnershipHint;

                    // THE WINDJAMMER WAY:
                    // Use the analyzed ownership from the analyzer, which has inferred
                    // the most permissive signature needed based on ALL implementations!
                    let ownership = if let Some(analyzed) = analyzed_method {
                        // Has default implementation OR global cross-file analysis - use analyzer's inferred ownership
                        match analyzed.inferred_ownership.get(&param.name) {
                            Some(OwnershipMode::Borrowed) => {
                                eprintln!("DEBUG CODEGEN: {} param {} -> Borrowed (&)", method.name, param.name);
                                OwnershipHint::Ref
                            }
                            Some(OwnershipMode::MutBorrowed) => {
                                eprintln!("DEBUG CODEGEN: {} param {} -> MutBorrowed (&mut)", method.name, param.name);
                                OwnershipHint::Mut
                            }
                            Some(OwnershipMode::Owned) => {
                                eprintln!("DEBUG CODEGEN: {} param {} -> Owned", method.name, param.name);
                                OwnershipHint::Owned
                            }
                            None => {
                                eprintln!("DEBUG CODEGEN: {} param {} -> None, using AST", method.name, param.name);
                                param.ownership.clone() // Fallback to AST
                            }
                        }
                    } else {
                        // No default implementation - check analyzed_trait_methods
                        // The analyzer has inferred the signature from ALL impls!
                        if let Some(trait_methods) = self.analyzed_trait_methods.get(&trait_decl.name) {
                            if let Some(method_analysis) = trait_methods.get(&method.name) {
                                if let Some(inferred_ownership) = method_analysis.inferred_ownership.get(&param.name) {
                                    eprintln!("DEBUG CODEGEN: Trait {} method {} param {} inferred as {:?}",
                                        trait_decl.name, method.name, param.name, inferred_ownership);
                                    match inferred_ownership {
                                        OwnershipMode::Borrowed => OwnershipHint::Ref,
                                        OwnershipMode::MutBorrowed => OwnershipHint::Mut,
                                        OwnershipMode::Owned => OwnershipHint::Owned,
                                    }
                                } else {
                                    eprintln!("DEBUG CODEGEN: Trait {} method {} param {} - NO INFERRED OWNERSHIP, using AST",
                                        trait_decl.name, method.name, param.name);
                                    param.ownership.clone()
                                }
                            } else {
                                eprintln!("DEBUG CODEGEN: Trait {} method {} - NOT FOUND in analyzed methods",
                                    trait_decl.name, method.name);
                                // Fallback to AST
                                param.ownership.clone()
                            }
                        } else {
                            eprintln!("DEBUG CODEGEN: Trait {} - NOT FOUND in analyzed_trait_methods", trait_decl.name);
                            // Fallback to AST
                            param.ownership.clone()
                        }
                    };

                    // THE WINDJAMMER WAY: Check if param.type_ already contains a reference
                    // If so, don't add another & (prevents &&Input bug)
                    use crate::parser::Type;
                    let type_already_has_ref = matches!(param.type_, Type::Reference(_) | Type::MutableReference(_));

                    let type_str = match &ownership {
                        OwnershipHint::Owned => {
                            if param.name == "self" {
                                // Trait signatures: just 'self' (no 'mut')
                                return "self".to_string();
                            }
                            // Trait signatures: no 'mut' for parameters
                            return format!("{}: {}", param.name, self.type_to_rust(&param.type_));
                        }
                        OwnershipHint::Ref => {
                            if param.name == "self" {
                                return "&self".to_string();
                            }
                            // CRITICAL FIX: If type already has &, don't add another!
                            if type_already_has_ref {
                                self.type_to_rust(&param.type_) // Already has &
                            } else {
                                format!("&{}", self.type_to_rust(&param.type_))
                            }
                        }
                        OwnershipHint::Mut => {
                            if param.name == "self" {
                                return "&mut self".to_string();
                            }
                            // CRITICAL FIX: If type already has &mut, don't add another!
                            if type_already_has_ref {
                                self.type_to_rust(&param.type_) // Already has &mut
                            } else {
                                format!("&mut {}", self.type_to_rust(&param.type_))
                            }
                        }
                        OwnershipHint::Inferred => {
                            // TRAIT SIGNATURES: Default to &self for trait methods
                            // This prevents E0277 (Self not Sized) errors
                            if param.name == "self" {
                                "&self".to_string()
                            } else {
                                // Owned parameter (no &)
                                self.type_to_rust(&param.type_)
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
        output.push_str(&codegen_helpers::format_where_clause(
            &impl_block.where_clause,
        ));

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

        // Store the wasm export flag and trait impl flag for use in generate_function
        let old_in_wasm_impl = self.in_wasm_bindgen_impl;
        let old_in_trait_impl = self.in_trait_impl;
        self.in_wasm_bindgen_impl = has_wasm_export;
        self.in_trait_impl = impl_block.trait_name.is_some();

        for func in &impl_block.functions {
            // Find the analyzed version of this function
            // Match on both function name AND parent type to handle multiple impl blocks with same method names
            if let Some(analyzed_func) = analyzed
                .iter()
                .find(|af| af.decl.name == func.name && af.decl.parent_type == func.parent_type)
            {
                output.push_str(&self.generate_function(analyzed_func));
                output.push('\n');
            }
        }

        self.in_wasm_bindgen_impl = old_in_wasm_impl;
        self.in_trait_impl = old_in_trait_impl;

        self.indent_level -= 1;
        output.push('}');
        output
    }

    // Helper method for expressions that need to be evaluated without &mut self
    fn generate_expression_immut(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => name.clone(),
            _ => "/* expression */".to_string(),
        }
    }

    fn function_returns_self_type(&self, func: &FunctionDecl) -> bool {
        // Check if the function returns Self (for builder pattern detection)
        use crate::parser::{Expression, Statement, Type};

        // First check if return type is a custom type (struct type)
        let returns_custom_type = matches!(&func.return_type, Some(Type::Custom(_)));

        if !returns_custom_type {
            return false;
        }

        // Now check if the function body actually returns `self`
        // Check the last statement in the body
        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Explicit return self
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                Statement::Expression { expr, .. } => {
                    // Implicit return self (last expression)
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn function_modifies_self(&self, func: &FunctionDecl) -> bool {
        // Check if the function body modifies self (specifically for self parameters)
        for stmt in &func.body {
            if self.statement_modifies_self(stmt) {
                return true;
            }
        }
        false
    }

    fn statement_modifies_self(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if target is self.field
                self.expression_is_self_field_modification(target)
            }
            Statement::Expression { expr, .. } => {
                // Check for mutating method calls like self.field.push()
                self.expression_modifies_self(expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| self.statement_modifies_self(s))
                    || else_block
                        .as_ref()
                        .is_some_and(|block| block.iter().any(|s| self.statement_modifies_self(s)))
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                body.iter().any(|s| self.statement_modifies_self(s))
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                // Match arms have a body expression, check if it contains modifications
                self.expression_modifies_self(&arm.body)
            }),
            _ => false,
        }
    }

    fn expression_is_self_field_modification(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    fn expression_modifies_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                statements.iter().any(|s| self.statement_modifies_self(s))
            }
            Expression::MethodCall { object, method, .. } => {
                // Check if this is a mutating method call on self.field
                // Common mutating methods: push, pop, remove, insert, clear, etc.
                let is_mutating_method = matches!(
                    method.as_str(),
                    "push"
                        | "pop"
                        | "remove"
                        | "insert"
                        | "clear"
                        | "append"
                        | "extend"
                        | "drain"
                        | "truncate"
                        | "resize"
                        | "swap_remove"
                        | "retain"
                );

                if is_mutating_method {
                    // Check if the object is self.field
                    if let Expression::FieldAccess {
                        object: field_obj, ..
                    } = &**object
                    {
                        if matches!(&**field_obj, Expression::Identifier { name, .. } if name == "self")
                        {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Generate extern "C" function declaration for FFI
    fn generate_extern_function(&self, func: &FunctionDecl) -> String {
        let mut output = String::new();

        output.push_str("    fn ");
        output.push_str(&func.name);

        // Add type parameters if present
        if !func.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&func.type_params));
            output.push('>');
        }

        output.push('(');

        // Generate parameters
        let params: Vec<String> = func
            .parameters
            .iter()
            .map(|param| format!("{}: {}", param.name, self.type_to_rust(&param.type_)))
            .collect();

        output.push_str(&params.join(", "));
        output.push(')');

        // Add return type if present
        if let Some(ret_type) = &func.return_type {
            output.push_str(" -> ");
            output.push_str(&self.type_to_rust(ret_type));
        }

        output.push_str(";\n");
        output
    }

    fn generate_function(&mut self, analyzed: &AnalyzedFunction) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // LOCAL VARIABLE TRACKING: Push new scope for this function
        self.local_variable_scopes
            .push(std::collections::HashSet::new());

        // AUTO-CLONE: Load auto-clone analysis for this function
        self.auto_clone_analysis = Some(analyzed.auto_clone_analysis.clone());

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        // Track function parameters for compound assignment optimization
        self.current_function_params = func.parameters.clone();

        // Track function return type for string literal conversion
        self.current_function_return_type = func.return_type.clone();

        // Track function body for data flow analysis
        self.current_function_body = func.body.clone();

        // Track parameters inferred as borrowed for field access cloning
        self.inferred_borrowed_params.clear();
        for (param_name, ownership) in &analyzed.inferred_ownership {
            if matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
                self.inferred_borrowed_params.insert(param_name.clone());
            }
        }

        // WINDJAMMER FIX: Track usize-typed parameters for auto-cast logic
        // Clear from previous function to prevent variable leakage between functions
        self.usize_variables.clear();

        // When a parameter is declared as `usize`, add it to usize_variables
        // so expression_produces_usize() correctly identifies it
        for (param_idx, param) in func.parameters.iter().enumerate() {
            // Use inferred type if available, otherwise use declared type
            let param_type = analyzed
                .inferred_param_types
                .get(param_idx)
                .unwrap_or(&param.type_);

            // Check if this parameter is usize
            if matches!(param_type, Type::Custom(name) if name == "usize") {
                self.usize_variables.insert(param.name.clone());
            }
        }

        // PHASE 8 OPTIMIZATION: Load SmallVec optimizations for this function
        // DISABLED: SmallVec optimizations conflict with return types
        // TODO: Re-enable with smarter conversion at return sites
        self.smallvec_optimizations.clear();
        // for opt in &analyzed.smallvec_optimizations {
        //     self.smallvec_optimizations
        //         .insert(opt.variable.clone(), opt.clone());
        //     self.needs_smallvec_import = true; // Mark that we need the smallvec crate
        // }

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

        // Generate doc comment if present
        if let Some(doc_comment) = &func.doc_comment {
            for line in doc_comment.lines() {
                output.push_str(&format!("/// {}\n", line.trim()));
            }
        }

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

            // Skip @export - it's used to determine visibility but doesn't map to a Rust attribute for native targets
            if decorator.name == "export" && self.target != CompilationTarget::Wasm {
                continue;
            }

            // Skip game framework decorators - they're handled by the game loop
            if matches!(
                decorator.name.as_str(),
                "game" | "init" | "update" | "render" | "render3d" | "input" | "cleanup"
            ) {
                continue;
            }

            let rust_attr = self.map_decorator(&decorator.name);
            output.push_str(&format!("#[{}]\n", rust_attr));
        }

        // Add `pub` if function is marked pub OR we're in a #[wasm_bindgen] impl block OR compiling a module OR has @export decorator
        // BUT NOT if we're in a trait implementation (trait methods cannot have visibility modifiers)
        let has_export = func.decorators.iter().any(|d| d.name == "export");
        if !self.in_trait_impl
            && (func.is_pub || self.in_wasm_bindgen_impl || self.is_module || has_export)
        {
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

        // Add implicit &self or &mut self for impl block methods that access fields
        // THE WINDJAMMER WAY: Constructors (associated functions) should NOT get self added!
        let mut params: Vec<String> = Vec::new();
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");

        // THE WINDJAMMER WAY: Auto-Self Inference
        // Check if analyzer inferred a self parameter (even if not in AST)
        let has_inferred_self = analyzed.inferred_ownership.contains_key("self");

        // Check if this is a constructor (associated function returning the struct type)
        // A constructor returns the struct being implemented, e.g., fn new() -> Tilemap
        let is_constructor = !has_explicit_self && !has_inferred_self && {
            if let Some(Type::Custom(return_type_name)) = &func.return_type {
                // Check if return type matches current struct name
                self.current_struct_name
                    .as_ref()
                    .is_some_and(|struct_name| struct_name == return_type_name)
            } else {
                false
            }
        };

        // Priority 1: Use analyzer's inferred self if available
        if has_inferred_self && !has_explicit_self {
            if let Some(ownership) = analyzed.inferred_ownership.get("self") {
                let self_param = match ownership {
                    OwnershipMode::Borrowed => "&self",
                    OwnershipMode::MutBorrowed => "&mut self",
                    OwnershipMode::Owned => {
                        // Check if function modifies self (builder pattern)
                        if self.function_modifies_self(&analyzed.decl) {
                            "mut self"
                        } else {
                            "self"
                        }
                    }
                };
                params.push(self_param.to_string());
            }
        }
        // Priority 2: Fallback to old field-based analysis (for backwards compatibility)
        else if self.in_impl_block
            && !has_explicit_self
            && !self.current_struct_fields.is_empty()
            && !is_constructor
        {
            // Check if function body mutates any struct fields
            let ctx =
                self_analysis::AnalysisContext::new(&func.parameters, &self.current_struct_fields);
            if self_analysis::function_mutates_fields(&ctx, func) {
                // Check if this is a builder pattern (modifies fields AND returns Self)
                let returns_self = self.function_returns_self_type(func);
                if returns_self {
                    // Builder pattern: use `mut self` (consuming)
                    params.push("mut self".to_string());
                } else {
                    // Regular mutating method: use `&mut self` (borrowing)
                    params.push("&mut self".to_string());
                }
            } else if self_analysis::function_accesses_fields(&ctx, func) {
                // Only read access needed
                params.push("&self".to_string());
            }
        }

        let additional_params: Vec<String> = func
            .parameters
            .iter()
            .enumerate()
            .map(|(param_idx, param)| {
                // SMART STRING INFERENCE: Use the inferred type from analyzer (string → &str vs String)
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(param_idx)
                    .unwrap_or(&param.type_);

                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(inferred_type);
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
                            // Check if analyzer inferred a different ownership for self
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => {
                                        // Check if function actually modifies self
                                        // Only add 'mut' if it does
                                        if self.function_modifies_self(&analyzed.decl) {
                                            return "mut self".to_string();
                                        } else {
                                            return "self".to_string();
                                        }
                                    }
                                }
                            }
                            // Default: check if function modifies self
                            if self.function_modifies_self(&analyzed.decl) {
                                return "mut self".to_string();
                            } else {
                                return "self".to_string();
                            }
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(inferred_type));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            // Check if analyzer inferred a different ownership (e.g., &mut self)
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => {
                                        // Shouldn't happen for explicit &self, but handle it
                                        return "self".to_string();
                                    }
                                }
                            }
                            return "&self".to_string();
                        }
                        // Don't add & if the type is already a Reference
                        if matches!(
                            inferred_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            self.type_to_rust(inferred_type)
                        } else {
                            format!("&{}", self.type_to_rust(inferred_type))
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(inferred_type, Type::MutableReference(_)) {
                            self.type_to_rust(inferred_type)
                        } else {
                            format!("&mut {}", self.type_to_rust(inferred_type))
                        }
                    }
                    OwnershipHint::Inferred => {
                        // SMART STRING INFERENCE: inferred_type already has &str vs String resolved!
                        // For strings: Type::Reference(String) → &str, Type::String → String
                        // For other types: Apply ownership mode from analyzer

                        // Special handling for `self` parameters (trait impl methods)
                        if param.name == "self" {
                            // Check analyzer for inferred ownership
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => {
                                        // Check if function actually modifies self
                                        if self.function_modifies_self(&analyzed.decl) {
                                            return "mut self".to_string();
                                        } else {
                                            return "self".to_string();
                                        }
                                    }
                                }
                            }
                            // Default: check if function modifies self
                            if self.function_modifies_self(&analyzed.decl) {
                                return "mut self".to_string();
                            } else {
                                return "self".to_string();
                            }
                        }

                        // Check if type already has ownership baked in (like &str from string inference)
                        if matches!(
                            inferred_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            // Already has & or &mut - just convert
                            self.type_to_rust(inferred_type)
                        } else {
                            // Apply ownership mode from analyzer
                            let ownership_mode = analyzed
                                .inferred_ownership
                                .get(&param.name)
                                .unwrap_or(&OwnershipMode::Borrowed);

                            match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(inferred_type),
                                OwnershipMode::Borrowed => {
                                    if type_analysis::is_copy_type(inferred_type) {
                                        // Copy types pass by value even when borrowed
                                        self.type_to_rust(inferred_type)
                                    } else {
                                        format!("&{}", self.type_to_rust(inferred_type))
                                    }
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(inferred_type))
                                }
                            }
                        }
                    }
                };

                // Check if parameter is declared with 'mut' keyword
                let mut_prefix = if param.is_mutable { "mut " } else { "" };

                // Check if this is a pattern parameter
                if let Some(pattern) = &param.pattern {
                    // Generate pattern: type syntax
                    format!(
                        "{}{}: {}",
                        mut_prefix,
                        self.generate_pattern(pattern),
                        type_str
                    )
                } else {
                    // Simple name: type syntax
                    format!("{}{}: {}", mut_prefix, param.name, type_str)
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
        output.push_str(&codegen_helpers::format_where_clause(&func.where_clause));

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

        // LOCAL VARIABLE TRACKING: Pop scope when exiting function
        self.local_variable_scopes.pop();

        output
    }

    fn type_to_rust(&self, type_: &Type) -> String {
        // Delegate to the refactored types module
        crate::codegen::rust::type_to_rust(type_)
    }

    fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Reference(inner) => format!("&{}", self.pattern_to_rust(inner)),
            Pattern::Tuple(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                format!("({})", rust_patterns.join(", "))
            }
            Pattern::EnumVariant(variant, binding) => {
                use crate::parser::EnumPatternBinding;
                match binding {
                    EnumPatternBinding::Single(name) => format!("{}({})", variant, name),
                    EnumPatternBinding::Wildcard => format!("{}(_)", variant),
                    EnumPatternBinding::None => variant.clone(),
                    EnumPatternBinding::Tuple(patterns) => {
                        let rust_patterns: Vec<String> =
                            patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                        format!("{}({})", variant, rust_patterns.join(", "))
                    }
                    EnumPatternBinding::Struct(fields, has_wildcard) => {
                        if fields.is_empty() {
                            // Empty struct binding means { .. } wildcard
                            format!("{} {{ .. }}", variant)
                        } else {
                            let field_strs: Vec<String> = fields
                                .iter()
                                .map(|(name, pat)| {
                                    format!("{}: {}", name, self.pattern_to_rust(pat))
                                })
                                .collect();
                            if *has_wildcard {
                                // Partial match: { field1, field2, .. }
                                format!("{} {{ {}, .. }}", variant, field_strs.join(", "))
                            } else {
                                // Complete match: { field1, field2 }
                                format!("{} {{ {} }}", variant, field_strs.join(", "))
                            }
                        }
                    }
                }
            }
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Or(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                rust_patterns.join(" | ")
            }
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> String {
        // Record source mapping if location info is available
        if let Some(location) = codegen_helpers::get_statement_location(stmt) {
            self.record_mapping(&location);
        }

        match stmt {
            Statement::Let {
                pattern,
                mutable,
                type_,
                value,
                ..
            } => {
                let mut output = self.indent();
                output.push_str("let ");

                // Check if we need &mut for index access on borrowed fields
                // e.g., let enemy = self.enemies[i] should be let enemy = &mut self.enemies[i]
                let needs_mut_ref = self.should_mut_borrow_index_access(value);

                // Extract variable name for optimizations (only works for simple identifiers)
                let var_name = match pattern {
                    Pattern::Identifier(name) => Some(name.as_str()),
                    _ => None,
                };

                if needs_mut_ref {
                    // Don't add mut keyword, but we'll add &mut to the value
                } else if *mutable {
                    output.push_str("mut ");
                }

                // Generate pattern (could be simple name or tuple)
                let pattern_str = self.generate_pattern(pattern);
                output.push_str(&pattern_str);

                // LOCAL VARIABLE TRACKING: Add this variable to the current scope
                // This enables proper shadowing of field names
                if let Some(name) = var_name {
                    if let Some(current_scope) = self.local_variable_scopes.last_mut() {
                        current_scope.insert(name.to_string());
                    }
                }

                // PHASE 8: Check if this variable should use SmallVec
                if let Some(name) = var_name {
                    if let Some(smallvec_opt) = self.smallvec_optimizations.get(name) {
                        // Use SmallVec with stack allocation
                        // If there's a type annotation, extract the element type
                        let elem_type = if let Some(Type::Vec(inner)) = type_ {
                            self.type_to_rust(inner)
                        } else {
                            "_".to_string() // Type inference
                        };
                        output.push_str(&format!(
                            ": SmallVec<[{}; {}]>",
                            elem_type, smallvec_opt.stack_size
                        ));
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

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type {
                            let should_convert = matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            );
                            if should_convert {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }
                        output.push_str(&value_str);
                    } else {
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }

                        // EXPRESSION CONTEXT: Mark that we're generating a value that will be used
                        // This prevents adding semicolons to if-else branches when used in let bindings
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // String literals assigned to variables should become String (not &str)
                        // because they may be passed to functions expecting String later.
                        // This is safe because String auto-borrows to &str when needed.
                        let mut value_str = self.generate_expression(value);

                        // AUTO-CLONE: Vec indexing of non-Copy types needs .clone()
                        // DATA FLOW ANALYSIS: Check if variable is only used for field access
                        if matches!(value, Expression::Index { .. }) {
                            if let Some(name) = var_name {
                                // HEURISTIC: Only apply smart borrowing for struct-like variable names
                                // Primitive types (int, float, bool) are Copy and don't need special handling
                                let struct_like_names = [
                                    "frame",
                                    "point",
                                    "pos",
                                    "position",
                                    "region",
                                    "sprite",
                                    "entity_data",
                                    "component",
                                ];
                                let is_likely_struct = struct_like_names
                                    .iter()
                                    .any(|pattern| name.contains(pattern));

                                if is_likely_struct {
                                    // Analyze how this variable is used after declaration
                                    if self.variable_is_only_field_accessed(name) {
                                        // Variable only used for field access → auto-borrow (don't clone)
                                        value_str = format!("&{}", value_str);
                                    } else {
                                        // Variable is moved/returned → need to clone
                                        value_str = format!("{}.clone()", value_str);
                                    }
                                }
                                // For non-struct-like names (likely primitives), do nothing
                                // Let Rust's copy semantics handle it
                            }
                        } else if matches!(
                            value,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            value_str = format!("{}.to_string()", value_str);
                        }

                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                    }
                } else {
                    // No SmallVec optimization for this variable
                    if let Some(t) = type_ {
                        output.push_str(": ");
                        output.push_str(&self.type_to_rust(t));
                        output.push_str(" = ");

                        // EXPRESSION CONTEXT: Mark that we're generating a value
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type {
                            let should_convert = matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            );
                            if should_convert {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }

                        if needs_mut_ref {
                            value_str = format!("&mut {}", value_str);
                        }
                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                    } else {
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }

                        // EXPRESSION CONTEXT: Mark that we're generating a value
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        // WINDJAMMER PHILOSOPHY: Auto-convert mutable string variables
                        // When a mutable variable is initialized with a string literal,
                        // it should be a String (not &str) because &str can't be mutated
                        let mut value_str = self.generate_expression(value);
                        if *mutable
                            && matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            )
                        {
                            value_str = format!("{}.to_string()", value_str);
                        }

                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                    }
                }

                output.push_str(";\n");

                // Track variables assigned from .len() as usize type
                // OR variables with explicit usize type annotation
                // This enables auto-casting in comparisons with i32
                if let Some(name) = var_name {
                    let is_usize = self.expression_produces_usize(value)
                        || matches!(type_, Some(Type::Custom(s)) if s == "usize");
                    if is_usize {
                        self.usize_variables.insert(name.to_string());
                    }
                }

                output
            }
            Statement::Const {
                name, type_, value, ..
            } => {
                let mut output = self.indent();

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

                output.push_str(&format!(
                    "const {}: {} = {};\n",
                    name,
                    rust_type,
                    self.generate_expression(value)
                ));
                output
            }
            Statement::Static {
                name,
                mutable,
                type_,
                value,
                ..
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
            Statement::Return { value: expr, .. } => {
                let mut output = self.indent();
                output.push_str("return");
                if let Some(e) = expr {
                    output.push(' ');
                    let mut return_str = self.generate_expression(e);

                    // WINDJAMMER PHILOSOPHY: Auto-convert string literals in return statements
                    // when the function returns String
                    let returns_string = match &self.current_function_return_type {
                        Some(Type::String) => true,
                        Some(Type::Custom(name)) if name == "String" => true,
                        _ => false,
                    };

                    if returns_string {
                        // String literal needs .to_string()
                        if matches!(
                            e,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) && !return_str.ends_with(".to_string()")
                        {
                            return_str = format!("{}.to_string()", return_str);
                        }
                        // self.field needs .clone() when self is borrowed
                        else if let Expression::FieldAccess { object, .. } = e {
                            if let Expression::Identifier { name: obj_name, .. } = &**object {
                                if obj_name == "self" && !return_str.ends_with(".clone()") {
                                    let self_is_borrowed =
                                        self.current_function_params.iter().any(|p| {
                                            p.name == "self"
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
                                        });
                                    if self_is_borrowed {
                                        return_str = format!("{}.clone()", return_str);
                                    }
                                }
                            }
                        }
                    }

                    // FIXED: Auto-cast usize to i64 when function returns int
                    // WINDJAMMER PHILOSOPHY: Compiler handles type conversions automatically
                    let returns_int = match &self.current_function_return_type {
                        Some(Type::Int) => true,
                        Some(Type::Custom(name)) if name == "i64" || name == "int" => true,
                        _ => false,
                    };

                    if returns_int && self.expression_produces_usize(e) {
                        // .len() returns usize, but function expects i64 - auto-cast!
                        return_str = format!("{} as i64", return_str);
                    }

                    // WINDJAMMER PHILOSOPHY: Auto-add .cloned() for HashMap.get() and similar methods
                    // When returning Option<T> but method returns Option<&T>, add .cloned()
                    // Common case: fn get(&self, key: K) -> Option<V> { self.map.get(&key) }
                    let returns_option_owned = self.returns_option_owned_type();
                    if returns_option_owned
                        && self.is_method_returning_option_ref(e)
                        && !return_str.ends_with(".cloned()")
                        && !return_str.ends_with(".clone()")
                    {
                        return_str = format!("{}.cloned()", return_str);
                    }

                    output.push_str(&return_str);
                }
                output.push_str(";\n");
                output
            }
            Statement::Expression { expr, .. } => {
                let mut output = self.indent();
                output.push_str(&self.generate_expression(expr));
                output.push_str(";\n");
                output
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // WINDJAMMER PHILOSOPHY: Check if any branch explicitly uses .as_str()
                // If so, we should NOT auto-convert string literals in other branches
                let any_branch_has_as_str = string_analysis::block_has_as_str(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| string_analysis::block_has_as_str(b));

                let old_suppress = self.suppress_string_conversion;
                if any_branch_has_as_str {
                    self.suppress_string_conversion = true;
                }

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

                self.suppress_string_conversion = old_suppress;
                output.push('\n');
                output
            }
            Statement::Match { value, arms, .. } => {
                let mut output = self.indent();
                output.push_str("match ");

                // Check if any arm has a string literal pattern
                // If so, add .as_str() to the match value for String types
                // BUT: Don't add .as_str() if the match value is a tuple (tuple patterns handle their own string matching)
                let has_string_literal = arms
                    .iter()
                    .any(|arm| pattern_analysis::pattern_has_string_literal(&arm.pattern));

                let is_tuple_match = arms
                    .iter()
                    .any(|arm| matches!(arm.pattern, Pattern::Tuple(_)));

                let value_str = self.generate_expression(value);
                if has_string_literal && !is_tuple_match {
                    // Add .as_str() if the value doesn't already end with it
                    if !value_str.ends_with(".as_str()") {
                        output.push_str(&format!("{}.as_str()", value_str));
                    } else {
                        output.push_str(&value_str);
                    }
                } else {
                    output.push_str(&value_str);
                }

                output.push_str(" {\n");

                self.indent_level += 1;

                // WINDJAMMER PHILOSOPHY: Auto-convert match arm strings when return type is String
                // OR when any other arm produces a String (e.g., format! macro, or blocks with converted strings)
                let needs_string_conversion = match &self.current_function_return_type {
                    Some(Type::String) => true,
                    Some(Type::Custom(name)) if name == "String" => true,
                    _ => {
                        // Check if any arm produces a String (format!, to_string(), etc.)
                        // OR if any arm has a block whose last expression is converted to String
                        arms.iter().any(|arm| {
                            string_analysis::expression_produces_string(&arm.body)
                                || arm_string_analysis::arm_returns_converted_string(&arm.body)
                        })
                    }
                };

                for arm in arms {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_pattern(&arm.pattern));

                    // Add guard if present
                    if let Some(guard) = &arm.guard {
                        output.push_str(" if ");
                        output.push_str(&self.generate_expression(guard));
                    }

                    output.push_str(" => ");

                    // Set context flag for block generation
                    let old_in_match_arm = self.in_match_arm_needing_string;
                    if needs_string_conversion {
                        self.in_match_arm_needing_string = true;
                    }

                    // Auto-convert string literals to String when any arm produces String
                    let mut arm_str = self.generate_expression(&arm.body);

                    self.in_match_arm_needing_string = old_in_match_arm;
                    let is_string_literal = matches!(
                        &arm.body,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    );

                    // Only apply .to_string() for direct string literals, NOT blocks
                    // Blocks handle their own string conversion via in_match_arm_needing_string flag
                    if needs_string_conversion && is_string_literal {
                        arm_str = format!("{}.to_string()", arm_str);
                    }

                    output.push_str(&arm_str);
                    output.push_str(",\n");
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Loop { body, .. } => {
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
            Statement::While {
                condition, body, ..
            } => {
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
                pattern,
                iterable,
                body,
                ..
            } => {
                let mut output = self.indent();
                output.push_str("for ");

                // Check if the loop body modifies the loop variable
                let pattern_str = self.pattern_to_rust(pattern);
                let loop_var = pattern_analysis::extract_pattern_identifier(pattern);
                let needs_mut = loop_var
                    .as_ref()
                    .is_some_and(|var| self.loop_body_modifies_variable(body, var));

                if needs_mut {
                    output.push_str("mut ");
                }
                output.push_str(&pattern_str);
                output.push_str(" in ");

                // Check if we need to add & for borrowed iteration
                // This handles the common case of iterating over fields of borrowed structs
                let needs_borrow = self.should_borrow_for_iteration(iterable);
                let needs_mut_borrow = needs_mut && needs_borrow;

                // BORROWED ITERATOR: Track if iterator variable is from borrowed collection
                // So we can auto-clone when pushing to new collections
                // If we add & to the iterable, the iterator variable is a reference
                let is_borrowed_iterator =
                    needs_borrow || self.is_iterating_over_borrowed(iterable);

                if needs_mut_borrow {
                    output.push_str("&mut ");
                } else if needs_borrow {
                    output.push('&');
                }

                output.push_str(&self.generate_expression(iterable));
                output.push_str(" {\n");

                self.indent_level += 1;

                // Track borrowed iterator variable for field access cloning
                if is_borrowed_iterator {
                    if let Some(var) = &loop_var {
                        self.borrowed_iterator_vars.insert(var.clone());
                    }
                }

                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }

                // Remove iterator variable from tracking after loop
                if is_borrowed_iterator {
                    if let Some(var) = &loop_var {
                        self.borrowed_iterator_vars.remove(var);
                    }
                }

                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Break { .. } => {
                let mut output = self.indent();
                output.push_str("break;\n");
                output
            }
            Statement::Continue { .. } => {
                let mut output = self.indent();
                output.push_str("continue;\n");
                output
            }
            Statement::Use { path, alias, .. } => {
                let mut output = self.indent();
                output.push_str("use ");
                output.push_str(&path.join("::"));
                if let Some(alias_name) = alias {
                    output.push_str(" as ");
                    output.push_str(alias_name);
                }
                output.push_str(";\n");
                output
            }
            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => {
                let mut output = self.indent();

                // Check if this is a compound assignment (+=, -=, etc.)
                if let Some(op) = compound_op {
                    // Generate compound assignment: target += value
                    // CRITICAL: Set flag to suppress auto-clone for assignment targets
                    self.generating_assignment_target = true;
                    output.push_str(&self.generate_expression(target));
                    self.generating_assignment_target = false;

                    // Generate compound operator
                    output.push_str(match op {
                        CompoundOp::Add => " += ",
                        CompoundOp::Sub => " -= ",
                        CompoundOp::Mul => " *= ",
                        CompoundOp::Div => " /= ",
                        CompoundOp::Mod => " %= ",
                        CompoundOp::BitAnd => " &= ",
                        CompoundOp::BitOr => " |= ",
                        CompoundOp::BitXor => " ^= ",
                        CompoundOp::Shl => " <<= ",
                        CompoundOp::Shr => " >>= ",
                    });

                    output.push_str(&self.generate_expression(value));
                    output.push_str(";\n");
                    return output;
                }

                // Regular assignment: target = value

                // PHASE 5 OPTIMIZATION: Check if this assignment matches x = x + y pattern
                // If so, convert to compound assignment: x += y
                if let Expression::Identifier {
                    name: target_var, ..
                } = target
                {
                    if let Expression::Binary {
                        left, right, op, ..
                    } = value
                    {
                        if let Expression::Identifier { name: left_var, .. } = &**left {
                            if left_var == target_var {
                                // Pattern matched: x = x op y → x op= y
                                let compound_op_str = match op {
                                    BinaryOp::Add => Some("+="),
                                    BinaryOp::Sub => Some("-="),
                                    BinaryOp::Mul => Some("*="),
                                    BinaryOp::Div => Some("/="),
                                    _ => None,
                                };

                                if let Some(op_str) = compound_op_str {
                                    // Generate compound assignment instead
                                    self.generating_assignment_target = true;
                                    output.push_str(&self.generate_expression(target));
                                    self.generating_assignment_target = false;
                                    output.push(' ');
                                    output.push_str(op_str);
                                    output.push(' ');
                                    output.push_str(&self.generate_expression(right));
                                    output.push_str(";\n");
                                    return output;
                                }
                            }
                        }
                    }
                }

                // Fall back to regular assignment
                // CRITICAL: Set flag to suppress auto-clone for assignment targets
                self.generating_assignment_target = true;
                output.push_str(&self.generate_expression(target));
                self.generating_assignment_target = false;
                output.push_str(" = ");

                // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                // When assigning a string literal to a field, it likely needs .to_string()
                let mut value_str = self.generate_expression(value);
                if matches!(
                    value,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) {
                    // String literal assigned to field - add .to_string()
                    value_str = format!("{}.to_string()", value_str);
                }

                // AUTO-CAST: When assigning usize (.len() result) to non-usize field, cast
                // WINDJAMMER PHILOSOPHY: Compiler does the work - no explicit casting needed
                if self.expression_produces_usize(value) {
                    // Check target field type to determine cast type
                    let target_type = self.get_assignment_target_type(target);

                    match target_type.as_deref() {
                        Some("usize") => {
                            // Target is usize, no cast needed!
                        }
                        Some("i64") | Some("int") => {
                            // Target is i64, cast to i64
                            value_str = format!("(({}) as i64)", value_str);
                        }
                        Some("i32") => {
                            // Target is i32, cast to i32
                            value_str = format!("(({}) as i32)", value_str);
                        }
                        _ => {
                            // Unknown or generic type, don't cast (let Rust's type inference handle it)
                        }
                    }
                }

                output.push_str(&value_str);
                output.push_str(";\n");
                output
            }
            Statement::Thread { body, .. } => {
                // Transpile to std::thread::spawn for parallelism
                // When used as a statement, discard the JoinHandle
                let mut output = self.indent();
                output.push_str("let _ = std::thread::spawn(move || {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Async { body, .. } => {
                // Transpile to tokio::spawn for async concurrency
                // When used as a statement, discard the JoinHandle
                let mut output = self.indent();
                output.push_str("let _ = tokio::spawn(async move {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Defer { statement: _, .. } => {
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
            Pattern::Reference(inner) => format!("&{}", self.generate_pattern(inner)),
            Pattern::EnumVariant(name, binding) => {
                use crate::parser::EnumPatternBinding;
                match binding {
                    EnumPatternBinding::Single(b) => format!("{}({})", name, b),
                    EnumPatternBinding::Wildcard => format!("{}(_)", name),
                    EnumPatternBinding::None => name.clone(),
                    EnumPatternBinding::Tuple(patterns) => {
                        let rust_patterns: Vec<String> =
                            patterns.iter().map(|p| self.generate_pattern(p)).collect();
                        format!("{}({})", name, rust_patterns.join(", "))
                    }
                    EnumPatternBinding::Struct(fields, has_wildcard) => {
                        if fields.is_empty() {
                            // Empty struct binding means { .. } wildcard
                            format!("{} {{ .. }}", name)
                        } else {
                            let field_strs: Vec<String> = fields
                                .iter()
                                .map(|(n, pat)| format!("{}: {}", n, self.generate_pattern(pat)))
                                .collect();
                            if *has_wildcard {
                                // Partial match: { field1, field2, .. }
                                format!("{} {{ {}, .. }}", name, field_strs.join(", "))
                            } else {
                                // Complete match: { field1, field2 }
                                format!("{} {{ {} }}", name, field_strs.join(", "))
                            }
                        }
                    }
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

    /// Check if match needs .clone() to avoid partial move from self
    /// This is needed when:
    /// 1. Match value is a field access on `self` (e.g., self.selected_id)
    /// 2. Self is used again after the match (pattern extracts value)
    /// 3. The pattern extracts a non-Copy value (Some(id), Ok(val), etc.)
    fn match_needs_clone_for_self_field(
        &self,
        value: &Expression,
        arms: &[crate::parser::MatchArm],
    ) -> bool {
        // Check if value is self.field
        let is_self_field = if let Expression::FieldAccess { object, .. } = value {
            matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        } else {
            false
        };

        if !is_self_field {
            return false;
        }

        // Check if current function has self (either borrowed or owned)
        let has_self = self
            .current_function_params
            .iter()
            .any(|p| p.name == "self");

        if !has_self {
            return false;
        }

        // Check if any arm pattern extracts a value (not just wildcard or literal)
        arms.iter()
            .any(|arm| self.pattern_extracts_value(&arm.pattern))
    }

    /// Check if a pattern extracts a value that would cause a move
    fn pattern_extracts_value(&self, pattern: &Pattern) -> bool {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard | Pattern::Literal(_) => false,
            Pattern::Identifier(_) => true, // Binding moves the value
            Pattern::Reference(inner) => self.pattern_extracts_value(inner),
            Pattern::Tuple(patterns) => patterns.iter().any(|p| self.pattern_extracts_value(p)),
            Pattern::EnumVariant(_, binding) => match binding {
                EnumPatternBinding::None | EnumPatternBinding::Wildcard => false,
                EnumPatternBinding::Single(_) => true, // Some(id) extracts id
                EnumPatternBinding::Tuple(patterns) => {
                    patterns.iter().any(|p| self.pattern_extracts_value(p))
                }
                EnumPatternBinding::Struct(fields, _) => {
                    fields.iter().any(|(_, p)| self.pattern_extracts_value(p))
                }
            },
            Pattern::Or(patterns) => patterns.iter().any(|p| self.pattern_extracts_value(p)),
        }
    }

    /// Check if an expression produces a String (not &str)
    /// Used to detect match arm type consistency
    /// Check if a match arm returns a string that will be converted to String
    /// This handles cases like: if x { "a" } else { "b" } where the if-else branches
    /// will be auto-converted to String, making the whole arm return String
    /// Check if an expression produces usize (e.g., .len(), array indexing)
    /// Used for auto-casting between i32 and usize in comparisons
    fn get_assignment_target_type(&self, target: &Expression) -> Option<String> {
        // Determine the type of an assignment target (e.g., self.field or variable)
        match target {
            Expression::FieldAccess { object, field, .. } => {
                // Check if it's self.field
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self") {
                    // Use the tracked struct name and usize_struct_fields to infer type
                    if let Some(struct_name) = &self.current_struct_name {
                        // Check if this field is tracked as usize
                        if let Some(usize_fields) = self.usize_struct_fields.get(struct_name) {
                            if usize_fields.contains(field) {
                                return Some("usize".to_string());
                            }
                        }
                        // If not usize, assume it's i64 (int) for numeric types
                        // This is a heuristic - we can't know for sure without more type info
                        return Some("i64".to_string());
                    }
                }
            }
            Expression::Identifier { name, .. } => {
                // Check if it's a tracked usize variable
                if self.usize_variables.contains(name) {
                    return Some("usize".to_string());
                }
                // Unknown type for other variables
                return None;
            }
            _ => {}
        }
        None
    }

    fn expression_produces_usize(&self, expr: &Expression) -> bool {
        match expr {
            // .len() returns usize
            Expression::MethodCall { method, .. } => {
                method == "len" || method == "count" || method == "capacity"
            }
            // Binary ops with len: len() - 1, len() + offset, etc.
            Expression::Binary { left, right, .. } => {
                self.expression_produces_usize(left) || self.expression_produces_usize(right)
            }
            // Casts to usize: (x as usize)
            Expression::Cast { type_, .. } => {
                matches!(type_, Type::Custom(name) if name == "usize")
            }
            // Variables assigned from .len()
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return true;
                }

                // Check if this is a struct field with usize type (in impl block)
                if self.in_impl_block && self.current_struct_fields.contains(name) {
                    // Look up the struct to see if this field is usize
                    if let Some(struct_name) = &self.current_struct_name {
                        if let Some(usize_fields) = self.usize_struct_fields.get(struct_name) {
                            return usize_fields.contains(name);
                        }
                    }
                }

                false
            }
            // Field access: self.field_name or obj.field_name
            Expression::FieldAccess { object, field, .. } => {
                // Check if accessing a usize field on self
                if let Expression::Identifier { name: obj_name, .. } = &**object {
                    if obj_name == "self" && self.in_impl_block {
                        // Look up struct to see if this field is usize
                        if let Some(struct_name) = &self.current_struct_name {
                            if let Some(usize_fields) = self.usize_struct_fields.get(struct_name) {
                                return usize_fields.contains(field);
                            }
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if the function returns Option<T> where T is owned (not a reference)
    /// Used to detect when we need to add .cloned() for methods that return Option<&T>
    fn returns_option_owned_type(&self) -> bool {
        match &self.current_function_return_type {
            Some(Type::Option(inner_type)) => {
                // Check if the inner type is NOT a reference
                // If it's a simple type (String, int, custom types), it's owned
                !matches!(**inner_type, Type::Reference(_))
            }
            _ => false,
        }
    }

    /// Check if an expression is a method call that returns Option<&T>
    /// Common examples: HashMap::get(), Vec::first(), Vec::last(), Vec::get()
    fn is_method_returning_option_ref(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { method, .. } => {
                // Methods that return Option<&T>:
                // - HashMap/BTreeMap: get
                // - Vec/slice: get, first, last
                matches!(method.as_str(), "get" | "first" | "last")
            }
            _ => false,
        }
    }

    fn generate_expression_with_precedence(&mut self, expr: &Expression) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. }
            | Expression::Binary { .. }
            | Expression::Closure { .. }
            | Expression::Unary { .. } => {
                // Unary expressions like (*entity).field need parens for correct precedence
                // Without parens: *entity.field means *(entity.field) - WRONG
                // With parens: (*entity).field means dereference then access field - CORRECT
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        }
    }

    // PHASE 7: Constant folding - evaluate constant expressions at compile time
    fn generate_expression(&mut self, expr: &Expression) -> String {
        // PHASE 7: Try constant folding first
        let folded_expr = constant_folding::try_fold_constant(expr);
        let expr_to_generate = folded_expr.as_ref().unwrap_or(expr);

        match expr_to_generate {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => {
                // Qualified paths use :: from parser (e.g., std::fs::read)
                // Simple identifiers: variable_name -> variable_name
                // Check if this is a struct field and we're in an impl block
                // BUT: Don't apply implicit field access if:
                // 1. It's a parameter name (parameters shadow fields)
                // 2. It's a local variable (local vars shadow fields)
                let is_parameter = self.current_function_params.iter().any(|p| p.name == *name);
                let is_local_variable = self
                    .local_variable_scopes
                    .iter()
                    .any(|scope| scope.contains(name));

                let base_name = if self.in_impl_block
                    && !is_parameter
                    && !is_local_variable  // NEW: Local variables shadow fields!
                    && self.current_struct_fields.contains(name)
                {
                    format!("self.{}", name)
                } else {
                    name.clone()
                };

                // AUTO-CLONE: Check if this variable needs to be cloned at this point
                if let Some(ref analysis) = self.auto_clone_analysis {
                    if analysis
                        .needs_clone(name, self.current_statement_idx)
                        .is_some()
                    {
                        // Skip .clone() for Copy types
                        // - String literal variables (they're &str, clone is a no-op)
                        // - usize variables (Copy type, no need to clone)
                        // - Variables with names suggesting Copy types
                        let is_copy_type = analysis.string_literal_vars.contains(name)
                            || self.usize_variables.contains(name)
                            || name.contains("usize")  // sparse_idx_usize, etc.
                            || name.contains("index")  // index, idx, etc.
                            || matches!(name.as_str(), "i" | "j" | "k" | "idx" | "pos");

                        if !is_copy_type {
                            // Automatically insert .clone() - this is the magic!
                            return format!("{}.clone()", base_name);
                        }
                    }
                }

                base_name
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                // Special handling for string concatenation
                if matches!(op, BinaryOp::Add) {
                    // Only treat as string concat if at least one operand is definitely a string literal
                    let has_string_literal = matches!(
                        left.as_ref(),
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || matches!(
                        right.as_ref(),
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || string_analysis::contains_string_literal(left)
                        || string_analysis::contains_string_literal(right);

                    if has_string_literal {
                        // For string concatenation, use format! macro for clean, efficient code
                        return self.generate_string_concat(left, right);
                    }
                }

                // Check for usize/i32 comparison or arithmetic - cast if needed
                let is_comparison = matches!(
                    op,
                    BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                        | BinaryOp::Eq
                        | BinaryOp::Ne
                );
                let is_arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                );
                let left_is_usize = self.expression_produces_usize(left);
                let right_is_usize = self.expression_produces_usize(right);
                let right_is_int_literal = matches!(
                    right.as_ref(),
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );
                let left_is_int_literal = matches!(
                    left.as_ref(),
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );

                // Wrap operands in parens if they have lower precedence
                let mut left_str = match left.as_ref() {
                    Expression::Binary { op: left_op, .. } => {
                        if operators::op_precedence(left_op) < operators::op_precedence(op) {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left),
                };
                let mut right_str = match right.as_ref() {
                    Expression::Binary { op: right_op, .. } => {
                        if operators::op_precedence(right_op) < operators::op_precedence(op) {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right),
                };

                // WINDJAMMER PHILOSOPHY: Auto-cast int/usize in comparisons
                // When comparing int (i64) with usize, automatically cast to make it work.
                //
                // SMART CASTING STRATEGY:
                // - If comparing .len() with int literal: cast literal to usize (keeps .len() clean)
                // - If comparing .len() with int variable: cast .len() to i64 (int context)
                //
                // Example: items.len() >= 10 → items.len() >= 10usize
                // Example: index >= items.len() → index >= items.len() as i64
                //
                // IMPORTANT: Only cast when there's a MISMATCH (one is usize, one is not)
                // If BOTH are usize, no cast needed!
                if is_comparison && left_is_usize && !right_is_usize {
                    // Left is usize, right is NOT usize
                    // Prefer casting literals to usize instead of casting .len() to i64
                    if right_is_int_literal {
                        right_str = format!("({} as usize)", right_str);
                    } else {
                        left_str = format!("({} as i64)", left_str);
                    }
                } else if is_comparison && right_is_usize && !left_is_usize {
                    // Right is usize, left is NOT usize
                    // Prefer casting literals to usize instead of casting .len() to i64
                    if left_is_int_literal {
                        left_str = format!("({} as usize)", left_str);
                    } else {
                        right_str = format!("({} as i64)", right_str);
                    }
                }
                // If both are usize: no cast (usize == usize is fine)
                // If neither is usize: no cast (i64 == i64 is fine)

                // AUTO-CAST: When doing arithmetic between usize and int literal, cast literal to usize
                // E.g., items.len() - 1 -> items.len() - 1usize
                if is_arithmetic && left_is_usize && right_is_int_literal && !right_is_usize {
                    right_str = format!("({} as usize)", right_str);
                } else if is_arithmetic && right_is_usize && left_is_int_literal && !left_is_usize {
                    left_str = format!("({} as usize)", left_str);
                }

                let op_str = operators::binary_op_to_rust(op);
                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Unary { op, operand, .. } => {
                let operand_str = self.generate_expression(operand);
                let op_str = operators::unary_op_to_rust(op);

                // CRITICAL: Preserve parentheses for binary expressions in unary context
                // !(a || b) should generate !(a || b), not !a || b
                // Binary operators have lower precedence than unary operators, so we need parens
                let needs_parens = matches!(&**operand, Expression::Binary { .. });

                if needs_parens {
                    format!("{}({})", op_str, operand_str)
                } else {
                    format!("{}{}", op_str, operand_str)
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Extract function name for signature lookup
                let func_name = ast_utilities::extract_function_name(function);

                // Special case: convert print/println/eprintln/eprint() to macros
                if func_name == "print"
                    || func_name == "println"
                    || func_name == "eprintln"
                    || func_name == "eprint"
                {
                    let macro_name = func_name.clone();

                    // For print() -> println!(), otherwise keep the same name
                    let target_macro = if macro_name == "print" {
                        "println".to_string()
                    } else {
                        macro_name.clone()
                    };
                    // Check if the first argument is a format! macro (from string interpolation)
                    if let Some((_, first_arg)) = arguments.first() {
                        // Check for MacroInvocation (explicit format! calls)
                        if let Expression::MacroInvocation {
                            name,
                            args: macro_args,
                            ..
                        } = first_arg
                        {
                            if name == "format" && !macro_args.is_empty() {
                                // Unwrap the format! call and put its arguments directly into println!
                                // format!("text {}", var) -> println!("text {}", var)
                                let format_str = self.generate_expression(&macro_args[0]);
                                let format_args: Vec<String> = macro_args[1..]
                                    .iter()
                                    .map(|arg| self.generate_expression(arg))
                                    .collect();

                                let args_str = if format_args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", format_args.join(", "))
                                };

                                return format!("{}!({}{})", target_macro, format_str, args_str);
                            }
                        }

                        // Check for Binary expression with string concatenation (will become format!)
                        if let Expression::Binary {
                            left,
                            op: BinaryOp::Add,
                            right,
                            ..
                        } = first_arg
                        {
                            // Check if this is string concatenation
                            let has_string_literal =
                                matches!(
                                    left.as_ref(),
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || matches!(
                                    right.as_ref(),
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || string_analysis::contains_string_literal(left)
                                    || string_analysis::contains_string_literal(right);

                            if has_string_literal {
                                // Collect all parts of the concatenation
                                let mut parts = Vec::new();
                                string_analysis::collect_concat_parts_static(left, &mut parts);
                                string_analysis::collect_concat_parts_static(right, &mut parts);

                                // Generate format string and arguments
                                let format_str = "{}".repeat(parts.len());
                                let format_args: Vec<String> = parts
                                    .iter()
                                    .map(|expr| self.generate_expression(expr))
                                    .collect();

                                return format!(
                                    "{}!(\"{}\", {})",
                                    target_macro,
                                    format_str,
                                    format_args.join(", ")
                                );
                            }
                        }

                        // Check if the first argument is a string literal with ${} (old-style, shouldn't happen but keep for safety)
                        if let Expression::Literal {
                            value: Literal::String(s),
                            ..
                        } = first_arg
                        {
                            if s.contains("${") {
                                // Handle string interpolation directly in println!
                                // Convert "${var}" to "{}" and extract variables
                                let mut format_str = String::new();
                                let mut args = Vec::new();
                                let mut chars = s.chars().peekable();

                                while let Some(ch) = chars.next() {
                                    if ch == '$' && chars.peek() == Some(&'{') {
                                        chars.next(); // consume {
                                        let mut var_name = String::new();

                                        while let Some(&next_ch) = chars.peek() {
                                            if next_ch == '}' {
                                                chars.next(); // consume }
                                                break;
                                            } else {
                                                var_name.push(next_ch);
                                                chars.next();
                                            }
                                        }

                                        if !var_name.is_empty() {
                                            format_str.push_str("{}");
                                            // Check if this is a struct field
                                            if self.in_impl_block
                                                && self.current_struct_fields.contains(&var_name)
                                            {
                                                args.push(format!("self.{}", var_name));
                                            } else {
                                                args.push(var_name);
                                            }
                                        }
                                    } else {
                                        format_str.push(ch);
                                    }
                                }

                                let args_str = if args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", args.join(", "))
                                };

                                return format!(
                                    "{}!(\"{}\"{})",
                                    target_macro,
                                    format_str.replace('\\', "\\\\").replace('"', "\\\""),
                                    args_str
                                );
                            }
                        }
                    }

                    // No interpolation, just regular print
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("{}!({})", target_macro, args.join(", "));
                }

                // Special case: convert assert() to assert!()
                if func_name == "assert" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("assert!({})", args.join(", "));
                }

                let func_str = self.generate_expression(function);

                // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
                // Some("literal") -> Some("literal".to_string())
                // Ok("literal") -> Ok("literal".to_string())
                // Err("literal") -> Err("literal".to_string())
                // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())
                if matches!(func_name.as_str(), "Some" | "Ok" | "Err") {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| {
                            let arg_str = self.generate_expression(arg);
                            // Auto-convert string literals to String for Option/Result wrappers
                            if matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) {
                                format!("{}.to_string()", arg_str)
                            } else if let Expression::Identifier { name, .. } = arg {
                                // AUTO-CLONE: When wrapping a borrowed iterator variable in Some/Ok/Err,
                                // we need to clone it since the wrapper takes ownership
                                if self.borrowed_iterator_vars.contains(name)
                                    && !arg_str.ends_with(".clone()")
                                {
                                    format!("{}.clone()", arg_str)
                                } else {
                                    arg_str
                                }
                            } else {
                                arg_str
                            }
                        })
                        .collect();
                    return format!("{}({})", func_str, args.join(", "));
                }

                // Look up signature and clone it to avoid borrow conflicts
                // THE WINDJAMMER WAY: Try qualified name first, then simple name
                // e.g., "Sound::new" -> try "Sound::new", then "new"
                let signature = self
                    .signature_registry
                    .get_signature(&func_name)
                    .cloned()
                    .or_else(|| {
                        // If qualified lookup fails, try simple name (just the method)
                        if let Some(pos) = func_name.rfind("::") {
                            let simple_name = &func_name[pos + 2..];
                            self.signature_registry.get_signature(simple_name).cloned()
                        } else {
                            None
                        }
                    });

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        let mut arg_str = self.generate_expression(arg);

                        // Auto-convert string literals to String for functions expecting owned String
                        // THE WINDJAMMER WAY: Smart inference based on available information!
                        if matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            // Check if the parameter expects an owned String
                            let should_convert = if let Some(ref sig) = signature {
                                if let Some(&ownership) = sig.param_ownership.get(i) {
                                    // Convert if parameter expects owned String
                                    matches!(ownership, OwnershipMode::Owned)
                                } else {
                                    // No ownership info for this param
                                    // THE WINDJAMMER WAY: Heuristic for constructors
                                    // Functions named 'new' (or Type::new) taking string params likely expect String
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            } else {
                                // No signature found
                                // THE WINDJAMMER WAY: Heuristic for constructors
                                // Functions named 'new' (or Type::new) taking string params likely expect String
                                func_name == "new" || func_name.ends_with("::new")
                            };

                            if should_convert {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }

                        // Check if this parameter expects a borrow
                        if let Some(ref sig) = signature {
                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed => {
                                        // String literals are ALREADY &str - don't add &!
                                        let is_string_literal = matches!(
                                            arg,
                                            Expression::Literal {
                                                value: Literal::String(_),
                                                ..
                                            }
                                        );

                                        // Insert & if not already a reference and not a string literal
                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_string_literal
                                        {
                                            return format!("&{}", arg_str);
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => {
                                        // Insert &mut if not already a reference
                                        if !expression_helpers::is_reference_expression(arg) {
                                            // CRITICAL FIX: Remove .clone() if present - we want to mutate the original!
                                            // &mut counter.clone() → &mut counter
                                            // When passing &mut, we're giving mutable access to the original,
                                            // not a clone. The .clone() would break mutation semantics.
                                            let mut_arg_str = if arg_str.ends_with(".clone()") {
                                                arg_str[..arg_str.len() - 8].to_string()
                                            } else {
                                                arg_str
                                            };
                                            return format!("&mut {}", mut_arg_str);
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // AUTO-CLONE: When passing a field from a borrowed parameter
                                        // to a function that expects an owned value, clone it
                                        if let Expression::FieldAccess {
                                            object: field_obj, ..
                                        } = arg
                                        {
                                            if let Expression::Identifier { name, .. } =
                                                &**field_obj
                                            {
                                                // Check if it's a borrowed parameter (explicit OR inferred)
                                                let is_explicitly_borrowed =
                                                    self.current_function_params.iter().any(|p| {
                                                        &p.name == name
                                                            && matches!(
                                                                p.ownership,
                                                                crate::parser::OwnershipHint::Ref
                                                            )
                                                    });
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(name);
                                                if (is_explicitly_borrowed || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // No signature found - still check for borrowed param field access
                            // This handles qualified calls like Type::method(param.field)
                            if let Expression::FieldAccess {
                                object: field_obj, ..
                            } = arg
                            {
                                if let Expression::Identifier { name, .. } = &**field_obj {
                                    let is_explicitly_borrowed =
                                        self.current_function_params.iter().any(|p| {
                                            &p.name == name
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
                                        });
                                    let is_inferred_borrowed =
                                        self.inferred_borrowed_params.contains(name);
                                    if (is_explicitly_borrowed || is_inferred_borrowed)
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                }
                            }
                        }

                        arg_str
                    })
                    .collect();

                // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
                // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
                let call_str = format!("{}({})", func_str, args.join(", "));

                // Check if this is an extern function call
                let is_extern_call = if let Some(ref sig) = signature {
                    sig.is_extern
                } else {
                    false
                };

                if is_extern_call {
                    format!("unsafe {{ {} }}", call_str)
                } else {
                    call_str
                }
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => {
                let obj_str = self.generate_expression_with_precedence(object);

                // Look up method signature for auto-ref
                let method_signature = self.signature_registry.get_signature(method).cloned();

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        let mut arg_str = self.generate_expression(arg);

                        // AUTO .to_string(): Convert string literals when parameter expects owned String
                        if matches!(arg, Expression::Literal { value: Literal::String(_), .. })
                            && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, &method_signature) {
                            arg_str = format!("{}.to_string()", arg_str);
                        }

                        // AUTO .clone(): Add .clone() when needed for borrowed values
                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_clone(
                            arg,
                            &arg_str,
                            method,
                            i,
                            &method_signature,
                            &self.borrowed_iterator_vars,
                            &self.current_function_params,
                            &self.inferred_borrowed_params,
                        ) {
                            arg_str = format!("{}.clone()", arg_str);
                        }

                        // AUTO-REF: Add & when parameter expects reference but arg is owned
                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                            arg,
                            &arg_str,
                            method,
                            i,
                            &method_signature,
                            &self.usize_variables,
                            &self.current_function_params,
                        ) {
                            arg_str = format!("&{}", arg_str);
                        }

                        // AUTO-BORROW for push_str: String::push_str expects &str, not String
                        // If arg is a String variable/expression (not a string literal), add &
                        if method == "push_str" && i == 0 {
                            let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                            // If not a string literal and not already a reference, add &
                            if !is_string_literal && !arg_str.starts_with('&') {
                                // Check if it's a String-producing expression (variable, field access, method call)
                                let needs_borrow = matches!(arg,
                                    Expression::Identifier { .. } |
                                    Expression::FieldAccess { .. } |
                                    Expression::MethodCall { .. }
                                );
                                if needs_borrow {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        arg_str
                    })
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

                // Special case: substring(start, end) -> &text[start..end]
                if method == "substring" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // Special case: contains() with String argument needs .as_str()
                // String::contains() expects &str, not String
                if method == "contains" && args.len() == 1 {
                    // Check if argument is a method call that returns String (like to_lowercase())
                    if let Some((_label, arg)) = arguments.first() {
                        if matches!(arg, Expression::MethodCall { method: m, .. } if
                            m == "to_lowercase" || m == "to_uppercase" ||
                            m == "to_string" || m == "trim" || m == "clone")
                        {
                            // The argument is String, needs .as_str()
                            return format!("{}.{}({}.as_str())", obj_str, method, args[0]);
                        }
                    }
                }

                // Determine separator: :: for static calls, . for instance methods
                // - Type/Module (starts with uppercase): use ::
                // - Variable (starts with lowercase): use .
                let separator = match &**object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier { name, .. } => {
                        // Check for known module/crate names that should use ::
                        // Note: Avoid common variable names like "path", "config" which are used as variables
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
                            // Stdlib modules (avoid common variable names)
                            "mime",
                            "http",
                            "fs",
                            "strings",
                            "json",
                            "regex",
                            "cli",
                            "log",
                            "crypto",
                            "io",
                            "env",
                            "time",
                            "sync",
                            "thread",
                            "collections",
                            "cmp",
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
                    Expression::FieldAccess { ref object, .. } => {
                        // Check if this is a module path (e.g., std::fs) or a field access (e.g., self.count)
                        // If the object is an identifier that looks like a module, use ::
                        // Otherwise, use . for instance methods on fields
                        match object.as_ref() {
                            Expression::Identifier { name, .. } => {
                                if name.chars().next().is_some_and(|c| c.is_uppercase())
                                    || name == "std"
                                {
                                    "::" // Module::path::method() -> static method
                                } else {
                                    "." // self.field.method() or variable.field.method() -> instance method
                                }
                            }
                            _ => ".", // Default to instance method
                        }
                    }
                    _ => ".", // Instance method on expressions
                };

                // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
                // Convert it back to proper Rust slice syntax
                // For strings, we need to add & to get &str (a reference)
                if method == "slice" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
                // DISABLED: This optimization was too aggressive and removed needed clones
                // TODO: Make this more conservative - only remove clone when we can prove
                // the value is Copy or when it's the last use
                // if method == "clone" && arguments.is_empty() {
                //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
                //         if self.clone_optimizations.contains(var_name) {
                //             // Skip the .clone(), just return the variable (or borrow if needed)
                //             return obj_str;
                //         }
                //     }
                // }

                // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
                // DISABLED: Too aggressive - needs type checking to determine if parameter expects VNode
                // TODO: Re-enable with proper type checking when VNode type bindings are implemented
                let processed_args = args;

                let base_expr = format!(
                    "{}{}{}{}({})",
                    obj_str,
                    separator,
                    method,
                    turbofish,
                    processed_args.join(", ")
                );

                // AUTO-CLONE: Check if this method call needs to be cloned
                if let Some(path) = ast_utilities::extract_field_access_path(expr) {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                        {
                            // Automatically insert .clone() for method call result!
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }
                base_expr
            }
            Expression::FieldAccess { object, field, .. } => {
                let obj_str = self.generate_expression_with_precedence(object);

                // Determine if this is a module/type path (::) or field access (.)
                // Check the object to decide:
                let separator = match &**object {
                    Expression::Identifier { name, .. }
                        if name.contains("::")
                            || (!name.is_empty()
                                && name.chars().next().unwrap().is_uppercase()) =>
                    {
                        "::" // Module path: std::fs or Type::CONST
                    }
                    Expression::FieldAccess { .. } => {
                        // Check if this is a module path or a field chain
                        // If the object string contains ::, it's a module path
                        if obj_str.contains("::") {
                            "::" // Module path: std::fs::File
                        } else {
                            "." // Field chain: transform.position.x
                        }
                    }
                    _ => ".", // Actual field access (e.g., config.field)
                };

                let base_expr = format!("{}{}{}", obj_str, separator, field);

                // AUTO-CLONE: Check if this field access needs to be cloned
                // Extract the full path (e.g., "config.paths")
                if let Some(path) = ast_utilities::extract_field_access_path(expr) {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                        {
                            // Automatically insert .clone() for field access!
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }

                // BORROWED ITERATOR: If accessing fields through a borrowed iterator variable,
                // we need to clone String fields since we can't move out of a reference
                // BUT: Don't clone for assignment targets (left side of =)
                if !self.generating_assignment_target {
                    if let Expression::Identifier { name: var_name, .. } = &**object {
                        if self.borrowed_iterator_vars.contains(var_name) {
                            // Clone non-Copy fields (String-like names)
                            // Exclude obvious Copy fields
                            // ALSO exclude method names that look like fields (as_str, to_string, etc.)
                            let needs_clone = !matches!(
                                field.as_str(),
                                "len" | "count" | "size" | "index" | "idx" | "i" | "j" | "k" |
                                "x" | "y" | "z" | "w" | "width" | "height" | "depth" |
                                "r" | "g" | "b" | "a" | "left" | "right" | "top" | "bottom" |
                                "min" | "max" | "start" | "end" | "offset" | "scale" |
                                "speed" | "time" | "delta" | "angle" | "radius" | "distance" |
                                "visible" | "enabled" | "active" | "selected" | "focused" |
                                "id" | "type" | "kind" | "priority" | "level" |
                                // Method-like names that should NOT be cloned (they're method calls, not fields)
                                "as_str" | "to_string" | "clone" | "iter" | "iter_mut" | "is_empty"
                            );
                            if needs_clone && !base_expr.ends_with(".clone()") {
                                return format!("{}.clone()", base_expr);
                            }
                        }
                    }
                }

                // NOTE: Auto-clone for self.field is handled at a higher level
                // (in struct literal generation and specific return contexts)
                // Do NOT clone here as it causes issues with .iter() on collections

                base_expr
            }
            Expression::StructLiteral { name, fields, .. } => {
                // PHASE 3 OPTIMIZATION: Check if we have optimization hints for this struct
                let _has_optimization_hint = self.struct_mapping_hints.get(name);

                // Generate field assignments
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(field_name, expr)| {
                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // In Windjammer, `string` type is always owned (maps to Rust String)
                        // So string literals in struct fields should be converted automatically
                        let mut expr_str = self.generate_expression(expr);

                        // Auto-convert string literals to String for struct fields
                        if matches!(
                            expr,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            expr_str = format!("{}.to_string()", expr_str);
                        }

                        // CRITICAL: Auto-clone self.field when constructing struct from borrowed self
                        // Pattern: fn method(&self) -> Self { Self { field: self.field } }
                        // Non-Copy fields from borrowed self need to be cloned
                        if let Expression::FieldAccess { object, .. } = expr {
                            if let Expression::Identifier { name: obj_name, .. } = &**object {
                                if obj_name == "self" && !expr_str.contains(".clone()") {
                                    // Check if current function takes &self (borrowed)
                                    let self_is_borrowed =
                                        self.current_function_params.iter().any(|p| {
                                            p.name == "self"
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
                                        });

                                    if self_is_borrowed {
                                        // Clone the field access since self is borrowed
                                        expr_str = format!("{}.clone()", expr_str);
                                    }
                                }
                            }
                        }

                        // Check for field shorthand: if expr is just the field name, use shorthand
                        if let Expression::Identifier { name: id, .. } = expr {
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
            Expression::MapLiteral { pairs, .. } => {
                // Generate HashMap literal: HashMap::from([(key, value), ...])
                if pairs.is_empty() {
                    "std::collections::HashMap::new()".to_string()
                } else {
                    let entries_str: Vec<String> = pairs
                        .iter()
                        .map(|(k, v)| {
                            let key_str = self.generate_expression(k);
                            let val_str = self.generate_expression(v);
                            format!("({}, {})", key_str, val_str)
                        })
                        .collect();
                    format!(
                        "std::collections::HashMap::from([{}])",
                        entries_str.join(", ")
                    )
                }
            }
            Expression::TryOp { expr: inner, .. } => {
                format!("{}?", self.generate_expression(inner))
            }
            Expression::Await { expr: inner, .. } => {
                format!("{}.await", self.generate_expression(inner))
            }
            Expression::ChannelSend { channel, value, .. } => {
                let ch_str = self.generate_expression(channel);
                let val_str = self.generate_expression(value);
                format!("{}.send({})", ch_str, val_str)
            }
            Expression::ChannelRecv { channel, .. } => {
                let ch_str = self.generate_expression(channel);
                format!("{}.recv()", ch_str)
            }
            Expression::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("{}..={}", start_str, end_str)
                } else {
                    format!("{}..{}", start_str, end_str)
                }
            }
            Expression::Closure {
                parameters, body, ..
            } => {
                let params = parameters.join(", ");
                let body_str = self.generate_expression(body);

                // In Windjammer, closures automatically use move semantics when needed
                // This is more ergonomic than requiring explicit 'move' keyword
                // We always generate 'move' for safety in concurrent contexts
                format!("move |{}| {}", params, body_str)
            }
            Expression::Index { object, index, .. } => {
                let obj_str = self.generate_expression(object);

                // Special case: if index is a Range, this is slice syntax
                // FIXED: Don't add & - Rust will auto-coerce to &[T] when needed
                // This prevents "&temporary" errors when chaining methods like .to_vec()
                if let Expression::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = &**index
                {
                    let start_str = self.generate_expression(start);
                    let end_str = self.generate_expression(end);
                    let range_op = if *inclusive { "..=" } else { ".." };
                    return format!("{}[{}{}{}]", obj_str, start_str, range_op, end_str);
                }

                let idx_str = self.generate_expression(index);

                // WINDJAMMER PHILOSOPHY: Auto-cast to usize for array indexing
                // Rust requires usize for indexing, but Windjammer uses int (i64)
                // Handle cases:
                // 1. Simple identifier: arr[idx] -> arr[idx as usize]
                // 2. Integer literal: arr[0] -> arr[0 as usize]
                // 3. Cast to int/i64: arr[x as int] -> arr[x as usize]
                // 4. Already usize: don't double-cast
                let final_idx = if idx_str.ends_with("as i64") || idx_str.ends_with("as int") {
                    // Replace i64/int cast with usize
                    let base = idx_str
                        .trim_end_matches("as i64")
                        .trim_end_matches("as int")
                        .trim();
                    format!("{} as usize", base)
                } else if matches!(
                    &**index,
                    Expression::Identifier { .. }
                        | Expression::Literal {
                            value: Literal::Int(_),
                            ..
                        }
                ) && !idx_str.contains(" as ")
                {
                    // Simple identifier or integer literal - add usize cast
                    format!("{} as usize", idx_str)
                } else {
                    idx_str
                };

                let base_expr = format!("{}[{}]", obj_str, final_idx);

                // TODO: Vec indexing of non-Copy types causes E0507
                // Need to implement context-aware borrowing:
                // - If used for field access: &vec[idx]
                // - If returned/stored: vec[idx].clone()
                // - If Copy type: vec[idx] (no change)
                //
                // For now, this generates vec[idx] which works for Copy types
                // but fails for non-Copy types. The proper fix requires:
                // 1. Type inference to detect non-Copy types
                // 2. Usage analysis to determine if borrow or clone is needed
                // 3. Context-aware code generation
                //
                // AUTO-CLONE: Check if this index expression needs to be cloned
                if let Some(path) = ast_utilities::extract_field_access_path(expr) {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                        {
                            // Automatically insert .clone() for index expression!
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }
                base_expr
            }
            Expression::Tuple {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("({})", expr_strs.join(", "))
            }
            Expression::Array {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                // FIXED: Generate array literal [...] instead of vec![...]
                // Array literals create stack-allocated arrays: [1, 2, 3]
                // vec! macro creates heap-allocated vectors: vec![1, 2, 3]
                format!("[{}]", expr_strs.join(", "))
            }
            Expression::MacroInvocation {
                name,
                args,
                delimiter,
                ..
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
                    // Special case: if this is println!/eprintln!/print!/eprint! with a single non-literal arg,
                    // wrap it with "{}" to make it valid Rust: println!(var) -> println!("{}", var)
                    // Also wrap format!() calls: println!(format!(...)) -> println!("{}", format!(...))
                    if (name == "println"
                        || name == "eprintln"
                        || name == "print"
                        || name == "eprint")
                        && args.len() == 1
                        && !matches!(
                            &args[0],
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        vec!["\"{}\"".to_string(), self.generate_expression(&args[0])]
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                };

                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };
                format!("{}!{}{}{}", name, open, arg_strs.join(", "), close)
            }
            Expression::Cast { expr, type_, .. } => {
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
            Expression::Block {
                statements: stmts, ..
            } => {
                // Special case: if the block contains only a match statement, generate it as a match expression
                if stmts.len() == 1 {
                    if let Statement::Match { value, arms, .. } = &stmts[0] {
                        let mut output = String::from("match ");

                        // Check if any arm has a string literal pattern
                        // BUT: Don't add .as_str() if the match value is a tuple
                        let has_string_literal = arms
                            .iter()
                            .any(|arm| pattern_analysis::pattern_has_string_literal(&arm.pattern));

                        let is_tuple_match = arms
                            .iter()
                            .any(|arm| matches!(arm.pattern, Pattern::Tuple(_)));

                        // CRITICAL: Check if matching on self.field to avoid partial move
                        let needs_clone_for_match =
                            self.match_needs_clone_for_self_field(value, arms);

                        let value_str = self.generate_expression(value);
                        if has_string_literal && !is_tuple_match {
                            // Add .as_str() if the value doesn't already end with it
                            if !value_str.ends_with(".as_str()") {
                                output.push_str(&format!("{}.as_str()", value_str));
                            } else {
                                output.push_str(&value_str);
                            }
                        } else if needs_clone_for_match && !value_str.ends_with(".clone()") {
                            // Clone the field to avoid partial move from self
                            output.push_str(&format!("{}.clone()", value_str));
                        } else {
                            output.push_str(&value_str);
                        }

                        output.push_str(" {\n");

                        self.indent_level += 1;

                        // WINDJAMMER PHILOSOPHY: Detect if any arm returns String and convert all arms
                        // Check if conversion is needed based on function return type FIRST
                        let needs_string_conversion_from_type =
                            match &self.current_function_return_type {
                                Some(Type::String) => true,
                                Some(Type::Custom(name)) if name == "String" => true,
                                _ => {
                                    // Also check if any arm explicitly produces String
                                    arms.iter().any(|arm| {
                                        string_analysis::expression_produces_string(&arm.body)
                                            || arm_string_analysis::arm_returns_converted_string(
                                                &arm.body,
                                            )
                                    })
                                }
                            };

                        // Set context flag BEFORE generating arms
                        let old_in_match_arm = self.in_match_arm_needing_string;
                        if needs_string_conversion_from_type {
                            self.in_match_arm_needing_string = true;
                        }

                        // Generate all arms with the flag set
                        let arm_strings: Vec<(String, bool)> = arms
                            .iter()
                            .map(|arm| {
                                let body_str = self.generate_expression(&arm.body);
                                let is_string_literal = matches!(
                                    &arm.body,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                );
                                (body_str, is_string_literal)
                            })
                            .collect();

                        // Restore flag
                        self.in_match_arm_needing_string = old_in_match_arm;

                        // For direct string literals, we still need to apply .to_string()
                        // (blocks handle their own conversion via the flag)
                        let any_arm_produces_string = needs_string_conversion_from_type;

                        for (arm, (arm_str, is_string_literal)) in
                            arms.iter().zip(arm_strings.iter())
                        {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_pattern(&arm.pattern));

                            // Add guard if present
                            if let Some(guard) = &arm.guard {
                                output.push_str(" if ");
                                output.push_str(&self.generate_expression(guard));
                            }

                            output.push_str(" => ");

                            // Auto-convert string literals to String when other arms return String
                            if any_arm_produces_string
                                && *is_string_literal
                                && !arm_str.ends_with(".to_string()")
                            {
                                output.push_str(&format!("{}.to_string()", arm_str));
                            } else {
                                output.push_str(arm_str);
                            }
                            output.push_str(",\n");
                        }
                        self.indent_level -= 1;

                        output.push_str(&self.indent());
                        output.push('}');
                        return output;
                    }
                }

                // Regular block - must handle last expression correctly
                let mut output = String::from("{\n");
                self.indent_level += 1;

                let len = stmts.len();
                for (i, stmt) in stmts.iter().enumerate() {
                    let is_last = i == len - 1;
                    if is_last
                        && matches!(
                            stmt,
                            Statement::Expression { .. }
                                | Statement::Thread { .. }
                                | Statement::Async { .. }
                        )
                    {
                        // Last statement is an expression or thread/async block - generate without discard (it's the return value)
                        match stmt {
                            Statement::Expression { expr, .. } => {
                                output.push_str(&self.indent());
                                let mut expr_str = self.generate_expression(expr);

                                // If in a match arm needing string conversion, convert string literals
                                if self.in_match_arm_needing_string {
                                    let is_string_literal = matches!(
                                        expr,
                                        Expression::Literal {
                                            value: Literal::String(_),
                                            ..
                                        }
                                    );
                                    if is_string_literal && !expr_str.ends_with(".to_string()") {
                                        expr_str = format!("{}.to_string()", expr_str);
                                    }
                                }

                                output.push_str(&expr_str);
                                output.push('\n');
                            }
                            Statement::Thread { body, .. } => {
                                // Generate as expression (returns JoinHandle)
                                output.push_str(&self.indent());
                                output.push_str("std::thread::spawn(move || {\n");
                                self.indent_level += 1;
                                for stmt in body {
                                    output.push_str(&self.generate_statement(stmt));
                                }
                                self.indent_level -= 1;
                                output.push_str(&self.indent());
                                output.push_str("})\n");
                            }
                            Statement::Async { body, .. } => {
                                // Generate as expression (returns JoinHandle)
                                output.push_str(&self.indent());
                                output.push_str("tokio::spawn(async move {\n");
                                self.indent_level += 1;
                                for stmt in body {
                                    output.push_str(&self.generate_statement(stmt));
                                }
                                self.indent_level -= 1;
                                output.push_str(&self.indent());
                                output.push_str("})\n");
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        output.push_str(&self.generate_statement(stmt));
                    }
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
                            // Check if it's {variable} pattern or {} placeholder
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
                                // It's a variable interpolation: {count} -> {}, count
                                format_str.push_str("{}");
                                args.push(var_name);
                            } else if is_variable && var_name.is_empty() {
                                // It's an empty placeholder: {} -> keep as-is (format! placeholder)
                                format_str.push_str("{}");
                            } else {
                                // Not a variable, escape the literal brace
                                format_str.push_str("{{");
                                format_str.push_str(&var_name);
                            }
                        } else if ch == '}' {
                            // Escape literal closing brace (not part of a placeholder)
                            format_str.push_str("}}");
                        } else {
                            format_str.push(ch);
                        }
                    }

                    if args.is_empty() {
                        // No interpolation found, just a regular string
                        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
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

                        format!(
                            "format!(\"{}\"{})",
                            format_str.replace('\\', "\\\\").replace('"', "\\\""),
                            formatted_args
                        )
                    }
                } else {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
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

    /// Generate efficient string concatenation using format! macro
    fn generate_string_concat(&mut self, left: &Expression, right: &Expression) -> String {
        // Collect all parts of the concatenation chain
        let mut parts = Vec::new();
        string_analysis::collect_concat_parts_static(left, &mut parts);
        string_analysis::collect_concat_parts_static(right, &mut parts);

        // Generate format! macro call
        let format_str = "{}".repeat(parts.len());

        // Generate expressions for each part
        let mut args = Vec::new();
        for expr in &parts {
            args.push(self.generate_expression(expr));
        }

        format!("format!(\"{}\", {})", format_str, args.join(", "))
    }

    fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        let mut traits = vec!["Debug".to_string(), "Clone".to_string()]; // Always safe to derive

        // Check if all fields are Copy
        if self.all_fields_are_copy(&struct_.fields) {
            traits.push("Copy".to_string());
        }

        // Check if all fields are PartialEq (most types support this)
        if self.all_fields_are_partial_eq(&struct_.fields) {
            traits.push("PartialEq".to_string());

            // Only add Eq if all fields support it (not floats)
            if self.all_fields_are_eq(&struct_.fields) {
                traits.push("Eq".to_string());

                // If Eq, also check for Hash
                if self.all_fields_are_hashable(&struct_.fields) {
                    traits.push("Hash".to_string());
                }
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
            .all(|field| type_analysis::is_copy_type(&field.field_type))
    }

    fn all_fields_are_partial_eq(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_partial_eq_type(&field.field_type))
    }

    fn all_enum_variants_are_partial_eq(&self, variants: &[crate::parser::EnumVariant]) -> bool {
        use crate::parser::EnumVariantData;
        variants.iter().all(|variant| {
            match &variant.data {
                EnumVariantData::Unit => true, // Unit variants always support PartialEq
                EnumVariantData::Tuple(types) => types.iter().all(|ty| self.is_partial_eq_type(ty)),
                EnumVariantData::Struct(fields) => fields
                    .iter()
                    .all(|(_, field_type)| self.is_partial_eq_type(field_type)),
            }
        })
    }

    /// Pre-pass: Collect which custom types (structs/enums) support PartialEq
    /// This enables smart enum derives that only add PartialEq if all variants support it
    fn collect_partial_eq_types(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Struct { decl: s, .. } => {
                    // Check if this struct has @auto or explicitly derives PartialEq
                    let has_auto = s.decorators.iter().any(|d| d.name == "auto");
                    if has_auto {
                        // Check if all fields support PartialEq
                        let all_fields_support_partial_eq = s
                            .fields
                            .iter()
                            .all(|f| self.is_partial_eq_type_recursive(&f.field_type));
                        if all_fields_support_partial_eq {
                            self.partial_eq_types.insert(s.name.clone());
                        }
                    }
                }
                Item::Enum { decl: e, .. } => {
                    // Enums support PartialEq if all variants do
                    if self.all_enum_variants_are_partial_eq_recursive(&e.variants) {
                        self.partial_eq_types.insert(e.name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    /// Recursive check for PartialEq without using the partial_eq_types set (for pre-pass)
    fn is_partial_eq_type_recursive(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "f32"
                        | "f64"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "isize"
                        | "char"
                ) =>
            {
                true
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_partial_eq_type_recursive(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_partial_eq_type_recursive(t)),
            Type::Vec(inner) => self.is_partial_eq_type_recursive(inner),
            Type::Option(inner) => self.is_partial_eq_type_recursive(inner),
            Type::Result(ok, err) => {
                self.is_partial_eq_type_recursive(ok) && self.is_partial_eq_type_recursive(err)
            }
            // For custom types in pre-pass, assume false (we don't know yet)
            _ => false,
        }
    }

    /// Recursive check for enum variants without using partial_eq_types set
    fn all_enum_variants_are_partial_eq_recursive(
        &self,
        variants: &[crate::parser::EnumVariant],
    ) -> bool {
        use crate::parser::EnumVariantData;
        variants.iter().all(|variant| match &variant.data {
            EnumVariantData::Unit => true,
            EnumVariantData::Tuple(types) => {
                types.iter().all(|ty| self.is_partial_eq_type_recursive(ty))
            }
            EnumVariantData::Struct(fields) => fields
                .iter()
                .all(|(_, field_type)| self.is_partial_eq_type_recursive(field_type)),
        })
    }

    fn all_fields_are_eq(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_eq_type(&field.field_type))
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
    #[allow(clippy::only_used_in_recursion)]
    fn is_partial_eq_type(&self, ty: &Type) -> bool {
        // Most types support PartialEq including floats
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            // Handle Rust-style type names that aren't Windjammer keywords
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "f32"
                        | "f64"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "isize"
                        | "char"
                ) =>
            {
                true
            }
            // Check collected custom types from pre-pass
            Type::Custom(name) => self.partial_eq_types.contains(name),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_partial_eq_type(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_partial_eq_type(t)),
            Type::Vec(inner) => self.is_partial_eq_type(inner),
            Type::Option(inner) => self.is_partial_eq_type(inner),
            Type::Result(ok, err) => self.is_partial_eq_type(ok) && self.is_partial_eq_type(err),
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_eq_type(&self, ty: &Type) -> bool {
        // Eq is stricter - floats don't support it (NaN != NaN)
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false, // Floats don't implement Eq
            // Handle Rust-style type names - floats don't support Eq
            Type::Custom(name) if matches!(name.as_str(), "f32" | "f64") => false,
            Type::Custom(name)
                if matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "isize"
                        | "char"
                ) =>
            {
                true
            }
            Type::Reference(inner) | Type::MutableReference(inner) => self.is_eq_type(inner),
            Type::Tuple(types) => types.iter().all(|t| self.is_eq_type(t)),
            Type::Vec(inner) => self.is_eq_type(inner),
            Type::Option(inner) => self.is_eq_type(inner),
            Type::Result(ok, err) => self.is_eq_type(ok) && self.is_eq_type(err),
            _ => false,
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
        let statement_count = ast_utilities::count_statements(&func.body);

        // Inline small functions (< 10 statements)
        if statement_count < 10 {
            return true;
        }

        // Inline trivial single-expression functions
        if statement_count == 1 {
            if let Statement::Return { value: Some(_), .. } = &func.body[0] {
                return true;
            }
            if let Statement::Expression { .. } = &func.body[0] {
                return true;
            }
        }

        // Default: don't inline large functions
        false
    }

    /// Check if we should add & for borrowed iteration in a for loop
    /// Returns true if iterating over a field of a borrowed parameter
    fn should_borrow_for_iteration(&self, iterable: &Expression) -> bool {
        match iterable {
            // Field access on a variable (e.g., game.walls)
            Expression::FieldAccess { object, .. } => {
                // Check if the object is a simple identifier
                if let Expression::Identifier { .. } = &**object {
                    // Check if this is a parameter in the current function
                    // For game decorator functions, the first parameter is always borrowed
                    // For impl methods, self is borrowed
                    // For now, we'll use a simple heuristic: if it's a field access, assume borrowed
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Check if we're iterating over a borrowed collection
    /// (the iterator variable will be a reference, so field access needs .clone())
    fn is_iterating_over_borrowed(&self, iterable: &Expression) -> bool {
        match iterable {
            // Borrowing the collection: &items, &self.items
            Expression::Unary { op, .. } => {
                matches!(op, UnaryOp::Ref | UnaryOp::MutRef)
            }
            // Field access on borrowed self: self.items (when self is &self)
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    // Check if self is borrowed
                    if name == "self" {
                        return self.current_function_params.iter().any(|p| {
                            p.name == "self"
                                && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                        });
                    }
                    // Check if it's a borrowed parameter
                    return self.current_function_params.iter().any(|p| {
                        &p.name == name && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                    });
                }
                false
            }
            // Direct variable that's a borrowed parameter
            Expression::Identifier { name, .. } => self.current_function_params.iter().any(|p| {
                &p.name == name && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
            }),
            _ => false,
        }
    }

    /// Extract the identifier from a pattern (for for-loop variable names)
    /// Check if a loop body modifies a variable
    fn loop_body_modifies_variable(&self, body: &[Statement], var_name: &str) -> bool {
        for stmt in body {
            if self.statement_modifies_variable(stmt, var_name) {
                return true;
            }
        }
        false
    }

    /// Check if a statement modifies a variable
    fn statement_modifies_variable(&self, stmt: &Statement, var_name: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if we're assigning to var_name or var_name.field
                self_analysis::expression_references_variable_or_field(target, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_modifies_variable(s, var_name))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_modifies_variable(s, var_name))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_modifies_variable(s, var_name)),
            _ => false,
        }
    }

    /// Check if we should add &mut for index access on borrowed fields
    /// FIXED: Never add &mut for index access - let auto-clone analysis handle it!
    ///
    /// WINDJAMMER PHILOSOPHY: Compiler does the work automatically
    /// - Copy types (i64, f32, etc.) are automatically copied
    /// - Non-Copy types get .clone() from auto-clone analysis
    /// - Adding &mut breaks Copy types and creates type errors
    fn should_mut_borrow_index_access(&self, _expr: &Expression) -> bool {
        // FIXED: Don't add &mut for index access
        // The auto-clone analysis will add .clone() when needed
        // Copy types will be automatically copied (no .clone() needed)
        false
    }

    fn variable_is_only_field_accessed(&self, var_name: &str) -> bool {
        // DATA FLOW ANALYSIS: Check if a variable is only used for field access
        //
        // Returns true if the variable is ONLY used like:
        // - frame.x
        // - frame.y
        // - frame.field (read-only field access)
        //
        // Returns false if the variable is:
        // - Returned from function: return frame
        // - Passed to functions: process(frame)
        // - Used by itself without field access

        // Bounds check: ensure we don't go out of range
        let next_idx = self.current_statement_idx + 1;
        if next_idx >= self.current_function_body.len() {
            // No statements after this one, variable not used → safe to borrow
            return true;
        }

        // Analyze statements after the current one
        let statements_after_current = &self.current_function_body[next_idx..];

        for stmt in statements_after_current {
            match self.analyze_variable_usage_in_statement(var_name, stmt) {
                VariableUsage::FieldAccessOnly => continue, // OK, keep checking
                VariableUsage::Moved => return false,       // Variable is moved, needs clone
                VariableUsage::NotUsed => continue,         // Not used in this statement
            }
        }

        // If we got here, variable is only used for field access
        true
    }

    fn analyze_variable_usage_in_statement(
        &self,
        var_name: &str,
        stmt: &Statement,
    ) -> VariableUsage {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            } => {
                // Check if the variable is returned
                self.analyze_variable_usage_in_expression(var_name, expr)
            }
            Statement::Expression { expr, .. } => {
                self.analyze_variable_usage_in_expression(var_name, expr)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // Check condition
                let cond_usage = self.analyze_variable_usage_in_expression(var_name, condition);
                if matches!(cond_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }

                // Check branches
                for s in then_block {
                    let usage = self.analyze_variable_usage_in_statement(var_name, s);
                    if matches!(usage, VariableUsage::Moved) {
                        return VariableUsage::Moved;
                    }
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        let usage = self.analyze_variable_usage_in_statement(var_name, s);
                        if matches!(usage, VariableUsage::Moved) {
                            return VariableUsage::Moved;
                        }
                    }
                }
                cond_usage
            }
            _ => VariableUsage::NotUsed,
        }
    }

    fn analyze_variable_usage_in_expression(
        &self,
        var_name: &str,
        expr: &Expression,
    ) -> VariableUsage {
        match expr {
            Expression::Identifier { name, .. } if name == var_name => {
                // Variable used by itself (not field access) → moved
                VariableUsage::Moved
            }
            Expression::FieldAccess { object, .. } => {
                // Check if it's our variable
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        // Variable used for field access → OK
                        return VariableUsage::FieldAccessOnly;
                    }
                }
                VariableUsage::NotUsed
            }
            Expression::Call { arguments, .. } => {
                // Check if variable is passed as argument
                for (_, arg) in arguments {
                    if let Expression::Identifier { name, .. } = arg {
                        if name == var_name {
                            // Variable passed to function → moved
                            return VariableUsage::Moved;
                        }
                    }
                    // Check for field access in arguments
                    if let Expression::FieldAccess { object, .. } = arg {
                        if let Expression::Identifier { name, .. } = &**object {
                            if name == var_name {
                                // Field access passed to function → OK
                                return VariableUsage::FieldAccessOnly;
                            }
                        }
                    }
                }
                VariableUsage::NotUsed
            }
            Expression::Binary { left, right, .. } => {
                // Check both sides
                let left_usage = self.analyze_variable_usage_in_expression(var_name, left);
                if matches!(left_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                let right_usage = self.analyze_variable_usage_in_expression(var_name, right);
                if matches!(right_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }

                // Return the most restrictive usage
                match (left_usage, right_usage) {
                    (VariableUsage::FieldAccessOnly, _) => VariableUsage::FieldAccessOnly,
                    (_, VariableUsage::FieldAccessOnly) => VariableUsage::FieldAccessOnly,
                    _ => VariableUsage::NotUsed,
                }
            }
            _ => VariableUsage::NotUsed,
        }
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
    /// Generate automatic trait implementation for @component decorator
    fn generate_component_impl(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // For now, generate a marker comment
        // In future iterations, we'll generate actual trait implementations
        output.push_str(&format!(
            "// Component trait implementation for {}\n// TODO: Implement Component trait",
            s.name
        ));

        output
    }

    /// Generate automatic trait implementation for @game decorator
    fn generate_game_impl(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Generate Default implementation for game state
        // All fields are initialized to their default values (0, 0.0, false, etc.)
        output.push_str(&format!("impl Default for {} {{\n", s.name));
        output.push_str("    fn default() -> Self {\n");
        output.push_str(&format!("        {} {{\n", s.name));

        for field in &s.fields {
            let default_value = match &field.field_type {
                Type::Int | Type::Int32 | Type::Uint => "0",
                Type::Float => "0.0",
                Type::Bool => "false",
                Type::String => "String::new()",
                Type::Vec(_) => "Vec::new()",
                Type::Custom(name) if name == "String" => "String::new()",
                Type::Custom(name) if name == "Vec3" => "Vec3::new(0.0, 0.0, 0.0)",
                Type::Custom(name) if name.starts_with("Vec") => "Vec::new()",
                _ => "Default::default()",
            };
            output.push_str(&format!("            {}: {},\n", field.name, default_value));
        }

        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push('}');

        output
    }
}
