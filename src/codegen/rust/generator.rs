#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Rust code generator
use crate::analyzer::*;
use crate::codegen::rust::{
    arm_string_analysis, ast_utilities, codegen_helpers, constant_folding, expression_helpers,
    operators, pattern_analysis, self_analysis, string_analysis, type_analysis,
};
use crate::parser::ast::CompoundOp;
use crate::parser::*;
use crate::CompilationTarget;

pub struct CodeGenerator<'ast> {
    pub(crate) indent_level: usize,
    pub(crate) signature_registry: SignatureRegistry,
    pub(crate) in_wasm_bindgen_impl: bool,
    pub(crate) in_trait_impl: bool, // true if currently generating code for a trait implementation
    needs_wasm_imports: bool,
    needs_web_imports: bool,
    needs_js_imports: bool,
    needs_serde_imports: bool,   // For JSON support
    needs_write_import: bool,    // For string capacity optimization (write! macro)
    needs_smallvec_import: bool, // For Phase 8 SmallVec optimization
    needs_cow_import: bool,      // For Phase 9 Cow optimization
    needs_hashmap_import: bool,  // Auto-detect HashMap usage
    needs_hashset_import: bool,  // Auto-detect HashSet usage
    target: CompilationTarget,
    pub(crate) is_module: bool, // true if generating code for a reusable module (not main file)
    source_map: crate::source_map::SourceMap,
    pub(crate) current_output_file: std::path::PathBuf, // Path to the Rust file being generated
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
    pub(crate) current_statement_idx: usize,
    // IMPLICIT SELF SUPPORT: Track struct fields for implicit self references
    pub(crate) current_struct_fields: std::collections::HashSet<String>, // Field names in current impl block
    pub(crate) current_struct_name: Option<String>, // Name of struct in current impl block
    current_impl_methods: std::collections::HashSet<String>, // Method names in current impl block
    pub(crate) current_impl_instance_methods: std::collections::HashSet<String>, // Methods that take self
    pub(crate) in_impl_block: bool,                 // true if currently generating code for an impl block
    // USIZE DETECTION: Track which struct fields have type usize (for auto-casting)
    pub(crate) usize_struct_fields: std::collections::HashMap<String, std::collections::HashSet<String>>, // Struct name -> usize field names
    // METHOD RETURN TYPES: Track which methods return usize (for auto-casting in comparisons)
    // Maps method name -> return type. Used by infer_expression_type for MethodCall.
    pub(crate) method_return_types: std::collections::HashMap<String, Type>,
    // FUNCTION CONTEXT: Track current function parameters for compound assignment optimization
    pub(crate) current_function_params: Vec<crate::parser::Parameter<'ast>>, // Parameters of the current function
    // FUNCTION CONTEXT: Track current function return type for string literal conversion
    current_function_return_type: Option<Type>,
    // WINDJAMMER TRAIT INFERENCE: Analyzed trait methods with inferred signatures from ALL impls
    pub(crate) analyzed_trait_methods: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::analyzer::AnalyzedFunction<'ast>>,
    >,
    // FUNCTION CONTEXT: Track current function body for data flow analysis
    pub(crate) current_function_body: Vec<&'ast Statement<'ast>>, // Body of the current function being generated
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
    // TDD: Track if we're generating the top-level function body (enables return optimization)
    in_function_body: bool,
    // TDD: Track if the current statement being generated is the last in its block
    current_is_last_statement: bool,
    // TRAIT TRACKING: Track which custom types support PartialEq
    pub(crate) partial_eq_types: std::collections::HashSet<String>,
    // MATCH ARM CONTEXT: Force string conversion in match arm blocks
    in_match_arm_needing_string: bool,
    // MATCH STATEMENT CONTEXT: Track if we're in a match used as a statement (not expression)
    // In statement-context matches, arm blocks should have semicolons on all statements
    in_statement_match: bool,
    // FOR-LOOP AUTO-BORROW: Track local variables that need `&` in for-loops
    // because they are used after the loop (pre-computed per function body)
    pub(crate) for_loop_borrow_needed: std::collections::HashSet<String>,
    // BORROWED ITERATOR VARIABLES: Track variables that are iterating over borrowed collections
    // These variables are references, so accessing their fields requires .clone()
    borrowed_iterator_vars: std::collections::HashSet<String>,
    // OWNED STRING ITERATOR VARIABLES: Track variables from for-loops over Vec<String>
    // These need to be borrowed when used in String += operations
    owned_string_iterator_vars: std::collections::HashSet<String>,
    // USIZE VARIABLES: Track variables assigned from .len() for auto-casting
    pub(crate) usize_variables: std::collections::HashSet<String>,
    // UNUSED LET BINDINGS: Track let bindings whose variable is never used after declaration.
    // Keyed by (line, column) of the let statement's source location.
    // These will be prefixed with `_` in the generated Rust to suppress "unused variable" warnings.
    unused_let_bindings: std::collections::HashSet<(usize, usize)>,
    // INFERRED BORROWED PARAMS: Parameters inferred to be borrowed (for field access cloning)
    pub(crate) inferred_borrowed_params: std::collections::HashSet<String>,
    // INFERRED MUT BORROWED PARAMS: Parameters inferred to be &mut (for avoiding double &mut in passthrough)
    inferred_mut_borrowed_params: std::collections::HashSet<String>,
    // ASSIGNMENT TARGET: Flag to suppress auto-clone when generating assignment targets
    generating_assignment_target: bool,
    // VOID BLOCK: When true, last expression in a block gets a semicolon (if-without-else bodies)
    in_void_block: bool,
    // EXPLICIT CLONE SUPPRESSION: When the source has `.clone()` (MethodCall with method "clone"),
    // suppress auto-clone on the object expression to prevent double .clone().clone()
    in_explicit_clone_call: bool,
    // FIELD CHAIN OPTIMIZATION: When accessing a Copy sub-field (e.g., .y on Vec2),
    // suppress borrowed-iterator cloning on the intermediate object.
    // e.g., enemy.velocity.y → no need to clone velocity just to read .y
    suppress_borrowed_clone: bool,
    // TDD FIX: When true, suppress .clone() for borrowed iterator field access in call arguments
    // The Call handler will add .clone() or & based on parameter ownership signature
    in_call_argument_generation: bool,
    // VEC INDEX CONTEXT: When generating the object of a FieldAccess, suppress Vec index
    // auto-clone since Rust allows field access on &T returned by Vec indexing.
    // e.g., players[i].score → no clone needed, just accesses the field through the ref.
    in_field_access_object: bool,
    // BORROW CONTEXT: When generating the operand of & or &mut, suppress Vec index
    // auto-clone since we want a reference to the original, not a reference to a clone.
    // e.g., &self.items[i] → reference to element, NOT &self.items[i].clone()
    in_borrow_context: bool,
    // RECURSION GUARD: Track traits currently being generated to prevent infinite recursion
    pub(crate) generating_traits: std::collections::HashSet<String>,
    // RECURSION DEPTH: Track recursion depth to prevent stack overflow
    recursion_depth: usize,
    // LOCAL VARIABLE TYPE TRACKING: Map variable names to their inferred types
    // Populated from struct literals (let x = Foo { .. }), type annotations (let x: Foo = ..),
    // and match-bound patterns (Some(x) from Option<Foo> → x: Foo).
    // Enables qualified method signature lookup for local variables (e.g., x.method() → Foo::method)
    pub(crate) local_var_types: std::collections::HashMap<String, Type>,
    // STRUCT FIELD TYPE TRACKING: Map struct names to their field types
    // Enables type inference for field accesses (e.g., self.transforms → ComponentArray<T>)
    pub(crate) struct_field_types: std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
    // USER-DEFINED COPY TYPES: Registry of structs/enums with @derive(Copy)
    // Enables is_copy_type to recognize types like VoxelType as Copy, preventing unnecessary .clone()
    pub(crate) copy_types_registry: std::collections::HashSet<String>,
    // STRUCT LITERAL CONTEXT: When generating values for struct literal fields,
    // array literals should use fixed-size [...] syntax instead of vec![...],
    // since struct fields have explicit type annotations (e.g., [f32; 3]).
    in_struct_literal_field: bool,
    // ENUM VARIANT TYPE TRACKING: Map "EnumName::VariantName" to field types
    // Enables string literal to String coercion in enum variant constructors
    pub(crate) enum_variant_types: std::collections::HashMap<String, Vec<Type>>,
}

// RECURSION GUARD MACRO: Check depth before entering recursive functions
const MAX_RECURSION_DEPTH: usize = 500; // Conservative limit to prevent stack overflow

impl<'ast> CodeGenerator<'ast> {
    /// Increment recursion depth and check if we've exceeded the limit
    fn enter_recursion(&mut self, context: &str) -> Result<(), String> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            eprintln!(
                "🚨 RECURSION DEPTH EXCEEDED in {}: {} levels",
                context, self.recursion_depth
            );
            return Err(format!(
                "Maximum recursion depth ({}) exceeded in {}. Possible infinite recursion.",
                MAX_RECURSION_DEPTH, context
            ));
        }
        // CI FIX: Use % instead of is_multiple_of() for Rust <1.83 compatibility
        // is_multiple_of() was added in Rust 1.83 (Dec 26, 2024), but CI runs on stable (1.82)
        #[allow(clippy::manual_is_multiple_of)]
        if self.recursion_depth % 100 == 0 {
            eprintln!(
                "⚠️  High recursion depth in {}: {} levels",
                context, self.recursion_depth
            );
        }
        Ok(())
    }

    /// Decrement recursion depth when exiting a recursive function
    fn exit_recursion(&mut self) {
        if self.recursion_depth > 0 {
            self.recursion_depth -= 1;
        }
    }

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
            needs_hashmap_import: false,
            needs_hashset_import: false,
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
            current_impl_methods: std::collections::HashSet::new(),
            current_impl_instance_methods: std::collections::HashSet::new(),
            in_impl_block: false,
            usize_struct_fields: std::collections::HashMap::new(),
            method_return_types: std::collections::HashMap::new(),
            current_function_params: Vec::new(),
            current_function_return_type: None,
            current_function_body: Vec::new(),
            workspace_root: None,
            suppress_string_conversion: false,
            for_loop_borrow_needed: std::collections::HashSet::new(),
            borrowed_iterator_vars: std::collections::HashSet::new(),
            owned_string_iterator_vars: std::collections::HashSet::new(),
            usize_variables: std::collections::HashSet::new(),
            unused_let_bindings: std::collections::HashSet::new(),
            inferred_borrowed_params: std::collections::HashSet::new(),
            inferred_mut_borrowed_params: std::collections::HashSet::new(),
            generating_assignment_target: false,
            in_void_block: false,
            in_explicit_clone_call: false,
            suppress_borrowed_clone: false,
            in_field_access_object: false,
            in_call_argument_generation: false,
            in_borrow_context: false,
            partial_eq_types: std::collections::HashSet::new(),
            in_match_arm_needing_string: false,
            in_statement_match: false,
            local_variable_scopes: Vec::new(),
            in_expression_context: false,
            in_function_body: false,
            current_is_last_statement: false,
            analyzed_trait_methods: std::collections::HashMap::new(),
            generating_traits: std::collections::HashSet::new(),
            recursion_depth: 0,
            local_var_types: std::collections::HashMap::new(),
            struct_field_types: std::collections::HashMap::new(),
            copy_types_registry: std::collections::HashSet::new(),
            in_struct_literal_field: false,
            enum_variant_types: std::collections::HashMap::new(),
        }
    }

    /// Pre-populate struct field types from cross-module definitions.
    /// This enables type inference for fields on imported structs,
    /// preventing unnecessary .clone() on Copy-type fields.
    pub fn set_global_struct_field_types(
        &mut self,
        field_types: std::collections::HashMap<
            String,
            std::collections::HashMap<String, crate::parser::Type>,
        >,
    ) {
        // Merge global types into local (local takes priority if there's overlap)
        for (struct_name, fields) in field_types {
            self.struct_field_types
                .entry(struct_name)
                .or_default()
                .extend(fields);
        }
    }

    /// Set Copy types registry from the global compiler state.
    /// This enables is_copy_type to recognize user-defined types with @derive(Copy)
    /// (e.g., VoxelType, FaceDirection) in addition to primitive Copy types.
    pub fn set_copy_types_registry(&mut self, registry: std::collections::HashSet<String>) {
        self.copy_types_registry = registry;
    }

    /// Set analyzed trait methods (used for trait signature inference from impls)
    pub fn set_analyzed_trait_methods(
        &mut self,
        methods: std::collections::HashMap<
            String,
            std::collections::HashMap<String, crate::analyzer::AnalyzedFunction<'ast>>,
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

    pub(crate) fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    /// Generate an item inside an inline module
    fn generate_inline_module_item(
        &mut self,
        item: &Item<'ast>,
        analyzed: &[AnalyzedFunction<'ast>],
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
            Item::TypeAlias {
                name,
                target,
                is_pub,
                ..
            } => {
                let pub_prefix = if *is_pub { "pub " } else { "" };
                format!(
                    "{}type {} = {};\n",
                    pub_prefix,
                    name,
                    self.type_to_rust(target)
                )
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

    /// Set whether this generator is producing module code (vs entry point)
    pub fn set_is_module(&mut self, is_module: bool) {
        self.is_module = is_module;
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
    fn generate_statement_tracked(&mut self, stmt: &Statement<'ast>) -> String {
        let code = self.generate_statement(stmt);
        self.track_generated_lines(&code);
        code
    }

    /// Map Windjammer decorators to Rust attributes
    /// This abstraction layer allows us to use semantic Windjammer names
    /// while generating appropriate Rust attributes based on compilation target
    pub(crate) fn map_decorator(&mut self, name: &str) -> String {
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
            ("ignore", _) => "ignore".to_string(),
            ("timeout", _) => {
                // TODO: Timeout requires special body wrapping
                "test".to_string()
            }
            ("bench", _) => {
                // TODO: Benchmark tests use criterion
                "bench".to_string()
            }
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
    fn generate_block(&mut self, stmts: &[&'ast Statement<'ast>]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            // Track current statement index for optimization hints
            self.current_statement_idx = i;

            let is_last = i == len - 1;
            // TDD: Track if this is the last statement (used by If handler)
            self.current_is_last_statement = is_last;
            // TDD FIX: Only optimize return statements in function body (not nested blocks)
            let should_optimize_return =
                self.in_function_body && matches!(stmt, Statement::Return { .. });
            if is_last
                && !self.in_void_block
                && (should_optimize_return
                    || matches!(
                        stmt,
                        Statement::Expression { .. }
                            | Statement::Thread { .. }
                            | Statement::Async { .. }
                    ))
            {
                // Last statement is an expression, thread/async block, or return - generate as implicit return
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
                            // BUT: Skip .clone() for Copy types (f32, i32, bool, etc.)
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
                                            let is_copy = self
                                                .infer_expression_type(expr)
                                                .as_ref()
                                                .is_some_and(|t| self.is_type_copy(t));
                                            if !is_copy {
                                                expr_str = format!("{}.clone()", expr_str);
                                            }
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
                    Statement::Return { value, .. } => {
                        // TDD FIX: Convert explicit return to implicit return when last statement
                        // Avoids Clippy warning: "unneeded `return` statement"
                        if let Some(expr) = value {
                            output.push_str(&self.indent());
                            let mut expr_str = self.generate_expression(expr);

                            // WINDJAMMER PHILOSOPHY: Auto-convert implicit returns when function returns String
                            // Same logic as Statement::Expression implicit returns
                            let returns_string = match &self.current_function_return_type {
                                Some(Type::String) => true,
                                Some(Type::Custom(name)) if name == "String" => true,
                                _ => false,
                            };

                            let in_match_needing_string = self.in_match_arm_needing_string;
                            let expr_uses_as_str = expr_str.contains(".as_str()");
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
                                // BUT: Skip .clone() for Copy types (f32, i32, bool, etc.)
                                else if let Expression::FieldAccess { object, .. } = expr {
                                    if let Expression::Identifier { name: obj_name, .. } = &**object
                                    {
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
                                                let is_copy = self
                                                    .infer_expression_type(expr)
                                                    .as_ref()
                                                    .is_some_and(|t| self.is_type_copy(t));
                                                if !is_copy {
                                                    expr_str = format!("{}.clone()", expr_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // FIXED: Auto-cast usize to i64 for implicit returns
                            // Same logic as Statement::Expression implicit returns
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
                            output.push('\n');
                        }
                        // Void return as last statement is omitted (block returns () implicitly)
                    }
                    _ => unreachable!(),
                }
            } else if !is_last {
                // TDD FIX: Non-last statements in a block ALWAYS need semicolons,
                // even when the block is used in an expression context (e.g., match arm body
                // inside `let _ = match ... { Arm => { expr1; expr2 } }`).
                // Temporarily clear in_expression_context so intermediate expression
                // statements get their semicolons.
                let old_expr_ctx = self.in_expression_context;
                self.in_expression_context = false;
                output.push_str(&self.generate_statement(stmt));
                self.in_expression_context = old_expr_ctx;
            } else {
                // Last statement of a non-Expression type (e.g., Statement::If used as block value):
                // Preserve in_expression_context so inner branches retain correct semicolon behavior
                output.push_str(&self.generate_statement(stmt));
            }
        }
        output
    }

    pub fn generate_program(
        &mut self,
        program: &Program<'ast>,
        analyzed: &[AnalyzedFunction<'ast>],
    ) -> String {
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

        // Track explicitly imported traits to avoid duplication with auto-imports
        let mut explicitly_imported_traits: std::collections::HashSet<String> =
            std::collections::HashSet::new();

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
            if !self.needs_hashmap_import && Self::program_references_collection(program, "HashMap")
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
                    self.current_impl_methods = impl_block.functions.iter()
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
        if self.is_module {
            let has_explicit_glob_imports = imports.lines().any(|line| {
                let trimmed = line.trim();
                trimmed.ends_with("::*;") && !trimmed.starts_with("//")
            });
            if !has_explicit_glob_imports {
                implicit_imports.push_str("#[allow(unused_imports)]\nuse super::*;\n");
            }
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

    // Helper method for expressions that need to be evaluated without &mut self
    pub(crate) fn generate_expression_immut(&self, expr: &Expression) -> String {
        use crate::parser::ast::operators::{BinaryOp, UnaryOp};

        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => name.clone(),
            Expression::Unary { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Not => "!",
                    UnaryOp::Neg => "-",
                    UnaryOp::Ref => "&",
                    UnaryOp::MutRef => "&mut ",
                    UnaryOp::Deref => "*",
                };
                format!("({}{})", op_str, self.generate_expression_immut(operand))
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                let op_str = match op {
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
                    BinaryOp::BitAnd => "&",
                    BinaryOp::BitOr => "|",
                    BinaryOp::BitXor => "^",
                    BinaryOp::Shl => "<<",
                    BinaryOp::Shr => ">>",
                };
                
                // TDD FIX: Generate comparison without adding incorrect dereferences
                // When comparing &String == &String, both sides are already borrowed - no deref needed!
                // Rust's PartialEq trait handles comparisons correctly for references.
                let left_str = self.generate_expression_immut(left);
                let right_str = self.generate_expression_immut(right);
                
                format!(
                    "{} {} {}",
                    left_str,
                    op_str,
                    right_str
                )
            }
            Expression::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.generate_expression_immut(object), field)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj_str = self.generate_expression_immut(object);
                let args_str = arguments
                    .iter()
                    .map(|(_label, arg)| self.generate_expression_immut(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}.{}({})", obj_str, method, args_str)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let func_str = self.generate_expression_immut(function);
                let args_str = arguments
                    .iter()
                    .map(|(_label, arg)| self.generate_expression_immut(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func_str, args_str)
            }
            Expression::Index { object, index, .. } => {
                format!(
                    "{}[{}]",
                    self.generate_expression_immut(object),
                    self.generate_expression_immut(index)
                )
            }
            // For complex expressions, just output a placeholder
            // Decorators are primarily documentation/runtime checks
            _ => "true".to_string(),
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
                self.expression_modifies_self(arm.body)
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

        output.push_str("    pub fn ");
        output.push_str(&func.name);

        // Add type parameters if present
        if !func.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&func.type_params));
            output.push('>');
        }

        output.push('(');

        // Generate parameters
        // WINDJAMMER FFI: Convert `str` parameters to FFI-compatible (*const u8, usize) pairs
        let mut params: Vec<String> = Vec::new();
        for param in &func.parameters {
            if matches!(&param.type_, Type::Custom(name) if name == "str") {
                // FFI: str -> (*const u8, usize)
                params.push(format!("{}_ptr: *const u8", param.name));
                params.push(format!("{}_len: usize", param.name));
            } else {
                params.push(format!("{}: {}", param.name, self.type_to_rust(&param.type_)));
            }
        }

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

    /// Check if function has decorators that need to wrap the function body
    fn has_wrapping_decorator(&self, func: &FunctionDecl<'ast>) -> bool {
        func.decorators.iter().any(|d| {
            matches!(
                d.name.as_str(),
                "timeout" | "bench" | "requires" | "ensures" | "property_test" | "invariant"
            ) || (d.name == "test" && !d.arguments.is_empty())
        })
    }

    /// Generate function with decorator wrapping (timeout, bench, requires, ensures, etc.)
    fn generate_function_with_wrapping(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // Generate doc comment if present
        if let Some(doc_comment) = &func.doc_comment {
            for line in doc_comment.lines() {
                output.push_str(&format!("/// {}\n", line.trim()));
            }
        }

        // Check for @async decorator
        let is_async = func.decorators.iter().any(|d| d.name == "async");
        if is_async && func.name == "main" {
            output.push_str("#[tokio::main]\n");
        }

        // Generate non-wrapping decorators (like @test, @ignore)
        for decorator in &func.decorators {
            if decorator.name == "async" {
                continue;
            }
            if decorator.name == "export" && self.target != CompilationTarget::Wasm {
                continue;
            }
            // Skip wrapping decorators - they'll be handled in the body
            if matches!(
                decorator.name.as_str(),
                "timeout" | "bench" | "requires" | "ensures" | "property_test" | "invariant"
            ) {
                continue;
            }
            // Skip @test with arguments (setup/teardown) - handled in body
            if decorator.name == "test" && !decorator.arguments.is_empty() {
                continue;
            }

            let rust_attr = self.map_decorator(&decorator.name);
            if decorator.arguments.is_empty() {
                output.push_str(&format!("#[{}]\n", rust_attr));
            }
        }

        // Add #[test] attribute for @property_test decorated functions
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");
        if has_property_test {
            output.push_str("#[test]\n");
        }

        // Function signature
        let has_export = func.decorators.iter().any(|d| d.name == "export");
        if !self.in_trait_impl
            && (func.is_pub || self.in_wasm_bindgen_impl || self.is_module || has_export)
        {
            output.push_str("pub ");
        }

        if is_async {
            output.push_str("async ");
        }

        output.push_str("fn ");
        output.push_str(&func.name);
        output.push('(');

        // For @property_test, remove parameters (they become generators)
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");

        // For @test(setup/teardown), remove parameters (they come from setup)
        let has_setup_teardown = func
            .decorators
            .iter()
            .any(|d| d.name == "test" && !d.arguments.is_empty());

        if !has_property_test && !has_setup_teardown {
            // Generate normal parameters
            let params: Vec<String> = func
                .parameters
                .iter()
                .enumerate()
                .map(|(idx, param)| {
                    let param_type = analyzed
                        .inferred_param_types
                        .get(idx)
                        .unwrap_or(&param.type_);
                    let ownership = analyzed
                        .inferred_ownership
                        .get(&param.name)
                        .unwrap_or(&crate::analyzer::OwnershipMode::Owned);
                    let rust_type = self.type_to_rust(param_type);

                    // THE WINDJAMMER WAY: Owned parameters are always mutable
                    match ownership {
                        crate::analyzer::OwnershipMode::Borrowed => {
                            format!("{}: &{}", param.name, rust_type)
                        }
                        crate::analyzer::OwnershipMode::MutBorrowed => {
                            format!("{}: &mut {}", param.name, rust_type)
                        }
                        crate::analyzer::OwnershipMode::Owned => {
                            format!("mut {}: {}", param.name, rust_type)
                        }
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
        }

        output.push(')');

        // Return type (not for @property_test or @test(setup/teardown))
        if !has_property_test && !has_setup_teardown {
            if let Some(return_type) = &func.return_type {
                output.push_str(" -> ");
                output.push_str(&self.type_to_rust(return_type));
            }
        }

        output.push_str(" {\n");
        self.indent_level += 1;

        // Generate wrapped body
        output.push_str(&self.generate_wrapped_function_body(analyzed));

        self.indent_level -= 1;
        output.push_str("}\n\n");

        output
    }

    /// Generate function body with decorator wrapping
    fn generate_wrapped_function_body(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // Collect decorators
        let timeout_decorator = func.decorators.iter().find(|d| d.name == "timeout");
        let bench_decorator = func.decorators.iter().find(|d| d.name == "bench");
        let requires_decorators: Vec<_> = func
            .decorators
            .iter()
            .filter(|d| d.name == "requires")
            .collect();
        let ensures_decorators: Vec<_> = func
            .decorators
            .iter()
            .filter(|d| d.name == "ensures")
            .collect();
        let invariant_decorators: Vec<_> = func
            .decorators
            .iter()
            .filter(|d| d.name == "invariant")
            .collect();
        let property_test_decorator = func.decorators.iter().find(|d| d.name == "property_test");
        let test_decorator = func
            .decorators
            .iter()
            .find(|d| d.name == "test" && !d.arguments.is_empty());

        // Handle @property_test
        if let Some(prop_decorator) = property_test_decorator {
            let iterations = if let Some((_, expr)) = prop_decorator.arguments.first() {
                self.generate_expression_immut(expr)
            } else {
                "100".to_string()
            };

            output.push_str(&self.indent());
            output.push_str(&format!(
                "property_test_with_gen{}({},\n",
                func.parameters.len(),
                iterations
            ));
            self.indent_level += 1;

            // Generate generators for each parameter
            for param in &func.parameters {
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "|| rand::random::<{}>(),\n",
                    self.type_to_rust(&param.type_)
                ));
            }

            // Generate test closure with typed parameters
            output.push_str(&self.indent());
            output.push('|');
            let param_with_types: Vec<String> = func
                .parameters
                .iter()
                .map(|p| format!("{}: {}", p.name, self.type_to_rust(&p.type_)))
                .collect();
            output.push_str(&param_with_types.join(", "));
            output.push_str("| {\n");
            self.indent_level += 1;

            // Generate body
            for stmt in &func.body {
                output.push_str(&self.generate_statement(stmt));
            }

            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}\n");
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str(");\n");

            return output;
        }

        // Handle @test(setup=fn, teardown=fn)
        if let Some(test_dec) = test_decorator {
            let mut setup_fn = None;
            let mut teardown_fn = None;

            for (key, expr) in &test_dec.arguments {
                if key == "setup" {
                    setup_fn = Some(self.generate_expression_immut(expr));
                } else if key == "teardown" {
                    teardown_fn = Some(self.generate_expression_immut(expr));
                }
            }

            output.push_str(&self.indent());
            output.push_str("with_setup_teardown(\n");
            self.indent_level += 1;

            output.push_str(&self.indent());
            output.push_str(&format!(
                "{},\n",
                setup_fn.unwrap_or_else(|| "|| ()".to_string())
            ));
            output.push_str(&self.indent());
            output.push_str(&format!(
                "{},\n",
                teardown_fn.unwrap_or_else(|| "|_| ()".to_string())
            ));

            output.push_str(&self.indent());
            output.push('|');
            if !func.parameters.is_empty() {
                output.push_str(&func.parameters[0].name);
            } else {
                output.push_str("_resource");
            }
            output.push_str("| {\n");
            self.indent_level += 1;

            // Generate body
            for stmt in &func.body {
                output.push_str(&self.generate_statement(stmt));
            }

            // Return the resource
            output.push_str(&self.indent());
            if !func.parameters.is_empty() {
                output.push_str(&func.parameters[0].name);
            } else {
                output.push_str("_resource");
            }
            output.push('\n');

            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}\n");
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str(");\n");

            return output;
        }

        // Start with timeout wrapper if present
        let needs_timeout = timeout_decorator.is_some();
        if needs_timeout {
            let timeout_ms = if let Some((_, expr)) = timeout_decorator.unwrap().arguments.first() {
                self.generate_expression_immut(expr)
            } else {
                "1000".to_string()
            };

            output.push_str(&self.indent());
            output.push_str(&format!(
                "windjammer_runtime::timeout::with_timeout(std::time::Duration::from_millis({}), || {{\n",
                timeout_ms
            ));
            self.indent_level += 1;
        }

        // Start with bench wrapper if present
        let needs_bench = bench_decorator.is_some();
        if needs_bench {
            output.push_str(&self.indent());
            output.push_str("let _bench_result = windjammer_runtime::bench::bench(|| {\n");
            self.indent_level += 1;
        }

        // Add @requires checks (preconditions)
        for req_decorator in requires_decorators {
            if let Some((_, expr)) = req_decorator.arguments.first() {
                let condition = self.generate_expression_immut(expr);
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "windjammer_runtime::test::requires({}, \"{}\");\n",
                    condition, condition
                ));
            }
        }

        // If we have @ensures, wrap body in a block and capture result
        let needs_ensures = !ensures_decorators.is_empty();

        // THE WINDJAMMER WAY: Clone owned parameters that are referenced in @ensures
        // This prevents E0382 errors when parameters are moved in the function body
        if needs_ensures {
            // Collect parameter names referenced in @ensures conditions
            let mut params_in_ensures = std::collections::HashSet::new();
            for ens_decorator in &ensures_decorators {
                if let Some((_, expr)) = ens_decorator.arguments.first() {
                    let condition = self.generate_expression_immut(expr);
                    // Extract parameter names from the condition
                    for param in &func.parameters {
                        if condition.contains(&param.name) {
                            params_in_ensures.insert(param.name.clone());
                        }
                    }
                }
            }

            // Clone owned parameters that appear in @ensures
            for param in &func.parameters {
                if params_in_ensures.contains(&param.name) {
                    let ownership = analyzed
                        .inferred_ownership
                        .get(&param.name)
                        .unwrap_or(&crate::analyzer::OwnershipMode::Owned);

                    // Only clone Owned parameters (borrowed ones can be used multiple times)
                    if matches!(ownership, crate::analyzer::OwnershipMode::Owned) {
                        output.push_str(&self.indent());
                        output.push_str(&format!(
                            "let __{}__for_ensures = {}.clone();\n",
                            param.name, param.name
                        ));
                    }
                }
            }

            output.push_str(&self.indent());
            output.push_str("let __result = {\n");
            self.indent_level += 1;
        }

        // Generate function body
        // THE WINDJAMMER WAY: Treat last expression specially (no semicolon for return value)
        // TDD FIX: Also convert explicit `return expr` to implicit return when last statement
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;

            // If this is the last statement, use implicit return (suppress `return` keyword)
            if is_last
                && matches!(
                    stmt,
                    Statement::Expression { .. } | Statement::Return { .. }
                )
            {
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        output.push_str(&self.generate_expression(expr));
                        output.push('\n');
                    }
                    Statement::Return {
                        value: Some(expr), ..
                    } => {
                        // TDD FIX: Convert explicit `return expr` to implicit return
                        // Generates idiomatic Rust without Clippy warnings
                        output.push_str(&self.indent());
                        output.push_str(&self.generate_expression(expr));
                        output.push('\n');
                    }
                    Statement::Return { value: None, .. } => {
                        // Void return as last statement — omit entirely (function returns () implicitly)
                    }
                    _ => unreachable!(),
                }
            } else {
                // Not last statement — generate normally (early returns keep `return` keyword)
                output.push_str(&self.generate_statement(stmt));
            }
        }

        // Add @invariant checks (after function body)
        for inv_decorator in &invariant_decorators {
            if let Some((_, expr)) = inv_decorator.arguments.first() {
                let condition = self.generate_expression_immut(expr);
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "windjammer_runtime::test::invariant({}, \"{}\");\n",
                    condition, condition
                ));
            }
        }

        // Close @ensures block and add checks
        if needs_ensures {
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("};\n");

            for ens_decorator in ensures_decorators {
                if let Some((_, expr)) = ens_decorator.arguments.first() {
                    let mut condition = self.generate_expression_immut(expr);
                    // Replace 'result' with '__result' in ensures conditions
                    condition = condition.replace("result", "__result");

                    // Replace parameter names with cloned versions
                    // Replace "name" but not ".name" (field access)
                    for param in &func.parameters {
                        let ownership = analyzed
                            .inferred_ownership
                            .get(&param.name)
                            .unwrap_or(&crate::analyzer::OwnershipMode::Owned);

                        if matches!(ownership, crate::analyzer::OwnershipMode::Owned) {
                            // Split condition into tokens and replace standalone param names
                            // Avoid replacing field accesses (e.g. ".name")
                            let tokens: Vec<&str> = condition.split(' ').collect();
                            let mut new_tokens = Vec::new();

                            for (i, token) in tokens.iter().enumerate() {
                                let prev_ends_with_dot = if i > 0 {
                                    tokens[i - 1].ends_with('.')
                                } else {
                                    false
                                };

                                if *token == param.name && !prev_ends_with_dot {
                                    new_tokens.push(format!("__{}__for_ensures", param.name));
                                } else {
                                    new_tokens.push(token.to_string());
                                }
                            }

                            condition = new_tokens.join(" ");
                        }
                    }

                    output.push_str(&self.indent());
                    output.push_str(&format!(
                        "windjammer_runtime::test::ensures({}, \"{}\");\n",
                        condition, condition
                    ));
                }
            }

            output.push_str(&self.indent());
            output.push_str("__result\n");
        }

        // Close bench wrapper
        if needs_bench {
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("});\n");
            output.push_str(&self.indent());
            output.push_str("println!(\"Benchmark: {:?}\", _bench_result);\n");
        }

        // Close timeout wrapper
        if needs_timeout {
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}).unwrap();\n");
        }

        output
    }

    /// Generate multiple test functions from a parameterized test (@test_cases)
    ///
    /// Example Windjammer:
    /// ```text
    /// @test_cases([
    ///     (5, 3, 8),
    ///     (10, -5, 5),
    ///     (0, 0, 0),
    /// ])
    /// fn add_numbers(a: int, b: int, expected: int) {
    ///     assert_eq(a + b, expected);
    /// }
    /// ```
    ///
    /// Generates:
    /// ```text
    /// fn add_numbers_case_0() { add_numbers_impl(5, 3, 8); }
    /// fn add_numbers_case_1() { add_numbers_impl(10, -5, 5); }
    /// fn add_numbers_case_2() { add_numbers_impl(0, 0, 0); }
    /// fn add_numbers_impl(a: i64, b: i64, expected: i64) {
    ///     assert_eq!(a + b, expected);
    /// }
    /// ```
    fn generate_parameterized_tests(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        test_cases_decorator: &Decorator<'ast>,
    ) -> String {
        use crate::parser::Expression;

        let func = &analyzed.decl;
        let mut output = String::new();

        // Extract test cases from decorator arguments
        // Expected format: @test_cases([(val1, val2, ...), (val1, val2, ...), ...])
        let test_cases = if let Some((_, cases_expr)) = test_cases_decorator.arguments.first() {
            // Parse the array literal
            if let Expression::Array { elements, .. } = cases_expr {
                elements.clone()
            } else {
                // Not an array, try to extract it directly
                vec![*cases_expr]
            }
        } else {
            // No arguments provided, skip parameterized test generation
            return "// ERROR: @test_cases decorator requires arguments\n".to_string();
        };

        if test_cases.is_empty() {
            return "// ERROR: @test_cases decorator requires at least one test case\n".to_string();
        }

        // Generate the implementation function (with _impl suffix)
        let impl_func_name = format!("{}_impl", func.name);

        // Create a modified function declaration for the implementation
        let mut impl_func_decl = func.clone();
        impl_func_decl.name = impl_func_name.clone();
        // Remove the @test_cases decorator from the impl function
        impl_func_decl
            .decorators
            .retain(|d| d.name != "test_cases" && d.name != "test");

        // Create a modified AnalyzedFunction for the implementation
        let mut impl_analyzed = analyzed.clone();
        impl_analyzed.decl = impl_func_decl;

        // Generate the implementation function (non-test, just regular function)
        output.push_str(&self.generate_function_impl(&impl_analyzed));
        output.push_str("\n\n");

        // Generate a test function for each test case
        for (case_idx, case_expr) in test_cases.iter().enumerate() {
            output.push_str("#[test]\n");
            output.push_str(&format!("fn {}_case_{}() {{\n", func.name, case_idx));

            // Generate the call to the implementation function with the test case arguments
            output.push_str("    ");
            output.push_str(&impl_func_name);
            output.push('(');

            // Extract arguments from the tuple or array expression
            // THE WINDJAMMER WAY: Support both (val1, val2) and [val1, val2] syntax
            if let Expression::Tuple { elements, .. } = case_expr {
                let args: Vec<String> = elements
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| self.generate_test_case_argument(arg, &func.parameters, idx))
                    .collect();
                output.push_str(&args.join(", "));
            } else if let Expression::Array { elements, .. } = case_expr {
                // Also support array syntax: ["val1", "val2", "val3"]
                let args: Vec<String> = elements
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| self.generate_test_case_argument(arg, &func.parameters, idx))
                    .collect();
                output.push_str(&args.join(", "));
            } else {
                // Single argument (not a tuple or array)
                output.push_str(&self.generate_test_case_argument(case_expr, &func.parameters, 0));
            }

            output.push_str(");\n");
            output.push_str("}\n\n");
        }

        output
    }

    /// Generate a test case argument with auto-conversion for string literals
    /// THE WINDJAMMER WAY: Compiler does the hard work, not the developer
    fn generate_test_case_argument(
        &mut self,
        arg_expr: &Expression<'ast>,
        params: &[Parameter<'ast>],
        param_idx: usize,
    ) -> String {
        use crate::parser::ast::core::Expression;
        use crate::parser::ast::literals::Literal;
        use crate::parser::ast::types::Type;

        // Get the expected parameter type
        let param_type = params.get(param_idx).map(|p| &p.type_);

        // Check if this is a string literal and the parameter expects String
        let needs_to_string = if let Expression::Literal {
            value: Literal::String(_),
            ..
        } = arg_expr
        {
            // Check if the parameter type is String
            if let Some(param_type) = param_type {
                match param_type {
                    Type::String => true,
                    Type::Custom(name) if name == "string" => true,
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        };

        // Generate the expression
        let mut result = self.generate_expression_immut(arg_expr);

        // Add .to_string() if needed
        if needs_to_string {
            result.push_str(".to_string()");
        }

        result
    }

    /// Generate a function without test decorators (used by parameterized tests)
    fn generate_function_impl(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        // Just call the regular generate_function since we've already removed the decorators
        self.generate_function(analyzed)
    }

    pub(crate) fn generate_function(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        let func = &analyzed.decl;

        // PARAMETERIZED TESTS: Check for @test_cases decorator
        // If present, generate multiple test functions instead of one
        if let Some(test_cases_decorator) = func.decorators.iter().find(|d| d.name == "test_cases")
        {
            return self.generate_parameterized_tests(analyzed, test_cases_decorator);
        }

        // TESTING DECORATORS: Check for decorators that need to wrap the function body
        // These include: @timeout, @bench, @requires, @ensures, @property_test, @test(setup/teardown)
        if self.has_wrapping_decorator(func) {
            return self.generate_function_with_wrapping(analyzed);
        }

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

        // Clear local variable types for new function scope
        self.local_var_types.clear();

        // Track function return type for string literal conversion
        self.current_function_return_type = func.return_type.clone();

        // Track method return types for usize inference in comparisons
        // When in an impl block, record the return type so expression_produces_usize
        // can resolve method calls like animation.frame_count() → usize
        if self.in_impl_block {
            if let Some(ref ret_type) = func.return_type {
                self.method_return_types
                    .insert(func.name.to_string(), ret_type.clone());
            }
        }

        // Track function body for data flow analysis
        self.current_function_body = func.body.clone();

        // FOR-LOOP AUTO-BORROW: Pre-scan function body to find local variables
        // that are iterated in for-loops and also used after the loop.
        // These need `&` auto-inserted to prevent consuming the collection.
        self.precompute_for_loop_borrows(&func.body);

        // Track parameters inferred as borrowed/mut-borrowed for codegen decisions
        self.inferred_borrowed_params.clear();
        self.inferred_mut_borrowed_params.clear();
        for (param_name, ownership) in &analyzed.inferred_ownership {
            match ownership {
                crate::analyzer::OwnershipMode::Borrowed => {
                    self.inferred_borrowed_params.insert(param_name.clone());
                }
                crate::analyzer::OwnershipMode::MutBorrowed => {
                    self.inferred_mut_borrowed_params.insert(param_name.clone());
                }
                _ => {}
            }
        }

        // WINDJAMMER FIX: Track usize-typed parameters for auto-cast logic
        // DON'T clear here - we need to accumulate variables from let statements during generation!
        // Only clear at the very beginning of function generation, before body processing.
        // TDD FIX (Bug #3): Moved clear to happen BEFORE pre-passes, so marking during
        // statement generation can accumulate variables.

        // Clear ONCE at function start (before any analysis)
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

            // Map Windjammer decorator to Rust attribute (same as struct decorator handling)
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

        // WINDJAMMER LIFETIME INFERENCE: Determine if explicit lifetime annotations are needed.
        // Rust's lifetime elision rules handle most cases automatically:
        //   1. Single input reference → output gets that lifetime
        //   2. &self/&mut self → output gets self's lifetime
        //   3. Multiple input references with no self → MUST be explicit
        // We only add 'a when case 3 applies AND the return type contains references.
        let needs_lifetime = self.function_needs_lifetime_annotations(func, analyzed);

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

        if needs_lifetime || !type_params.is_empty() {
            output.push('<');
            let mut parts = Vec::new();
            if needs_lifetime {
                parts.push("'a".to_string());
            }
            if !type_params.is_empty() {
                parts.push(self.format_type_params(&type_params));
            }
            output.push_str(&parts.join(", "));
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

        // TDD FIX: Pre-compute which parameters are actually used in the function body.
        // Unused parameters get prefixed with `_` to suppress "unused variable" warnings.
        // THE WINDJAMMER WAY: The compiler handles this automatically — developers don't
        // need to manually prefix unused parameters with `_`.
        let body_refs: Vec<&Statement> = func.body.to_vec();
        let unused_params: std::collections::HashSet<String> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .filter(|p| !Self::variable_used_in_statements(&body_refs, &p.name))
            .map(|p| p.name.clone())
            .collect();

        // TDD FIX: Pre-compute unused let bindings and for-loop variables.
        // Like unused params, these get prefixed with `_` in the generated Rust.
        self.unused_let_bindings.clear();
        Self::find_unused_bindings(&func.body, &mut self.unused_let_bindings);

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
                            // TDD FIX: Default to Owned, not Borrowed
                            // THE WINDJAMMER WAY: Parameters are owned by default unless analyzer
                            // detects they should be borrowed (e.g., only read, passed to & functions)
                            let ownership_mode = analyzed
                                .inferred_ownership
                                .get(&param.name)
                                .unwrap_or(&OwnershipMode::Owned);

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

                // WINDJAMMER LIFETIME INFERENCE: Add 'a lifetime to reference parameters
                // when the function needs explicit lifetime annotations.
                let type_str = if needs_lifetime && param.name != "self" {
                    if let Some(stripped) = type_str.strip_prefix("&mut ") {
                        format!("&'a mut {}", stripped)
                    } else if let Some(stripped) = type_str.strip_prefix("&") {
                        format!("&'a {}", stripped)
                    } else {
                        type_str
                    }
                } else {
                    type_str
                };

                // TDD FIX: Auto-infer `mut` for owned parameters
                // THE WINDJAMMER WAY: Users don't track mutability - the compiler does.
                // If a parameter has mutating method calls or field mutations,
                // the binding needs `mut` even if not explicitly written.
                let auto_needs_mut = param.name != "self"
                    && !param.is_mutable
                    && matches!(type_str.as_str(), s if !s.starts_with("&"))
                    && self.variable_needs_mut(&param.name);
                let mut_prefix = if param.is_mutable || auto_needs_mut {
                    "mut "
                } else {
                    ""
                };

                // TDD FIX: Prefix unused parameter names with `_` to suppress warnings
                let display_name = if unused_params.contains(&param.name) {
                    format!("_{}", param.name)
                } else {
                    param.name.clone()
                };

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
                    format!("{}{}: {}", mut_prefix, display_name, type_str)
                }
            })
            .collect();

        params.extend(additional_params);

        output.push_str(&params.join(", "));
        output.push(')');

        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            if needs_lifetime {
                output.push_str(&crate::codegen::rust::types::type_to_rust_with_lifetime(
                    return_type,
                ));
            } else {
                output.push_str(&self.type_to_rust(return_type));
            }
        }

        // Add where clause if present
        output.push_str(&codegen_helpers::format_where_clause(&func.where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;

        // TDD: Generate function body with return optimization
        // Set flag to enable implicit return for last statement
        let old_in_function_body = self.in_function_body;
        self.in_function_body = true;
        let mut body_code = self.generate_block(&func.body);
        self.in_function_body = old_in_function_body;

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

    pub(crate) fn type_to_rust(&self, type_: &Type) -> String {
        // Delegate to the refactored types module
        crate::codegen::rust::type_to_rust(type_)
    }

    fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Reference(inner) => format!("&{}", self.pattern_to_rust(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
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

    pub(crate) fn generate_statement(&mut self, stmt: &Statement<'ast>) -> String {
        // RECURSION GUARD: Check depth before processing statement
        if let Err(e) = self.enter_recursion("generate_statement") {
            eprintln!("{}", e);
            return format!("/* {} */", e);
        }

        // Record source mapping if location info is available
        if let Some(location) = codegen_helpers::get_statement_location(stmt) {
            self.record_mapping(&location);
        }

        let result = self.generate_statement_impl(stmt);
        self.exit_recursion();
        result
    }

    fn generate_statement_impl(&mut self, stmt: &Statement<'ast>) -> String {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                type_,
                value,
                location,
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

                // Immutable-by-default: `let` is immutable, `let mut` is mutable.
                // The compiler no longer silently infers `mut` for local bindings.
                // Users must explicitly write `let mut` when mutation is intended.
                // This follows the modern language consensus (Rust, Swift, Kotlin, Zig).
                //
                // NOTE: Parameter ownership inference (& vs &mut vs owned) is unchanged --
                // that's a mechanical detail the compiler still handles automatically.
                if needs_mut_ref {
                    // Don't add mut keyword, but we'll add &mut to the value
                } else if *mutable {
                    output.push_str("mut ");
                }

                // TDD FIX: Prefix unused let bindings with `_` to suppress warnings
                let is_unused_binding = location
                    .as_ref()
                    .is_some_and(|loc| self.unused_let_bindings.contains(&(loc.line, loc.column)));

                // Generate pattern (could be simple name or tuple)
                let pattern_str = if is_unused_binding {
                    match pattern {
                        Pattern::Identifier(name) => format!("_{}", name),
                        other => self.generate_pattern(other),
                    }
                } else {
                    self.generate_pattern(pattern)
                };
                output.push_str(&pattern_str);

                // LOCAL VARIABLE TRACKING: Add this variable to the current scope
                // This enables proper shadowing of field names
                if let Some(name) = var_name {
                    if let Some(current_scope) = self.local_variable_scopes.last_mut() {
                        current_scope.insert(name.to_string());
                    }
                }

                // LOCAL VARIABLE TYPE TRACKING: Infer type from value expression or annotation
                // This enables qualified method signature lookup (e.g., stack.remove() → Stack::remove)
                if let Some(name) = var_name {
                    let inferred_type: Option<Type> = if let Some(type_) = type_ {
                        // Explicit type annotation: let x: Foo = ...
                        Some((*type_).clone())
                    } else {
                        // Infer from value expression
                        match value {
                            Expression::StructLiteral {
                                name: struct_name, ..
                            } => Some(Type::Custom(struct_name.to_string())),
                            // Literal types: let x = 25 → i32, let y = 3.14 → f32, let b = true → bool
                            Expression::Literal {
                                value: crate::parser::Literal::Int(_),
                                ..
                            } => Some(Type::Int),
                            Expression::Literal {
                                value: crate::parser::Literal::Float(_),
                                ..
                            } => Some(Type::Float),
                            Expression::Literal {
                                value: crate::parser::Literal::Bool(_),
                                ..
                            } => Some(Type::Bool),
                            Expression::Literal {
                                value: crate::parser::Literal::String(_),
                                ..
                            } => Some(Type::String),
                            Expression::Call { function, .. } => {
                                // Type::method() pattern (e.g., Foo::new())
                                if let Expression::FieldAccess { object, field, .. } = *function {
                                    if let Expression::Identifier {
                                        name: type_name, ..
                                    } = *object
                                    {
                                        if type_name
                                            .chars()
                                            .next()
                                            .is_some_and(|c| c.is_uppercase())
                                            && (field == "new"
                                                || field.starts_with("from_")
                                                || field.starts_with("with_")
                                                || field == "default")
                                        {
                                            Some(Type::Custom(type_name.to_string()))
                                        } else {
                                            // Not a constructor — look up return type from signature registry
                                            // e.g., MathHelper::fade(x) → return type is f32
                                            let qualified = format!("{}::{}", type_name, field);
                                            self.signature_registry
                                                .get_signature(&qualified)
                                                .and_then(|sig| sig.return_type.clone())
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    // Simple function call: look up in signature registry
                                    self.infer_expression_type(value)
                                }
                            }
                            _ => {
                                // Fall back to general expression type inference
                                // Handles if/else, binary ops, method calls, etc.
                                self.infer_expression_type(value)
                            }
                        }
                    };
                    if let Some(t) = inferred_type {
                        self.local_var_types.insert(name.to_string(), t);
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

                        // TDD FIX: Vec indexing ownership inference
                        // WINDJAMMER PHILOSOPHY: "Compiler does the hard work, not the developer"
                        // 
                        // Pattern: let x = vec[i]
                        // If vec[i] type is Copy → no modification needed (Rust copies automatically)
                        // If vec[i] type is Clone (not Copy):
                        //   - If only field-accessed → &vec[i] (optimize to borrow)
                        //   - If moved/returned → vec[i].clone() (need explicit clone)
                        if matches!(value, Expression::Index { .. }) {
                            if let Some(name) = var_name {
                                // TDD FIX: Only apply ownership transformations if we can infer the type
                                // WINDJAMMER PHILOSOPHY: Be conservative - better to get a clear E0507 
                                // than add wrong & causing E0308
                                let indexed_type = self.infer_expression_type(value);
                                
                                if let Some(elem_type) = indexed_type {
                                    // SUCCESS: We know the element type
                                    let is_copy = self.is_type_copy(&elem_type);

                                    if is_copy {
                                        // Copy types don't need & or .clone() - Rust copies automatically
                                        // Example: let x = numbers[0] → let x = numbers[0]
                                        // DO NOTHING - leave as-is
                                    } else {
                                        // Non-Copy type - need to decide between & and .clone()
                                        // DATA FLOW ANALYSIS: Check how variable is used
                                        if self.variable_is_only_field_accessed(name) {
                                            // Only field-accessed → optimize with borrow
                                            // Example: let frame = frames[i]; frame.x += 1;
                                            // Generate: let frame = &frames[i]
                                            // TDD FIX: Set in_borrow_context BEFORE generating expression
                                            // to prevent Expression::Index from adding .clone()
                                            // Without this: value_str = "vec[i].clone()" then "&" → "&vec[i].clone()" ❌
                                            // With this: value_str = "vec[i]" then "&" → "&vec[i]" ✅
                                            let prev_borrow_ctx = self.in_borrow_context;
                                            self.in_borrow_context = true;
                                            value_str = self.generate_expression(value);
                                            self.in_borrow_context = prev_borrow_ctx;
                                            value_str = format!("&{}", value_str);
                                        } else {
                                            // Moved/returned → need explicit clone
                                            // Example: let child = children[idx]; recursive(child);
                                            // Expression::Index will add .clone() automatically
                                            // (no need to add it here - already in value_str)
                                        }
                                    }
                                } else {
                                    // CANNOT INFER: Leave as-is, let Rust give clear error
                                    // This happens when Vec is created without explicit type annotation
                                    // Example: let mask = Vec::with_capacity(size); let x = mask[i];
                                    // Better to get E0507 "cannot move" than E0308 "expected u8, found &u8"
                                }
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
                        // BUT: Skip .clone() for Copy types (f32, i32, bool, etc.)
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
                                        let is_copy = self
                                            .infer_expression_type(e)
                                            .as_ref()
                                            .is_some_and(|t| self.is_type_copy(t));
                                        if !is_copy {
                                            return_str = format!("{}.clone()", return_str);
                                        }
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
                let expr_str = self.generate_expression(expr);
                output.push_str(&expr_str);

                // TDD FIX: Only add semicolon if not in expression context
                // This prevents semicolons in if-else branches when used as values
                // e.g., `x = if cond { Some(42) } else { None }` (not `{ Some(42); }`)
                if !self.in_expression_context {
                    output.push_str(";\n");
                } else {
                    output.push('\n');
                }
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

                // DOGFOODING FIX: Preserve explicit returns in if-without-else
                // In Rust, `if` without `else` must evaluate to `()`, so any value expression
                // (including implicit returns) is invalid: E0308 "if without else has incompatible types"
                // 
                // Safe to optimize returns ONLY in if-else (both branches have values/returns)
                // Must preserve returns in if-without-else (then block evaluates to ())
                let old_in_func_body = self.in_function_body;
                let old_in_void_block = self.in_void_block;
                if else_block.is_none() || !self.current_is_last_statement {
                    self.in_function_body = false;
                }
                // if-without-else must evaluate to (); suppress implicit returns
                if else_block.is_none() {
                    self.in_void_block = true;
                }

                self.indent_level += 1;
                output.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;
                self.in_void_block = old_in_void_block;

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

                self.in_function_body = old_in_func_body;

                self.suppress_string_conversion = old_suppress;
                output.push('\n');
                output
            }
            Statement::Match { value, arms, .. } => {
                // TDD FIX: Optimize boolean match expressions to matches! macro
                // Clippy warns about match expressions that just return true/false
                // Example: match x { Some(_) => true, None => false } → matches!(x, Some(_))
                if arms.len() == 2 && arms[0].guard.is_none() && arms[1].guard.is_none() {
                    // Check if both arms have simple literal bodies (true/false)
                    let arm0_is_true = matches!(
                        arms[0].body,
                        Expression::Literal {
                            value: Literal::Bool(true),
                            ..
                        }
                    );
                    let arm0_is_false = matches!(
                        arms[0].body,
                        Expression::Literal {
                            value: Literal::Bool(false),
                            ..
                        }
                    );
                    let arm1_is_true = matches!(
                        arms[1].body,
                        Expression::Literal {
                            value: Literal::Bool(true),
                            ..
                        }
                    );
                    let arm1_is_false = matches!(
                        arms[1].body,
                        Expression::Literal {
                            value: Literal::Bool(false),
                            ..
                        }
                    );

                    // Pattern 1: first arm true, second arm false
                    // match x { Pattern => true, _ => false } → matches!(x, Pattern)
                    if arm0_is_true && arm1_is_false {
                        let value_str = self.generate_expression(value);
                        let pattern_str = self.generate_pattern(&arms[0].pattern);
                        let mut output = self.indent();
                        output.push_str(&format!("matches!({}, {})\n", value_str, pattern_str));
                        return output;
                    }

                    // Pattern 2: first arm false, second arm true
                    // match x { Pattern => false, _ => true } → !matches!(x, Pattern)
                    if arm0_is_false && arm1_is_true {
                        let value_str = self.generate_expression(value);
                        let pattern_str = self.generate_pattern(&arms[0].pattern);
                        let mut output = self.indent();
                        output.push_str(&format!("!matches!({}, {})\n", value_str, pattern_str));
                        return output;
                    }
                }

                // TDD FIX: Detect `if let` pattern and generate `if let` instead of `match`
                //
                // The parser converts `if let Pattern = expr { body }` into:
                //   Statement::Match { arms: [MatchArm(Pattern, body), MatchArm(Wildcard, empty_block)] }
                //
                // We detect this pattern (2 arms, last is Wildcard) and generate proper
                // `if let` syntax, eliminating clippy's "single pattern match" warnings.
                //
                // THE WINDJAMMER WAY: The compiler generates idiomatic Rust, not just correct Rust.
                if arms.len() == 2
                    && matches!(arms[1].pattern, Pattern::Wildcard)
                    && arms[1].guard.is_none()
                {
                    let wildcard_body_is_empty =
                        if let Expression::Block { statements, .. } = arms[1].body {
                            statements.is_empty()
                        } else {
                            false
                        };

                    let wildcard_body_stmts: Option<&[&Statement]> =
                        if let Expression::Block { statements, .. } = arms[1].body {
                            if statements.is_empty() {
                                None
                            } else {
                                Some(statements)
                            }
                        } else {
                            None
                        };

                    // Only convert to if-let when the wildcard arm is empty or has an else body
                    // Skip when borrow-break is needed (rare edge case, keep as match)
                    let match_binds_refs_early_check = self.match_expression_binds_refs(value);
                    let needs_borrow_break_check = match_binds_refs_early_check
                        && self.match_scrutinee_is_self_method_call(value)
                        && self.match_arms_mutate_self(arms);

                    if !needs_borrow_break_check
                        && (wildcard_body_is_empty || wildcard_body_stmts.is_some())
                    {
                        let value_str = self.generate_expression(value);
                        let main_arm = &arms[0];

                        // Track pattern-bound variables (same as regular match)
                        let match_binds_refs = self.match_expression_binds_refs(value);
                        let mut bound_vars = std::collections::HashSet::new();
                        self.extract_pattern_bindings(&main_arm.pattern, &mut bound_vars);

                        let added_borrowed: Vec<String> = if match_binds_refs {
                            bound_vars.iter().cloned().collect()
                        } else {
                            Vec::new()
                        };
                        for var in &added_borrowed {
                            self.borrowed_iterator_vars.insert(var.clone());
                        }

                        self.local_variable_scopes.push(bound_vars);

                        let match_bound_type_entries: Vec<(String, Type)> =
                            self.infer_match_bound_types(value, &main_arm.pattern);
                        for (var_name, var_type) in &match_bound_type_entries {
                            self.local_var_types
                                .insert(var_name.clone(), var_type.clone());
                        }

                        // Generate: if let Pattern = value {
                        let mut output = self.indent();
                        output.push_str("if let ");
                        output.push_str(&self.generate_pattern(&main_arm.pattern));

                        if let Some(guard) = &main_arm.guard {
                            output.push_str(" if ");
                            output.push_str(&self.generate_expression(guard));
                        }

                        output.push_str(" = ");
                        output.push_str(&value_str);
                        output.push_str(" {\n");

                        // DOGFOODING FIX: Preserve explicit returns in if-let-without-else
                        // Match-to-if-let optimization must NOT optimize returns in the then block
                        // when there's no else block, to prevent E0308 errors.
                        let has_else = wildcard_body_stmts.is_some();
                        let old_in_func_body = self.in_function_body;
                        if !has_else {
                            // No else block → disable return optimization
                            self.in_function_body = false;
                        }

                        // Generate the then-block body
                        self.indent_level += 1;
                        if let Expression::Block { statements, .. } = main_arm.body {
                            output.push_str(&self.generate_block(statements));
                        } else {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_expression(main_arm.body));
                            output.push_str(";\n");
                        }
                        self.indent_level -= 1;

                        output.push_str(&self.indent());
                        output.push('}');

                        // Generate else block if wildcard arm has a non-empty body
                        if let Some(else_stmts) = wildcard_body_stmts {
                            output.push_str(" else {\n");
                            self.indent_level += 1;
                            output.push_str(&self.generate_block(else_stmts));
                            self.indent_level -= 1;
                            output.push_str(&self.indent());
                            output.push('}');
                        }
                        
                        // Restore flag after generating both then and else blocks
                        self.in_function_body = old_in_func_body;

                        output.push('\n');

                        // Clean up scopes
                        self.local_variable_scopes.pop();
                        for (var_name, _) in &match_bound_type_entries {
                            self.local_var_types.remove(var_name);
                        }
                        for var in &added_borrowed {
                            self.borrowed_iterator_vars.remove(var);
                        }

                        return output;
                    }
                }

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

                // TDD FIX: Detect borrow conflict pattern in match on self.method()
                //
                // When the match scrutinee borrows from self (e.g., self.current_scene_id()
                // returning Option<&str>) AND any arm body mutates self (e.g.,
                // self.paused_scenes.remove(current)), Rust reports E0502 because the
                // immutable borrow from the method call conflicts with the mutable borrow
                // needed for the mutation.
                //
                // THE WINDJAMMER WAY: The compiler handles this automatically by extracting
                // the scrutinee into an owned temporary, breaking the borrow chain:
                //   let __match_borrow_break = self.method().map(|v| v.to_owned());
                //   match __match_borrow_break.as_ref() { ... }
                //
                // We use .as_ref() (not .as_deref()) because .as_deref() requires the
                // inner type to implement Deref, which fails for custom types like
                // DialogueNode. .as_ref() works universally for all types:
                //   Option<String>.as_ref() → Option<&String> (auto-coerces to &str)
                //   Option<Custom>.as_ref() → Option<&Custom> (works for any type)
                let match_binds_refs_early = self.match_expression_binds_refs(value);
                let needs_borrow_break = match_binds_refs_early
                    && self.match_scrutinee_is_self_method_call(value)
                    && self.match_arms_mutate_self(arms);

                let mut output = self.indent();

                if needs_borrow_break {
                    // Extract scrutinee into owned temporary to break borrow on self
                    output.push_str(&format!(
                        "let __match_borrow_break = {}.map(|__v| __v.to_owned());\n",
                        value_str
                    ));
                    output.push_str(&self.indent());
                    output.push_str("match __match_borrow_break.as_ref()");
                } else {
                    output.push_str("match ");
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
                }

                output.push_str(" {\n");

                self.indent_level += 1;

                // TDD FIX: Detect if match expression produces references for pattern-bound variables
                // When matching on &expr (like `match &self.field { Some(var) => ... }`),
                // `var` is automatically a reference (&T). We track these to prevent
                // double-borrowing (e.g., HashMap.get(&var) where var is already &String).
                // Also handles methods returning Option<&T> (like current_scene_id() -> Option<&str>).
                let match_binds_refs = self.match_expression_binds_refs(value);

                // WINDJAMMER PHILOSOPHY: Auto-convert match arm strings when return type is String
                // OR when any other arm produces a String (e.g., format! macro, or blocks with converted strings)
                let needs_string_conversion = match &self.current_function_return_type {
                    Some(Type::String) => true,
                    Some(Type::Custom(name)) if name == "String" => true,
                    _ => {
                        // Check if any arm produces a String (format!, to_string(), etc.)
                        // OR if any arm has a block whose last expression is converted to String
                        arms.iter().any(|arm| {
                            string_analysis::expression_produces_string(arm.body)
                                || arm_string_analysis::arm_returns_converted_string(arm.body)
                        })
                    }
                };

                // TDD FIX: Track if this is a statement-context match (not used as an expression)
                // In statement matches, arm blocks should preserve semicolons on all statements
                // Only apply this when the function returns void (no return type)
                let old_in_statement_match = self.in_statement_match;
                let match_is_statement = self.current_function_return_type.is_none();
                if match_is_statement {
                    self.in_statement_match = true;
                }

                for arm in arms {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_pattern(&arm.pattern));

                    // Add guard if present
                    if let Some(guard) = &arm.guard {
                        output.push_str(" if ");
                        output.push_str(&self.generate_expression(guard));
                    }

                    output.push_str(" => ");

                    // TDD FIX: Track pattern-bound variables as local variables
                    // This prevents them from being incorrectly resolved as `self.field`
                    // Example: `Some(search)` binds `search` as a local variable
                    let mut bound_vars = std::collections::HashSet::new();
                    self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);

                    // TDD FIX: Track match-bound reference variables
                    // When matching on &expr or Option<&T>, pattern-bound vars are already references.
                    // We must NOT add & to them (would create &&T, which is incorrect).
                    // Example: match &self.current_animation { Some(anim_name) => ... }
                    //   → anim_name is &String, self.animations.get(anim_name) is correct
                    //   → self.animations.get(&anim_name) would be WRONG (&&String)
                    let added_borrowed: Vec<String> = if match_binds_refs {
                        bound_vars.iter().cloned().collect()
                    } else {
                        Vec::new()
                    };
                    for var in &added_borrowed {
                        self.borrowed_iterator_vars.insert(var.clone());
                    }

                    // Create a new scope for this match arm
                    self.local_variable_scopes.push(bound_vars);

                    // TDD FIX: Track types of match-bound variables
                    // When matching `Some(x)` on an `Option<T>` expression,
                    // `x` has type `T`. This enables qualified method signature lookup.
                    // Example: let opt: Option<Stack> = ...; if let Some(s) = opt { s.remove(v) }
                    //   → infer_type_name("s") should return "Stack" → Stack::remove is found
                    let match_bound_type_entries: Vec<(String, Type)> =
                        self.infer_match_bound_types(value, &arm.pattern);
                    for (var_name, var_type) in &match_bound_type_entries {
                        self.local_var_types
                            .insert(var_name.clone(), var_type.clone());
                    }

                    // Set context flag for block generation
                    let old_in_match_arm = self.in_match_arm_needing_string;
                    if needs_string_conversion {
                        self.in_match_arm_needing_string = true;
                    }

                    // Auto-convert string literals to String when any arm produces String
                    let mut arm_str = self.generate_expression(arm.body);

                    self.in_match_arm_needing_string = old_in_match_arm;

                    // Pop the match arm scope
                    self.local_variable_scopes.pop();

                    // Clean up match-bound type entries
                    for (var_name, _) in &match_bound_type_entries {
                        self.local_var_types.remove(var_name);
                    }

                    // Clean up match-bound borrowed variables
                    for var in &added_borrowed {
                        self.borrowed_iterator_vars.remove(var);
                    }
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

                // Restore statement match context
                self.in_statement_match = old_in_statement_match;

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
                // TDD FIX (Bug #3): Before generating while condition expression,
                // check if it compares a variable to .len() - if so, mark that variable as usize
                // This must happen BEFORE generate_expression to prevent `as i64` cast
                self.mark_usize_variables_in_condition(condition);

                let mut output = self.indent();
                output.push_str("while ");
                
                // Now generate the condition - usize variables already marked
                let condition_str = self.generate_expression(condition);
                output.push_str(&condition_str);
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
                location,
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

                // Check if we need to add & for borrowed iteration
                // This handles the common case of iterating over fields of borrowed structs
                let needs_borrow = self.should_borrow_for_iteration(iterable);
                let needs_mut_borrow = needs_mut && needs_borrow;

                // TDD FIX: Only add `mut` to the loop variable when it's reassigned directly,
                // NOT when iterating with `&mut`. When iterating with `&mut`, the loop variable
                // is already a `&mut T` reference, so field modifications work without `mut` on
                // the binding. Adding `mut` generates: `for mut member in &mut collection`
                // which triggers "variable does not need to be mutable" warning.
                //
                // Check two cases:
                // 1. We infer `&mut` iteration (needs_mut_borrow) - don't add `mut`
                // 2. Source already has `&mut` on the iterable (Expression::Unary MutRef) - don't add `mut`
                let iterable_already_mut_ref = matches!(
                    iterable,
                    Expression::Unary {
                        op: UnaryOp::MutRef,
                        ..
                    }
                );
                if needs_mut && !needs_mut_borrow && !iterable_already_mut_ref {
                    output.push_str("mut ");
                }

                // TDD FIX: Prefix unused for-loop variables with `_`
                let is_unused_loop_var = location
                    .as_ref()
                    .is_some_and(|loc| self.unused_let_bindings.contains(&(loc.line, loc.column)));
                let display_pattern = if is_unused_loop_var {
                    format!("_{}", pattern_str)
                } else {
                    pattern_str
                };
                output.push_str(&display_pattern);
                output.push_str(" in ");

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

                // TDD FIX: Strip explicit & from iterable when inner identifier is
                // already a borrowed param. Prevents &&Vec<T> which isn't iterable.
                // Example: for x in &stacks where stacks: &Vec<T> → for x in stacks
                let iterable_to_generate = if let Expression::Unary {
                    op: crate::parser::UnaryOp::Ref,
                    operand,
                    ..
                } = iterable
                {
                    if let Expression::Identifier { name, .. } = &**operand {
                        if self.inferred_borrowed_params.contains(name) {
                            operand // Strip & — param is already a reference
                        } else {
                            iterable
                        }
                    } else {
                        iterable
                    }
                } else {
                    iterable
                };

                output.push_str(&self.generate_expression(iterable_to_generate));
                output.push_str(" {\n");

                self.indent_level += 1;

                // Track borrowed iterator variable for field access cloning
                if is_borrowed_iterator {
                    if let Some(var) = &loop_var {
                        self.borrowed_iterator_vars.insert(var.clone());
                    }
                }

                // TDD FIX: Track owned String iterator variables (from Vec<String>)
                // These need to be borrowed when used in String += operations
                // Heuristic: If NOT borrowed iterator AND iterable looks like a Vec parameter
                let is_owned_string_iterator = !is_borrowed_iterator;
                if is_owned_string_iterator {
                    if let Some(var) = &loop_var {
                        self.owned_string_iterator_vars.insert(var.clone());
                    }
                }

                // TDD FIX: Track range iteration variables as usize
                // When iterating `for i in 0..items.len()`, the loop variable `i` is usize.
                // This prevents redundant `i as usize` casts in the loop body.
                if let Some(var) = &loop_var {
                    if let Expression::Range { end, .. } = iterable {
                        if self.expression_produces_usize(end) {
                            self.usize_variables.insert(var.clone());
                        }
                    }
                }

                // TDD FIX: Track for-loop variable types for method signature lookup
                // When iterating `for slot in slots` where `slots: Vec<Option<T>>`,
                // `slot` has type `Option<T>`. This enables match-bound type inference:
                // `if let Some(x) = slot` → x has type T → x.method() resolves correctly.
                if let Some(var) = &loop_var {
                    if let Some(iterable_type) = self.infer_expression_type(iterable) {
                        if let Some(elem_type) = Self::extract_iterator_element_type(&iterable_type)
                        {
                            self.local_var_types.insert(var.clone(), elem_type);
                        }
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
                if is_owned_string_iterator {
                    if let Some(var) = &loop_var {
                        self.owned_string_iterator_vars.remove(var);
                    }
                }
                // Clean up for-loop variable type tracking
                if let Some(var) = &loop_var {
                    self.local_var_types.remove(var);
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

                    // TDD FIX: For String += String, we need to borrow the RHS
                    // String implements AddAssign<&str>, not AddAssign<String>
                    let mut value_str = self.generate_expression(value);
                    if matches!(op, CompoundOp::Add) {
                        // Check if the value is an identifier (owned String)
                        if let Expression::Identifier { name, .. } = value {
                            // Only add & if this is a tracked owned String iterator variable
                            // These are owned Strings from for-loops over Vec<String>
                            if self.owned_string_iterator_vars.contains(name) {
                                value_str = format!("&{}", value_str);
                            }
                        }
                    }

                    output.push_str(&value_str);
                    output.push_str(";\n");
                    return output;
                }

                // Regular assignment: target = value

                // PHASE 5 OPTIMIZATION: Check if this assignment matches x = x + y pattern
                // If so, convert to compound assignment: x += y
                // Handles both simple identifiers (x = x + y) and field access (self.x = self.x + y)
                if let Expression::Binary {
                    left, right, op, ..
                } = value
                {
                    let targets_match = match (target, &**left) {
                        // Simple: x = x + y
                        (
                            Expression::Identifier { name: t, .. },
                            Expression::Identifier { name: l, .. },
                        ) => t == l,
                        // Field access (any depth): self.x, obj.field, entity.transform.x
                        // Compare by generated string to handle nested field chains uniformly
                        (Expression::FieldAccess { .. }, Expression::FieldAccess { .. })
                        | (Expression::Index { .. }, Expression::Index { .. }) => {
                            self.generate_expression(target) == self.generate_expression(left)
                        }
                        _ => false,
                    };

                    // SAFETY: Only apply compound assignment for types known to support it.
                    // Primitive types (i32, f32, i64, f64, usize, u32, String) always
                    // implement AddAssign etc. Custom types (Vec3, Color) may NOT,
                    // even if they implement Add + Copy.
                    let target_type = self.infer_expression_type(target);
                    // Check if type is a known custom type that may NOT implement AddAssign.
                    // Types like Vec2, Vec3, Color implement Add but not AddAssign.
                    let is_known_non_assignable = target_type.as_ref().is_some_and(|t| {
                        if let Type::Custom(name) = t {
                            // Blacklist: custom types from the game engine that implement
                            // Add/Sub/Mul but NOT AddAssign/SubAssign/MulAssign
                            matches!(
                                name.as_str(),
                                "Vec2"
                                    | "Vec3"
                                    | "Vec4"
                                    | "Color"
                                    | "Quat"
                                    | "Mat3"
                                    | "Mat4"
                                    | "Point"
                                    | "Size"
                            )
                        } else {
                            false
                        }
                    });
                    // Compound assignment is safe when:
                    // 1. Simple identifier: x = x + y always implies x += y works
                    // 2. Not a known non-assignable custom type (Vec3, Color, etc.)
                    //    If type is None (inference failed) or primitive, it's safe because
                    //    field assignments like self.hp = self.hp + 1 are almost always numeric.
                    let is_compound_safe = !is_known_non_assignable;

                    if targets_match && is_compound_safe {
                        let compound_op_str = match op {
                            BinaryOp::Add => Some("+="),
                            BinaryOp::Sub => Some("-="),
                            BinaryOp::Mul => Some("*="),
                            BinaryOp::Div => Some("/="),
                            BinaryOp::Mod => Some("%="),
                            BinaryOp::BitAnd => Some("&="),
                            BinaryOp::BitOr => Some("|="),
                            BinaryOp::BitXor => Some("^="),
                            BinaryOp::Shl => Some("<<="),
                            BinaryOp::Shr => Some(">>="),
                            _ => None,
                        };

                        if let Some(op_str) = compound_op_str {
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

                // Fall back to regular assignment
                // CRITICAL: Set flag to suppress auto-clone for assignment targets
                self.generating_assignment_target = true;
                output.push_str(&self.generate_expression(target));
                self.generating_assignment_target = false;
                output.push_str(" = ");

                // TDD: Set expression context for the value
                // This prevents adding semicolons to if-else branches when used as values
                // Bug was: `x = if cond { Some(42); } else { None; }` (semicolons broke it)
                // Fix: `x = if cond { Some(42) } else { None }` (expression, no semicolons)
                let old_expr_ctx = self.in_expression_context;
                self.in_expression_context = true;

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

                // TDD FIX: Auto-clone when assigning borrowed String param to owned String field
                // Bug: self.data.field = param where param: &String, field: String
                // Error: expected `String`, found `&String`
                // Solution: Auto-insert .clone() or .to_string()
                if let Expression::Identifier { ref name, .. } = value {
                    if self.inferred_borrowed_params.contains(name) {
                        // Check if target is a String field using full type inference
                        let target_type = self.infer_expression_type(target);
                        if let Some(Type::String) = target_type {
                            // Borrowed param -> owned String field: need .clone()
                            if !value_str.contains(".clone()") && !value_str.contains(".to_string()") {
                                value_str = format!("{}.clone()", value_str);
                            }
                        }
                    }
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
                        Some("int") | Some("i64") => {
                            // Target is i64 (Windjammer's default int type)
                            value_str = format!("(({}) as i64)", value_str);
                        }
                        Some("i32") => {
                            // Target is explicit i32, cast to i32
                            value_str = format!("(({}) as i32)", value_str);
                        }
                        _ => {
                            // Unknown or generic type, don't cast (let Rust's type inference handle it)
                        }
                    }
                }

                output.push_str(&value_str);

                // Restore expression context
                self.in_expression_context = old_expr_ctx;

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
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
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
                                .map(|(n, pat)| {
                                    // THE WINDJAMMER WAY: Use shorthand field pattern when
                                    // binding name matches field name (e.g., `{ base, height }`
                                    // instead of `{ base: base, height: height }`)
                                    if let Pattern::Identifier(binding) = pat {
                                        if binding == n {
                                            return n.clone();
                                        }
                                    }
                                    format!("{}: {}", n, self.generate_pattern(pat))
                                })
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

    /// TDD FIX: Extract all variable bindings from a pattern
    /// This tracks pattern-bound variables (e.g., `search` in `Some(search)`)
    /// so they can be added to local_variable_scopes and properly shadow struct fields
    fn extract_pattern_bindings(
        &self,
        pattern: &Pattern,
        bindings: &mut std::collections::HashSet<String>,
    ) {
        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier binding: Some(x) binds 'x'
                bindings.insert(name.clone());
            }
            Pattern::Reference(inner) => {
                // &pattern - recurse into inner pattern
                self.extract_pattern_bindings(inner, bindings);
            }
            Pattern::Ref(name) | Pattern::RefMut(name) => {
                // ref x or ref mut x - binds 'x' by reference
                bindings.insert(name.clone());
            }
            Pattern::EnumVariant(_name, binding) => {
                use crate::parser::EnumPatternBinding;
                match binding {
                    EnumPatternBinding::Single(var_name) => {
                        // Some(x) binds 'x'
                        bindings.insert(var_name.clone());
                    }
                    EnumPatternBinding::Tuple(patterns) => {
                        // Some((x, y)) binds 'x' and 'y'
                        for pat in patterns {
                            self.extract_pattern_bindings(pat, bindings);
                        }
                    }
                    EnumPatternBinding::Struct(fields, _) => {
                        // Some { x, y } binds 'x' and 'y'
                        for (_field_name, pat) in fields {
                            self.extract_pattern_bindings(pat, bindings);
                        }
                    }
                    EnumPatternBinding::Wildcard | EnumPatternBinding::None => {
                        // No bindings
                    }
                }
            }
            Pattern::Tuple(patterns) => {
                // (x, y, z) - recurse into all tuple elements
                for pat in patterns {
                    self.extract_pattern_bindings(pat, bindings);
                }
            }
            Pattern::Or(patterns) => {
                // x | y | z - recurse into all or patterns
                for pat in patterns {
                    self.extract_pattern_bindings(pat, bindings);
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {
                // No bindings
            }
        }
    }

    /// TDD FIX: Detect if a match expression produces references for pattern-bound variables
    /// Check if a type is Copy, considering both primitive types (i32, f32, bool, etc.)
    /// and user-defined types with @derive(Copy) (e.g., VoxelType, FaceDirection).
    /// Extract the root identifier from a (possibly nested) field access expression.
    /// e.g., stack.item.id → "stack", self.field → "self"
    fn extract_root_identifier(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => self.extract_root_identifier(object),
            Expression::Index { object, .. } => self.extract_root_identifier(object),
            _ => None,
        }
    }

    fn is_type_copy(&self, ty: &Type) -> bool {
        crate::codegen::rust::type_analysis::is_copy_type(ty)
            || match ty {
                Type::Custom(name) => self.copy_types_registry.contains(name.as_str()),
                _ => false,
            }
    }

    /// When matching on:
    ///   1. &expr (explicit reference) - pattern vars are references (e.g., match &self.field)
    ///   2. method() returning Option<&T> - pattern vars are references (e.g., match self.current_scene_id())
    ///
    /// This prevents double-borrowing: HashMap.get(&var) where var is already &String would be &&String
    fn match_expression_binds_refs(&self, expr: &Expression) -> bool {
        match expr {
            // match &expr { Some(var) => ... } — var is a reference
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                ..
            } => true,

            // TDD FIX (Bug #6): match self { Enum::Variant(x) => ... } when self is &self
            // When matching on a borrowed parameter (like &self), enum variant bindings are borrowed
            Expression::Identifier { name, .. } => {
                // Check if this identifier is a borrowed parameter
                self.inferred_borrowed_params.contains(name.as_str())
            }

            // match method_call() { Some(var) => ... } — check if return type contains &T
            Expression::MethodCall { method, object, .. } => {
                let type_name = self.infer_type_name(object);
                let sig = if let Some(ref type_name) = type_name {
                    let qualified = format!("{}::{}", type_name, method);
                    self.signature_registry.get_signature(&qualified)
                } else {
                    self.signature_registry.get_signature(method)
                };
                if let Some(sig) = sig {
                    if let Some(ref ret_type) = sig.return_type {
                        Self::type_contains_reference(ret_type)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            // match func_call() - check if return type contains &T
            Expression::Call { function, .. } => {
                let func_name =
                    crate::codegen::rust::ast_utilities::extract_function_name(function);
                if !func_name.is_empty() {
                    if let Some(sig) = self.signature_registry.get_signature(&func_name) {
                        if let Some(ref ret_type) = sig.return_type {
                            return Self::type_contains_reference(ret_type);
                        }
                    }
                }
                false
            }

            _ => false,
        }
    }

    /// Check if a type contains a reference (directly or inside Option/Result)
    fn type_contains_reference(ty: &Type) -> bool {
        match ty {
            Type::Reference(_) | Type::MutableReference(_) => true,
            Type::Option(inner) => Self::type_contains_reference(inner),
            Type::Result(ok, _err) => Self::type_contains_reference(ok),
            _ => false,
        }
    }

    /// Check if the match scrutinee is a method call on self (e.g., self.current_scene_id())
    /// or on self.field (e.g., self.scene_stack.last())
    fn match_scrutinee_is_self_method_call(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { object, .. } => {
                // self.method()
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return true;
                    }
                }
                // self.field.method()
                if let Expression::FieldAccess {
                    object: inner_obj, ..
                } = &**object
                {
                    if let Expression::Identifier { name, .. } = &**inner_obj {
                        if name == "self" {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if any match arm body mutates self fields.
    /// Used to detect borrow conflicts in match on self.method().
    fn match_arms_mutate_self(&self, arms: &[crate::parser::MatchArm]) -> bool {
        let ctx = self_analysis::AnalysisContext::new(&[], &self.current_struct_fields);
        arms.iter().any(|arm| {
            // The arm body is an Expression (often a Block)
            self_analysis::expression_mutates_fields(&ctx, arm.body)
        })
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
            Pattern::Ref(_) | Pattern::RefMut(_) => false, // ref/ref mut borrow, don't move
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
                    // Strip generic parameters: "Pool<T>" → "Pool"
                    if let Some(struct_name) = &self.current_struct_name {
                        let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                        // Check if this field is tracked as usize
                        if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                            if usize_fields.contains(field) {
                                return Some("usize".to_string());
                            }
                        }
                        // If not usize, assume it's i64 (int) for numeric types
                        // This is a heuristic - we can't know for sure without more type info
                        // WINDJAMMER: int type maps to i64 in Rust by default
                        // For explicit i32 fields, we'd need proper type tracking
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
            // BUGFIX: Some(...) is a constructor, not a method call
            // Don't add .cloned() to Some(squad) when squad is already &Squad
            Expression::Call { .. } => {
                // Function calls (like Some, None, Ok, Err) are not methods
                false
            }
            _ => false,
        }
    }

    fn generate_expression_with_precedence(&mut self, expr: &Expression<'ast>) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. }
            | Expression::Binary { .. }
            | Expression::Closure { .. }
            | Expression::Unary { .. }
            | Expression::Cast { .. } => {
                // Unary expressions like (*entity).field need parens for correct precedence
                // Without parens: *entity.field means *(entity.field) - WRONG
                // With parens: (*entity).field means dereference then access field - CORRECT
                //
                // Cast expressions like (x as usize).method() need parens because `as` has
                // lower precedence than `.` in Rust:
                // Without parens: x as usize.method() means x as (usize.method()) - WRONG
                // With parens: (x as usize).method() - CORRECT
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        }
    }

    // PHASE 7: Constant folding - evaluate constant expressions at compile time
    pub(crate) fn generate_expression(&mut self, expr: &Expression<'ast>) -> String {
        // RECURSION GUARD: Check depth before processing expression
        if let Err(e) = self.enter_recursion("generate_expression") {
            eprintln!("{}", e);
            return format!("/* {} */", e);
        }

        // PHASE 7: Try constant folding first
        let folded_expr = constant_folding::try_fold_constant(expr);
        let expr_to_generate = folded_expr.as_ref().unwrap_or(expr);

        let result = self.generate_expression_impl(expr_to_generate);
        self.exit_recursion();
        result
    }

    fn generate_expression_impl(&mut self, expr_to_generate: &Expression<'ast>) -> String {
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
                // CRITICAL: Never clone assignment targets (left side of `=`)
                // DOUBLE-CLONE FIX: Skip auto-clone when inside an explicit .clone() call
                if !self.generating_assignment_target && !self.in_explicit_clone_call {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(name, self.current_statement_idx)
                            .is_some()
                        {
                            // Skip .clone() for Copy types — they are implicitly copied,
                            // so .clone() is unnecessary noise.
                            let is_copy_type = analysis.string_literal_vars.contains(name)
                                || self.usize_variables.contains(name)
                                || self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));

                            if !is_copy_type {
                                // Automatically insert .clone() - this is the magic!
                                return format!("{}.clone()", base_name);
                            }
                        }
                    }
                }

                base_name
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                
                // TDD FIX: Optimize .len() comparisons to .is_empty()
                // Clippy warns about .len() == 0, .len() != 0, .len() > 0
                // Transform to .is_empty() or !.is_empty()
                if let Expression::MethodCall {
                    object,
                    method,
                    arguments,
                    ..
                } = left
                {
                    if method == "len" && arguments.is_empty() {
                        // Check if comparing to 0
                        if let Expression::Literal {
                            value: Literal::Int(0),
                            ..
                        } = right
                        {
                            match op {
                                BinaryOp::Eq => {
                                    // .len() == 0 → .is_empty()
                                    let obj_str = self.generate_expression(object);
                                    return format!("{}.is_empty()", obj_str);
                                }
                                BinaryOp::Ne | BinaryOp::Gt => {
                                    // .len() != 0 → !.is_empty()
                                    // .len() > 0 → !.is_empty()
                                    let obj_str = self.generate_expression(object);
                                    return format!("!{}.is_empty()", obj_str);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Special handling for string concatenation
                if matches!(op, BinaryOp::Add) {
                    // Only treat as string concat if at least one operand is definitely a string literal
                    let has_string_literal = matches!(
                        left,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || matches!(
                        right,
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
                    right,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );
                let left_is_int_literal = matches!(
                    left,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );

                // COMPARISON CLONE SUPPRESSION: For comparison operators (==, !=, <, >, etc.),
                // suppress borrowed-iterator cloning on operands. Comparisons work on references
                // in Rust (&String == &String, &T == &T via PartialEq), so cloning is unnecessary.
                // e.g., `recipe.name.clone() == target` → `recipe.name == target`
                let prev_suppress = self.suppress_borrowed_clone;
                if is_comparison {
                    self.suppress_borrowed_clone = true;
                }

                // Wrap operands in parens if they have lower precedence
                let mut left_str = match left {
                    Expression::Binary { op: left_op, .. } => {
                        if operators::op_precedence(left_op) < operators::op_precedence(op) {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left),
                };
                let mut right_str = match right {
                    Expression::Binary { op: right_op, .. } => {
                        if operators::op_precedence(right_op) < operators::op_precedence(op) {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right),
                };

                // Restore previous suppress state
                self.suppress_borrowed_clone = prev_suppress;

                // WINDJAMMER PHILOSOPHY: Auto-cast int/usize in comparisons
                // When comparing int (i64) with usize, automatically cast to make it work.
                //
                // CORRECTNESS: Always cast the usize side to i64, NOT the int side to usize.
                // Casting i64 → usize is UNSAFE for negative values (wraps to huge number).
                // Casting usize → i64 is SAFE (vec lengths always fit in i64).
                //
                // For int literals compared to usize: cast literal to usize (always non-negative).
                // For int variables compared to usize: cast usize to i64 (preserves negative semantics).
                //
                // Example: items.len() >= 10 → items.len() >= 10usize (literal, always safe)
                // Example: index >= items.len() → index >= (items.len() as i64) (safe cast)
                //
                // IMPORTANT: Wrap the cast operand in ((...) as i64) to handle compound
                // expressions like `width * height` → ((width * height) as i64), not
                // (width * (height as i64)) which would have wrong precedence.
                if is_comparison && left_is_usize && !right_is_usize {
                    // Left is usize, right is NOT usize
                    if right_is_int_literal {
                        // Int literals in comparisons with usize don't need explicit cast —
                        // Rust infers the literal type from context. `vec.len() > 0` is fine.
                    } else {
                        // Cast the usize side (LEFT) to i64 for safety
                        // Use parens around compound expressions to prevent precedence issues
                        // because `as` has higher precedence than arithmetic:
                        // `a + b as i64` → `a + (b as i64)` (wrong), need `(a + b) as i64`
                        let needs_inner_parens = matches!(left, Expression::Binary { .. });
                        if needs_inner_parens {
                            left_str = format!("({}) as i64", left_str);
                        } else {
                            left_str = format!("{} as i64", left_str);
                        }
                    }
                } else if is_comparison && right_is_usize && !left_is_usize {
                    // Right is usize, left is NOT usize
                    if left_is_int_literal {
                        // Int literals in comparisons with usize don't need explicit cast —
                        // Rust infers the literal type from context.
                    } else {
                        // Cast the usize side (RIGHT) to i64 for safety
                        // Use parens around compound expressions to prevent precedence issues
                        let needs_inner_parens = matches!(right, Expression::Binary { .. });
                        if needs_inner_parens {
                            right_str = format!("({}) as i64", right_str);
                        } else {
                            right_str = format!("{} as i64", right_str);
                        }
                    }
                }
                // If both are usize: no cast (usize == usize is fine)
                // If neither is usize: no cast (i64 == i64 is fine)

                // AUTO-CAST: When doing arithmetic between usize and int literal, Rust infers
                // the literal type from context. So `items.len() - 1` works without casting.
                // Only cast if the literal is negative (usize can't represent negative values).
                if is_arithmetic && left_is_usize && right_is_int_literal && !right_is_usize {
                    let is_negative = matches!(right, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
                    if is_negative {
                        right_str = format!("{} as usize", right_str);
                    }
                } else if is_arithmetic && right_is_usize && left_is_int_literal && !left_is_usize {
                    let is_negative = matches!(left, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
                    if is_negative {
                        left_str = format!("{} as usize", left_str);
                    }
                }

                let op_str = operators::binary_op_to_rust(op);

                // TDD FIX: Rust parses `expr as usize < y` as `expr as usize<y>` (generics).
                // When the left operand is a cast (or ends with `as TYPE`) and the operator
                // is `<`, we must wrap the left side in parentheses to disambiguate.
                // Other comparison operators (>=, <=, ==, !=, >) don't have this ambiguity.
                //
                // TDD FIX (VOXEL DOGFOODING): Bitwise operators (<<, >>, |, &, ^) have
                // LOWER precedence than `as` in Rust, so `(x as u32) << 8` is required.
                // Without parens: `x as u32 << 8` is parsed as `x as (u32 << 8)` - WRONG!
                //
                // DISCOVERED: VoxelColor::to_hex() compilation failure
                //   Source: `let r = (self.r as u32) << 24;`
                //   Generated: `let r = self.r as u32 << 24;`  ← Missing parens!
                //   Error: `<<` is interpreted as start of generic arguments for `u32`
                let needs_cast_parens_for_op = matches!(
                    op_str,
                    "<" | ">" | "<<" | ">>" | "|" | "&" | "^"
                );
                let left_needs_cast_parens = needs_cast_parens_for_op
                    && (matches!(left, Expression::Cast { .. }) || left_str.contains(" as "));
                let right_needs_cast_parens = needs_cast_parens_for_op
                    && (matches!(right, Expression::Cast { .. }) || right_str.contains(" as "));

                if left_needs_cast_parens {
                    left_str = format!("({})", left_str);
                }
                if right_needs_cast_parens {
                    right_str = format!("({})", right_str);
                }

                // TDD FIX: Smart XOR deref logic for string comparisons
                // Check if BOTH sides are borrowed, or only ONE side is borrowed
                //
                // Rules:
                // - Both borrowed (&String == &String): NO deref (PartialEq<&T> works)
                // - Both owned (String == String): NO deref (PartialEq<T> works)
                // - One borrowed, one owned: Add * to borrowed side (XOR)
                //
                // Borrowed sources:
                // - Identifier in inferred_borrowed_params (function parameters like `name: &String`)
                // - Identifier in borrowed_iterator_vars (for-loop variables like `for item in items.iter()`)
                // - MethodCall returning &str (e.g., `t.as_str()` returns `&str`)
                //
                // Owned sources (everything else):
                // - FieldAccess (e.g., `m.id` where `m: &Member` → `String`)
                // - Literal values
                // - Method calls returning owned types
                
                let left_is_borrowed = match left {
                    Expression::Identifier { name, .. } => {
                        self.inferred_borrowed_params.contains(name.as_str())
                        || self.borrowed_iterator_vars.contains(name)
                    }
                    Expression::MethodCall { method, .. } => {
                        // Methods like .as_str() return &str (borrowed)
                        method == "as_str"
                    }
                    _ => false,  // FieldAccess, Literal, etc. are owned
                };
                
                let right_is_borrowed = match right {
                    Expression::Identifier { name, .. } => {
                        self.inferred_borrowed_params.contains(name.as_str())
                        || self.borrowed_iterator_vars.contains(name)
                    }
                    Expression::MethodCall { method, .. } => {
                        // Methods like .as_str() return &str (borrowed)
                        method == "as_str"
                    }
                    _ => false,  // FieldAccess, Literal, etc. are owned
                };
                
                // XOR: Add deref only if exactly ONE side is borrowed
                if left_is_borrowed != right_is_borrowed {
                    if left_is_borrowed {
                        // &String == String → *&String == String
                        left_str = format!("*{}", left_str);
                    } else {
                        // String == &String → String == *&String
                        right_str = format!("*{}", right_str);
                    }
                }
                // If both borrowed OR both owned: NO deref needed

                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Unary { op, operand, .. } => {
                let op_str = operators::unary_op_to_rust(op);

                // BORROW CONTEXT: When generating &expr or &mut expr, suppress Vec index
                // auto-clone in the operand. We want a reference to the original element.
                // e.g., &self.items[i] → NOT &self.items[i].clone()
                //        &mut self.items[i] → NOT &mut self.items[i].clone()
                let is_borrow = matches!(
                    op,
                    crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef
                );
                let prev_borrow = self.in_borrow_context;
                if is_borrow {
                    self.in_borrow_context = true;
                }
                let operand_str = self.generate_expression(operand);
                self.in_borrow_context = prev_borrow;

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

                // THE WINDJAMMER WAY: User-defined functions always take priority
                // over built-in name mappings. If the user defines a function with
                // the same name as a test macro or runtime function (e.g., their own
                // `assert_approx`), their definition wins. We check the signature
                // registry: if the function exists and is NOT extern, it's user-defined.
                let is_user_defined = self
                    .signature_registry
                    .get_signature(&func_name)
                    .map(|sig| !sig.is_extern)
                    .unwrap_or(false);

                if !is_user_defined {
                    // Special case: convert test assertion functions to macros
                    // THE WINDJAMMER WAY: assert_eq(a, b) -> assert_eq!(a, b)
                    // NOTE: assert_gt, assert_gte, assert_is_some, assert_is_none, etc. are runtime functions, not macros
                    // Print functions need special handling (format! unwrapping, interpolation)
                    // so they are NOT in the simple macro list — handled separately below.
                    let test_macros = [
                        "assert",
                        "assert_eq",
                        "assert_ne",
                        "assert_ok",
                        "assert_err",
                        "panic",
                        "vec",
                        "format",
                        "write",
                        "writeln",
                        "dbg",
                        "todo",
                        "unimplemented",
                        "unreachable",
                    ];

                    if test_macros.contains(&func_name.as_str()) {
                        let args: Vec<String> = arguments
                            .iter()
                            .map(|(_label, arg)| self.generate_expression(arg))
                            .collect();
                        return format!("{}!({})", func_name, args.join(", "));
                    }

                    // Special case: qualify test assertion runtime functions
                    // THE WINDJAMMER WAY: These are functions, not macros, so they need proper paths
                    let test_functions = [
                        "assert_gt",
                        "assert_lt",
                        "assert_gte",
                        "assert_lte",
                        "assert_approx",
                        "assert_not_empty",
                        "assert_empty",
                        "assert_contains",
                        "assert_is_some",
                        "assert_is_none",
                    ];

                    if test_functions.contains(&func_name.as_str()) {
                        let args: Vec<String> = arguments
                            .iter()
                            .enumerate()
                            .map(|(idx, (_label, arg))| {
                                let generated = self.generate_expression(arg);
                                // assert_is_some and assert_is_none expect &Option, so add & for first arg
                                if (func_name == "assert_is_some" || func_name == "assert_is_none")
                                    && idx == 0
                                {
                                    format!("&{}", generated)
                                } else {
                                    generated
                                }
                            })
                            .collect();
                        return format!(
                            "windjammer_runtime::test::{}({})",
                            func_name,
                            args.join(", ")
                        );
                    }
                }

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
                        // first_arg is &&Expression (ref to ref from Vec element), deref both
                        if let Expression::MacroInvocation {
                            is_repeat: _,
                            ref name,
                            args: ref macro_args,
                            ..
                        } = **first_arg
                        {
                            if name == "format" && !macro_args.is_empty() {
                                // Unwrap the format! call and put its arguments directly into println!
                                // format!("text {}", var) -> println!("text {}", var)
                                let format_str = self.generate_expression(macro_args[0]);
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
                        } = **first_arg
                        {
                            // Check if this is string concatenation
                            let has_string_literal =
                                matches!(
                                    left,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || matches!(
                                    right,
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
                            value: Literal::String(ref s),
                            ..
                        } = **first_arg
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
                    // TDD FIX: Auto-format non-string arguments
                    // println(value) where value: bool → println!("{}", value)
                    // println("text") → println!("text") (string literals stay as-is)
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    
                    // Check if first argument is a string literal
                    let first_arg_is_string_literal = arguments.first()
                        .map(|(_, arg)| matches!(arg, Expression::Literal { value: Literal::String(_), .. }))
                        .unwrap_or(false);
                    
                    if args.len() == 1 && !first_arg_is_string_literal {
                        // Single non-string argument - format it
                        return format!("{}!(\"{{}}\", {})", target_macro, args[0]);
                    } else {
                        // Multiple args or string literal - keep as-is
                        return format!("{}!({})", target_macro, args.join(", "));
                    }
                }

                // Special case: convert assert() to assert!()
                if func_name == "assert" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("assert!({})", args.join(", "));
                }

                // TDD FIX: Call(FieldAccess) → method call WITH SIGNATURE LOOKUP
                // When the parser produces Call { function: FieldAccess { object, field }, args }
                // instead of MethodCall { object, method, args }, we need to:
                // 1. Handle it as a method call (not function call)
                // 2. Do signature lookup to get parameter ownership info
                // 3. Apply correct ownership conversions (& vs .clone() etc.)
                // 
                // This was the AUTO-CLONE BUG: method calls skipped signature lookup!
                if let Expression::FieldAccess {
                    object: call_obj,
                    field: call_method,
                    ..
                } = &**function
                {
                    // DOUBLE-CLONE FIX: When the method is .clone(), suppress auto-clone on
                    // the object to prevent .clone().clone(). Same as MethodCall handler.
                    let prev_explicit_clone = self.in_explicit_clone_call;
                    if call_method == "clone" {
                        self.in_explicit_clone_call = true;
                    }
                    let mut obj_str = self.generate_expression(call_obj);
                    self.in_explicit_clone_call = prev_explicit_clone;
                    // DOUBLE-CLONE SAFETY NET: Strip redundant auto-clone from object
                    if call_method == "clone" && obj_str.ends_with(".clone()") {
                        obj_str = obj_str[..obj_str.len() - 8].to_string();
                    }
                    
                    // TDD FIX: Lookup method signature for ownership inference
                    // Try multiple lookup strategies:
                    // 1. Type::method (if we can infer object type)
                    // 2. method (simple name fallback)
                    let method_signature = self.signature_registry.get_signature(call_method).cloned();

                    // Generate arguments with ownership awareness (same logic as regular Call)
                    let args: Vec<String> = if let Some(ref sig) = method_signature {
                        arguments
                            .iter()
                            .enumerate()
                            .flat_map(|(i, (_label, arg))| {
                                let mut arg_str = self.generate_expression(arg);
                                
                                // Apply ownership conversion based on signature
                                if let Some(&ownership) = sig.param_ownership.get(i) {
                                    match ownership {
                                        OwnershipMode::Borrowed => {
                                            // Destination wants borrowed - add & if needed
                                            let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                                            if !is_string_literal && !arg_str.starts_with("&") {
                                                arg_str = format!("&{}", arg_str);
                                            }
                                        }
                                        OwnershipMode::Owned => {
                                            // Destination wants owned - add .clone() for borrowed sources
                                            if let Expression::FieldAccess { object: field_obj, .. } = arg {
                                                if let Expression::Identifier { name, .. } = &**field_obj {
                                                    let is_borrowed = self.borrowed_iterator_vars.contains(name)
                                                        || self.inferred_borrowed_params.contains(name);
                                                    if is_borrowed && !arg_str.ends_with(".clone()") {
                                                        let is_copy = self.infer_expression_type(arg)
                                                            .as_ref()
                                                            .is_some_and(|t| self.is_type_copy(t));
                                                        if !is_copy {
                                                            arg_str = format!("{}.clone()", arg_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                
                                vec![arg_str]
                            })
                            .collect()
                    } else {
                        // No signature - just generate args without ownership hints
                        arguments
                            .iter()
                            .map(|(_label, arg)| self.generate_expression(arg))
                            .collect()
                    };
                    
                    return format!("{}.{}({})", obj_str, call_method, args.join(", "));
                }

                let mut func_str = self.generate_expression(function);
                
                // In an impl block, bare function calls to sibling methods need qualified dispatch.
                // Instance methods (take self) → self.method(args)
                // Static methods → Self::method(args)
                if self.in_impl_block
                    && !func_name.contains("::")
                    && self.current_impl_methods.contains(&func_name)
                {
                    if self.current_impl_instance_methods.contains(&func_name) {
                        func_str = format!("self.{}", func_str);
                    } else {
                        func_str = format!("Self::{}", func_str);
                    }
                }

                // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
                // Some("literal") -> Some("literal".to_string())
                // Ok("literal") -> Ok("literal".to_string())
                // Err("literal") -> Err("literal".to_string())
                // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())
                
                // TDD FIX (Bug #2): Detect ALL enum constructors, not just Some/Ok/Err
                // Pattern: Module::Variant or Enum::Variant (both CamelCase)
                let is_std_enum = matches!(func_name.as_str(), "Some" | "Ok" | "Err");
                let is_custom_enum = func_name.contains("::") && {
                    let parts: Vec<&str> = func_name.split("::").collect();
                    parts.len() == 2 && 
                    parts[0].chars().next().is_some_and(|c| c.is_uppercase()) &&
                    parts[1].chars().next().is_some_and(|c| c.is_uppercase())
                };
                
                if is_std_enum || is_custom_enum {
                    // TDD FIX (Bug #16 completion): Extract format!() to temp variables for enum variants too!
                    let generated_args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    
                    let has_format_arg = generated_args.iter().any(|arg_str| arg_str.contains("format!("));
                    
                    if has_format_arg {
                        // Extract format!() macros to temp variables
                        let mut temp_decls = String::new();
                        let mut temp_counter = 0;
                        let fixed_args: Vec<String> = generated_args
                            .iter()
                            .map(|arg_str| {
                                if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                    // Strip leading & if present
                                    let format_expr = if arg_str.starts_with("&") {
                                        arg_str.strip_prefix("&").unwrap()
                                    } else {
                                        arg_str
                                    };
                                    // Extract to temp var
                                    let temp_name = format!("_temp{}", temp_counter);
                                    temp_counter += 1;
                                    temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));
                                    
                                    // TDD FIX: Don't add & for owned parameters
                                    // Err(format!(...)) should be Err(_temp0), not Err(&_temp0)
                                    // Original arg didn't have &, so pass owned value
                                    if arg_str.starts_with("&") {
                                        format!("&{}", temp_name)
                                    } else {
                                        temp_name
                                    }
                                } else {
                                    arg_str.clone()
                                }
                            })
                            .collect();
                        
                        return format!("{{ {}{}({}) }}", temp_decls, func_str, fixed_args.join(", "));
                    }
                    
                    let args: Vec<String> = generated_args
                        .iter()
                        .enumerate()
                        .map(|(i, arg_str)| {
                            // Get the original argument expression for type checking
                            let arg = &arguments[i].1;
                                    let result = arg_str.clone();
                            
                            // Auto-convert string literals to String for Option/Result wrappers
                            if matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) {
                                format!("{}.to_string()", result)
                            } else if let Expression::Identifier { name, .. } = arg {
                                // BUGFIX: Don't clone if function returns Option<&T>, Option<&mut T>, or Result<&T, E>
                                // When returning Option<&Squad>, Some(squad) should NOT become Some(squad.clone())

                                // Check if return type is Option<&T> or Option<&mut T> (reference inside)
                                let returns_option_ref = match &self.current_function_return_type {
                                    Some(Type::Option(inner_type)) => {
                                        matches!(
                                            **inner_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // Check if return type is Result<&T, E> or Result<&mut T, E>
                                let returns_result_ref = match &self.current_function_return_type {
                                    Some(Type::Result(ok_type, _err_type)) => {
                                        matches!(
                                            **ok_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // AUTO-CLONE: When wrapping a borrowed iterator variable in Some/Ok/Err,
                                // we need to clone it since the wrapper takes ownership
                                // UNLESS we're returning Option<&T>, Option<&mut T>, Result<&T, E>, etc.
                                if !returns_option_ref
                                    && !returns_result_ref
                                    && self.borrowed_iterator_vars.contains(name)
                                    && !result.ends_with(".clone()")
                                {
                                    // Function returns owned, but variable is borrowed - need to clone
                                    format!("{}.clone()", result)
                                } else {
                                    // Function returns reference, or variable not borrowed - don't clone
                                    result
                                }
                            } else {
                                result
                            }
                        })
                        .collect();
                    return format!("{}({})", func_str, args.join(", "));
                }

                // Look up signature and clone it to avoid borrow conflicts
                // THE WINDJAMMER WAY: Try qualified name first, then simple name
                // e.g., "Sound::new" -> try "Sound::new", then "new"
                
                // TDD FIX: Function pointer signature extraction
                // When calling a function pointer parameter (e.g., has_item(arg1, arg2)),
                // extract the signature from the parameter's type instead of the registry
                let signature = if let Some(param) = self.current_function_params.iter().find(|p| p.name == func_name) {
                    // Check if this parameter is a function pointer
                    if let Type::FunctionPointer { params, return_type } = &param.type_ {
                        // TDD FIX: Build signature from function pointer type
                        // CRITICAL: Match the conversion logic in types.rs type_to_rust()!
                        // fn(string, i32) in Windjammer → fn(&String, i32) in Rust
                        // 
                        // Conversion rules (from types.rs lines 148-160):
                        // - Type::String → "&String" → Borrowed
                        // - Type::Custom("string") → "&String" → Borrowed
                        // - Type::Reference(_) → "&T" → Borrowed
                        // - Copy types (Int, Bool, etc.) → owned → Owned
                        // - Everything else → as-is (keep explicit types)
                        let param_ownership: Vec<OwnershipMode> = params.iter().map(|ty| {
                            match ty {
                                // Idiomatic Windjammer: string parameters are borrowed (types.rs:151)
                                Type::String => OwnershipMode::Borrowed,
                                Type::Custom(name) if name == "string" => OwnershipMode::Borrowed,
                                // Explicit references - borrowed (types.rs:154)
                                Type::Reference(_) | Type::MutableReference(_) => OwnershipMode::Borrowed,
                                // Copy types - owned (types.rs:156-157)
                                Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => OwnershipMode::Owned,
                                Type::Custom(name) if matches!(name.as_str(), "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "char" | "usize" | "isize") => OwnershipMode::Owned,
                                // Everything else - keep as-is (types.rs:159)
                                // For non-Copy custom types, default is as-is, which means Owned in this context
                                // (the analyzer will have determined the correct type already)
                                _ => OwnershipMode::Owned,
                            }
                        }).collect();
                        
                        Some(crate::analyzer::FunctionSignature {
                            name: func_name.clone(),
                            param_types: params.clone(),
                            param_ownership,
                            return_type: return_type.as_ref().map(|t| (**t).clone()),
                            return_ownership: OwnershipMode::Owned, // Functions return owned by default
                            has_self_receiver: false,
                            is_extern: false,
                        })
                    } else {
                        // Not a function pointer - try registry
                        self.signature_registry.get_signature(&func_name).cloned()
                    }
                } else {
                    // Not a parameter - try registry lookup
                    self.signature_registry
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
                        })
                };

                // Check if this is an extern function call for FFI str handling
                let is_extern_call = if let Some(ref sig) = signature {
                    sig.is_extern
                } else {
                    false
                };

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (_label, arg))| {
                        // CRITICAL: Reset in_field_access_object for argument generation.
                        // Arguments are independent expressions, NOT part of a field/method/index chain.
                        // Without this, `process_property(prop.name, prop.value).as_str()` would
                        // leak in_field_access_object from the MethodCall handler into prop.name/prop.value,
                        // suppressing necessary .clone() calls.
                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;
                        
                        // TDD FIX: Set call argument context to suppress premature .clone()
                        // The FieldAccess handler normally adds .clone() for borrowed iterator vars,
                        // but in call arguments, we need to let the ownership check below decide
                        let prev_in_call_arg = self.in_call_argument_generation;
                        self.in_call_argument_generation = true;
                        
                        let mut arg_str = self.generate_expression(arg);
                        
                        self.in_call_argument_generation = prev_in_call_arg;
                        self.in_field_access_object = prev_field_access_obj;
                        
                        // WINDJAMMER FFI: Convert string arguments to (*const u8, usize) for extern functions
                        if is_extern_call {
                            if let Some(ref sig) = signature {
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, Type::Custom(name) if name == "str") {
                                        // Expand str to (ptr, len)
                                        // Always use .as_bytes() for consistency (works for both &str and String)
                                        return vec![
                                            format!("{}.as_bytes().as_ptr()", arg_str),
                                            format!("{}.as_bytes().len()", arg_str),
                                        ];
                                    }
                                }
                            }
                        }

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
                                // No signature found — check enum variant registry
                                // WINDJAMMER FIX: Enum variant constructors like GameEvent::ItemPickup("text")
                                // need .to_string() when the variant field is String type
                                if let Some(variant_types) = self.enum_variant_types.get(&func_name) {
                                    // TDD FIX: Check for both Type::String and Type::Custom("String")
                                    variant_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "String")
                                    })
                                } else {
                                    // Fallback heuristic for constructors
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            };

                            if should_convert {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }

                        // Check if this parameter expects a borrow
                        // Skip ownership inference for extern function calls - they have explicit types
                        if let Some(ref sig) = signature {
                            if sig.is_extern {
                                return vec![arg_str];
                            }

                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed => {
                                        // TDD FIX (Bug #12): String literals &str -> &String conversion
                                        // When parameter expects &String and arg is "literal" (&str),
                                        // we need to convert: &"literal".to_string()
                                        // 
                                        // Check if this is a string literal AND parameter expects &String
                                        let is_string_literal = matches!(
                                            arg,
                                            Expression::Literal {
                                                value: Literal::String(_),
                                                ..
                                            }
                                        );
                                        
                                        if is_string_literal {
                                            // Check if parameter type is String (not some other type)
                                            let expects_string = sig.param_types.get(i)
                                                .map(|t| match t {
                                                    Type::String => true,
                                                    Type::Custom(name) if name == "String" || name == "string" => true,
                                                    _ => false,
                                                })
                                                .unwrap_or(false);
                                            
                                            if expects_string {
                                                // Convert &str literal to &String
                                                // "test" -> &"test".to_string()
                                                return vec![format!("&{}.to_string()", arg_str)];
                                            }
                                            // For other types expecting &T, string literal is already &str, don't add &
                                            return vec![arg_str];
                                        }

                                        // TDD FIX: Check if parameter is already a reference type
                                        // If param is &string, don't add another & (would be &&string)
                                        let is_param_already_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::Reference(_)
                                                                | Type::MutableReference(_)
                                                        )
                                                })
                                            } else {
                                                false
                                            };

                                        // TDD FIX: Don't add & for Copy type parameters
                                        // When signature says Borrowed but param type is Copy,
                                        // codegen keeps it as owned (e.g., x: usize not x: &usize)
                                        // So the call site should NOT add &
                                        // BUT: Reference types (&Vec<T>, &[T]) are NOT treated as
                                        // Copy here - if param type is &T, caller still needs &
                                        let is_copy_param = sig.param_types.get(i)
                                            .map(|t| {
                                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                                            })
                                            .unwrap_or(false);

                                        // TDD FIX (Bug #16): Don't add & to temp variables!
                                        // Temp variables (like _temp0) hold OWNED values from format!()
                                        // format!() returns String, not &str, so _temp0 is String
                                        // If we add &, we get &String when we need String
                                        let is_temp_variable = arg_str.starts_with("_temp") 
                                            && arg_str.chars().skip(5).all(|c| c.is_numeric());
                                        
                                        // TDD FIX: IDIOMATIC WINDJAMMER - Strip .clone() if present!
                                        // When destination wants Borrowed, pass &field, NOT &field.clone()
                                        // Example: has_item(ingredient.item_id) with has_item(item_id: string)
                                        // Should generate: has_item(&ingredient.item_id)
                                        // NOT: has_item(&ingredient.item_id.clone())
                                        // The .clone() may have been added by generate_expression for borrowed iterator vars
                                        if arg_str.ends_with(".clone()") {
                                            arg_str = arg_str[..arg_str.len() - 8].to_string();
                                        }

                                        // Insert & if not already a reference and not a string literal and not a temp var
                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_param_already_ref
                                            && !is_copy_param
                                            && !is_temp_variable
                                        {
                                            return vec![format!("&{}", arg_str)];
                                        } else {
                                            return vec![arg_str];
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => {
                                        // TDD FIX: Don't add &mut if arg is already a &mut parameter
                                        // Covers both explicitly declared &mut params AND
                                        // params inferred as &mut through ownership analysis
                                        let is_already_mut_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                // Check 1: Explicit &mut in AST type
                                                let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::MutableReference(_)
                                                        )
                                                });
                                                // Check 2: Inferred &mut through ownership analysis
                                                let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                explicit_mut_ref || inferred_mut_ref
                                            } else {
                                                false
                                            };

                                        // Insert &mut if not already a reference
                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_already_mut_ref
                                        {
                                            // CRITICAL FIX: Remove .clone() if present - we want to mutate the original!
                                            // &mut counter.clone() → &mut counter
                                            // When passing &mut, we're giving mutable access to the original,
                                            // not a clone. The .clone() would break mutation semantics.
                                            let mut_arg_str = if arg_str.ends_with(".clone()") {
                                                arg_str[..arg_str.len() - 8].to_string()
                                            } else {
                                                arg_str
                                            };
                                            return vec![format!("&mut {}", mut_arg_str)];
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // TDD FIX: AUTO-CONVERT for &str/&String → String, &T → T
                                        // When passing a reference to a function expecting owned, convert it
                                        // - &str → String: use .to_string()
                                        // - &String → String: use .clone()
                                        // - &T → T: use .clone()
                                        if let Expression::Identifier { name, .. } = arg {
                                            // Find the parameter type
                                            let param_type = self
                                                .current_function_params
                                                .iter()
                                                .find(|p| &p.name == name)
                                                .map(|p| &p.type_);

                                            // Check if it's a reference parameter (&str, &String, &T)
                                            if let Some(Type::Reference(inner_type)) = param_type {
                                                // Special case: &str (Type::Reference(Type::String) in Rust parlance)
                                                // &str.clone() → &str, but we need String, so use .to_string()
                                                if matches!(**inner_type, Type::String)
                                                    && !arg_str.ends_with(".to_string()")
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                } else if !arg_str.ends_with(".clone()") {
                                                    // For other reference types, .clone() works
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            } else {
                                                // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                                // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                                // npc_id is &String from iterator, needs .clone() for owned String
                                                // 
                                                // CRITICAL: We're in OwnershipMode::Owned block, which means
                                                // the DESTINATION parameter wants an owned value (String, not &String).
                                                // So it's correct to .clone() borrowed iterator vars.
                                                // 
                                                // This block is fine - it only runs when ownership == Owned
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(name);

                                                // Also check if it's inferred as borrowed
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(name);

                                                if (is_borrowed_iterator_var
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    // Borrowed from iterator or inferred - use .clone()
                                                    // This handles &String → String, &T → T
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            }
                                        }

                                        // TDD FIX: AUTO-CLONE for borrowed_param.field
                                        // When passing ingredient.item_id where ingredient is borrowed,
                                        // we need to clone() IF destination wants Owned.
                                        // 
                                        // We're ALREADY in OwnershipMode::Owned block,
                                        // so destination wants owned. Safe to add .clone().
                                        //
                                        // This handles: for ingredient in &vec { func(ingredient.field) }
                                        // where func(field: String) expects owned.
                                        if let Expression::FieldAccess { .. } = arg {
                                            // Trace through nested field accesses to find the root identifier
                                            // Handles: stack.field, stack.item.id, stack.item.nested.deep
                                            let root_name = self.extract_root_identifier(arg);
                                            if let Some(name) = root_name {
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(&name);
                                                let is_explicitly_borrowed =
                                                    self.current_function_params.iter().any(|p| {
                                                        p.name == name
                                                            && matches!(
                                                                p.ownership,
                                                                crate::parser::OwnershipHint::Ref
                                                            )
                                                    });
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(&name);
                                                
                                                if (is_borrowed_iterator_var 
                                                    || is_explicitly_borrowed 
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    let is_copy = self.infer_expression_type(arg)
                                                        .as_ref()
                                                        .is_some_and(|t| self.is_type_copy(t));
                                                    if !is_copy {
                                                        arg_str = format!("{}.clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // No signature found - don't auto-clone!
                            // Without signature info, we can't know if destination wants Owned or Borrowed
                            // Better to let Rust compiler catch the error than guess wrong
                        }

                        vec![arg_str]
                    })
                    .collect();

                // TDD FIX (Bug #3): Extract format!() macros in arguments to temp variables
                // The args vec has already been generated as Rust strings
                // Check if any contain format!() and extract them
                let has_format_arg = args.iter().any(|arg_str| arg_str.contains("format!("));
                
                // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
                // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
                if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // TDD FIX (Bug #16 COMPLETE): Check if original had & to preserve intent
                                let has_borrow_prefix = arg_str.starts_with("&");
                                // Strip leading & if present
                                let format_expr = if has_borrow_prefix {
                                    &arg_str[1..]
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));
                                
                                // TDD FIX: Only add & if original had it!
                                // format!() returns owned String, so if caller wants owned, pass temp directly
                                // If caller wants borrowed, pass &temp (when original was &format!())
                                if has_borrow_prefix {
                                    format!("&{}", temp_name)
                                } else {
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();
                    
                    let call_expr = format!("{}({})", func_str, fixed_args.join(", "));
                    
                    // Wrap in unsafe block if extern, otherwise regular block
                    if is_extern_call {
                        format!("unsafe {{ {}{}  }}", temp_decls, call_expr)
                    } else {
                        format!("{{ {}{} }}", temp_decls, call_expr)
                    }
                } else {
                    // No format!() args - generate normally with optional unsafe wrapper
                    let call_str = format!("{}({})", func_str, args.join(", "));
                    if is_extern_call {
                        format!("unsafe {{ {} }}", call_str)
                    } else {
                        call_str
                    }
                }
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => {
                // METHOD CALL CONTEXT: Suppress Vec index auto-clone when generating the
                // object of a method call. Methods take &self or &mut self, so Rust allows
                // calling methods on &T returned by Vec indexing without cloning.
                // e.g., self.lights[i].is_enabled() → no need to clone the whole Light2D
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                // DOUBLE-CLONE FIX: When the source has explicit .clone(), suppress auto-clone
                // on the object to prevent .clone().clone(). The explicit clone IS the clone.
                let prev_explicit_clone = self.in_explicit_clone_call;
                if method == "clone" {
                    self.in_explicit_clone_call = true;
                }
                let mut obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.in_explicit_clone_call = prev_explicit_clone;

                // DOUBLE-CLONE SAFETY NET: If the object was auto-cloned by the FieldAccess
                // handler and this IS a .clone() call, strip the redundant auto-clone.
                // e.g., "stack.item.clone()" from auto-clone + ".clone()" from source
                //     → should be "stack.item.clone()", not "stack.item.clone().clone()"
                if method == "clone" && obj_str.ends_with(".clone()") {
                    obj_str = obj_str[..obj_str.len() - 8].to_string();
                }
                
                // TDD FIX: Option::unwrap() move error prevention
                // TDD FIX: AUTO-CLONE Option::unwrap() on borrowed fields
                // When calling .unwrap() on a borrowed Option field, we must clone before unwrap:
                //   node.children.unwrap() where node is &Node → ERROR: cannot move from &Option
                //   node.children.clone().unwrap() → ✅ OK
                // THE WINDJAMMER WAY: Users write .unwrap() naturally, compiler handles ownership
                if method == "unwrap" {
                    // Check if object is a field access (node.children) that needs clone
                    let needs_clone = if let Expression::FieldAccess { object: field_obj, .. } = object {
                        // Is this accessing a field on a borrowed parameter?
                        if let Expression::Identifier { ref name, .. } = **field_obj {
                            // Check if the identifier is an inferred borrowed parameter
                            self.inferred_borrowed_params.contains(name)
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    
                    if needs_clone && !obj_str.contains(".clone()") {
                        obj_str = format!("{}.clone()", obj_str);
                    }
                }
                // BUG #8 FIX: Look up method signature with qualified name (Type::method)
                // First try to infer the type from the object expression
                let type_name = self.infer_type_name(object);
                let method_signature = if let Some(type_name) = type_name {
                    let qualified_name = format!("{}::{}", type_name, method);
                    self.signature_registry
                        .get_signature(&qualified_name)
                        .cloned()
                    // CRITICAL: Do NOT fall back to unqualified method name lookup!
                    // Unqualified lookup for common names like "get", "remove", "contains"
                    // can match WRONG user-defined methods (e.g., ComponentArray::get when
                    // we want HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                    // When the qualified name isn't found, method_signature stays None and
                    // the stdlib heuristics in should_add_ref handle common patterns correctly.
                } else {
                    // No type info available - only look up methods that are unlikely to
                    // conflict with stdlib methods (i.e., not "get", "remove", "contains_key" etc.)
                    let is_common_stdlib_name = matches!(
                        method.as_str(),
                        "get"
                            | "get_mut"
                            | "remove"
                            | "contains_key"
                            | "contains"
                            | "insert"
                            | "push"
                            | "pop"
                            | "len"
                            | "is_empty"
                            | "iter"
                            | "keys"
                            | "values"
                            | "first"
                            | "last"
                            | "clear"
                            | "binary_search"
                            | "starts_with"
                            | "ends_with"
                    );
                    if is_common_stdlib_name {
                        None // Use stdlib heuristics instead of potentially wrong signature
                    } else {
                        self.signature_registry.get_signature(method).cloned()
                    }
                };

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                        // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                        // Fix: Suppress clone when param expects Borrowed -> just add & to field
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_expects_borrowed = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Borrowed));
                        
                        let prev_suppress = self.suppress_borrowed_clone;
                        if param_expects_borrowed && matches!(arg, Expression::FieldAccess { .. }) {
                            self.suppress_borrowed_clone = true;
                        }
                        
                        // CRITICAL: Reset in_field_access_object for method argument generation.
                        // Same rationale as function call arguments — method arguments are
                        // independent expressions, not part of a field/method/index chain.
                        // TDD FIX: STRIP explicit &ref when parameter expects owned value.
                        // WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
                        // If the user writes `&object.transform` but the method takes `Transform` (owned),
                        // the compiler strips the & and passes by value (Copy types) or moves.
                        // Example: self.render_transform(&object.transform) → self.render_transform(object.transform)
                        //
                        // TDD FIX: ALSO strip explicit & for HashMap/BTreeMap key methods with &String arguments.
                        // HashMap<String, V>.contains_key() expects &str, not &&String.
                        // User writes: map.contains_key(&key) where key is inferred as &String
                        // Compiler generates: map.contains_key(key) which auto-derefs &String to &str ✅
                        let arg_to_generate = if let Expression::Unary {
                            op: crate::parser::UnaryOp::Ref,
                            operand,
                            ..
                        } = arg
                        {
                            // Check if this is a HashMap/BTreeMap key method with a borrowed String argument
                            let is_hashmap_key_method = matches!(
                                method.as_str(),
                                "contains_key" | "get" | "get_mut" | "remove" | "get_key_value"
                            ) && i == 0; // Key is always first argument
                            
                            if is_hashmap_key_method {
                                // Check if the operand is a borrowed String parameter
                                if let Expression::Identifier { name, .. } = &**operand {
                                    let is_string_type = |t: &Type| {
                                        matches!(t, Type::String)
                                            || matches!(t, Type::Custom(s) if s == "String" || s == "string")
                                    };
                                    let is_borrowed_string = self.inferred_borrowed_params.contains(name)
                                        && self.current_function_params.iter().any(|param| {
                                            &param.name == name && is_string_type(&param.type_)
                                        });
                                    if is_borrowed_string {
                                        operand // Strip & — &String auto-derefs to &str
                                    } else {
                                        arg // Keep as-is
                                    }
                                } else {
                                    arg // Not an identifier — keep as-is
                                }
                            } else if let Some(ref sig) = method_signature {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                                let param_is_owned = sig
                                    .param_ownership
                                    .get(sig_param_idx)
                                    .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned));
                                if param_is_owned {
                                    operand // Strip & — generate the inner expression
                                } else {
                                    arg // Keep the & — parameter expects a reference
                                }
                            } else {
                                arg // No signature info — keep as-is
                            }
                        } else {
                            arg // Not a & expression — keep as-is
                        };

                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;
                        let mut arg_str = self.generate_expression(arg_to_generate);
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                        // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                        // but bare function pointers fn(&T) -> bool don't auto-deref.
                        // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                        // compiler generates `filter(|__e| predicate(__e))`.
                        if i == 0
                            && matches!(
                                method.as_str(),
                                "filter" | "any" | "all" | "find" | "find_map" | "position"
                                    | "take_while" | "skip_while" | "map_while" | "partition"
                                    | "rposition"
                            )
                            && matches!(arg, Expression::Identifier { .. })
                        {
                            // Bare identifier (function pointer) passed to iterator adapter -
                            // wrap in closure so Rust's auto-deref handles &&T -> &T.
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }

                        // TDD FIX: String literal ownership conversion
                        // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                        // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                        let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                        let string_literal_converted = if is_string_literal {
                            // Check what the parameter wants
                            let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                            let param_ownership = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_ownership.get(sig_param_idx));
                            
                            // CRITICAL: Check if parameter is explicitly &str (not inferred &String)
                            // Explicit &str parameters should NOT get .to_string() conversion
                            let param_type = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_types.get(sig_param_idx));
                            let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
                                matches!(**inner, Type::String) || 
                                matches!(**inner, Type::Custom(ref s) if s == "str")
                            } else {
                                false
                            };
                            
                            if is_explicit_str_ref {
                                // Explicit &str parameter - no conversion needed
                                false
                            } else {
                                match param_ownership {
                                    Some(&OwnershipMode::Owned) => {
                                        // Parameter wants owned String → add .to_string()
                                        arg_str = format!("{}.to_string()", arg_str);
                                        true // Mark that we converted
                                    }
                                    Some(&OwnershipMode::Borrowed) => {
                                        // Parameter wants &String (inferred) → add &.to_string()
                                        arg_str = format!("&{}.to_string()", arg_str);
                                        true // Mark that we converted
                                    }
                                    _ => {
                                        // No signature info - use heuristic (fallback to old logic)
                                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, &method_signature) {
                                            arg_str = format!("{}.to_string()", arg_str);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                }
                            }
                        } else {
                            false
                        };

                        // TDD FIX: AUTO-CONVERT &str/&String → String for method calls
                        // When passing a &str parameter to a method expecting owned String, convert it
                        // This handles cases like: recipe.add_ingredient("herb", 1) where add_ingredient expects String
                        if let Expression::Identifier { name, .. } = arg {
                            // Find the parameter type
                            let param_type = self.current_function_params.iter()
                                .find(|p| &p.name == name)
                                .map(|p| &p.type_);

                            // Check if parameter type is &str (Type::Reference(Type::String))
                            if let Some(Type::Reference(inner_type)) = param_type {
                                if matches!(**inner_type, Type::String) {
                                    // Check if method signature expects owned String for this parameter
                                    let expects_owned = method_signature
                                        .as_ref()
                                        .and_then(|sig| sig.param_ownership.get(i))
                                        .is_some_and(|&ownership| matches!(ownership, OwnershipMode::Owned));

                                    if expects_owned && !arg_str.ends_with(".to_string()") && !arg_str.ends_with(".clone()") {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }
                            }
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
                            &self.current_function_return_type,
                        ) {
                            arg_str = format!("{}.clone()", arg_str);
                        }

                        // TDD FIX: Strip unnecessary .clone() when method param is Borrowed
                        // When a field like `ingredient.item_id` is auto-cloned by the
                        // FieldAccess handler (because owner is borrowed), but the method
                        // expects &String (Borrowed), the clone is wasteful:
                        //   &ingredient.item_id.clone()  ← clones then borrows (wasteful)
                        //   &ingredient.item_id          ← borrows directly (correct)
                        // Strip the .clone() so should_add_ref can add & cleanly.
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                            if param_is_borrowed && arg_str.ends_with(".clone()") {
                                arg_str = arg_str[..arg_str.len() - 8].to_string();
                            }
                        }

                        // AUTO-REF: Add & when parameter expects reference but arg is owned
                        // TDD FIX: Don't add & if we already handled string literal conversion above
                        if !string_literal_converted {
                            let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                arg,
                                &arg_str,
                                method,
                                i,
                                &method_signature,
                                &self.usize_variables,
                                &self.current_function_params,
                                &self.borrowed_iterator_vars,
                                &self.inferred_borrowed_params,
                                arguments.len(),
                            );
                            if should_ref {
                                arg_str = format!("&{}", arg_str);
                            }
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

                        // Restore suppress flag
                        self.suppress_borrowed_clone = prev_suppress;
                        
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
                            // NOTE: "json" removed - it's a common variable name!
                            // Use "serde_json" for the module instead
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
                        match object {
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

                // WINDJAMMER STDLIB → RUST TRANSLATION
                // Some Windjammer methods don't exist in Rust and need translation.
                //
                // reversed() → into_iter().rev().collect::<Vec<_>>()
                if method == "reversed" && processed_args.is_empty() {
                    return format!("{}.into_iter().rev().collect::<Vec<_>>()", obj_str);
                }
                // enumerate() → iter().enumerate()
                // Rust Vec doesn't have .enumerate() — only iterators do.
                // But if the object already ends with .iter(), .iter_mut(), or
                // .into_iter(), don't add a redundant .iter() prefix.
                if method == "enumerate" && processed_args.is_empty() {
                    let already_iterator = obj_str.ends_with(".iter()")
                        || obj_str.ends_with(".iter_mut()")
                        || obj_str.ends_with(".into_iter()");
                    if already_iterator {
                        return format!("{}.enumerate()", obj_str);
                    } else {
                        return format!("{}.iter().enumerate()", obj_str);
                    }
                }

                // TDD FIX (Bug #3): Extract format!() macros in method arguments too
                let has_format_arg = processed_args.iter().any(|arg_str| arg_str.contains("format!("));
                
                let base_expr = if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = processed_args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // Strip leading & if present (was added by argument processing)
                                let format_expr = if arg_str.starts_with("&") {
                                    arg_str.strip_prefix("&").unwrap()
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));
                                
                                // TDD FIX (Bug #16): Don't always add & - format!() returns owned String
                                // If the parameter expects &str, Rust's coercion handles it automatically
                                // If the parameter expects String, we need the owned value
                                // Check if original arg_str had & to preserve caller's intent
                                if arg_str.starts_with("&") {
                                    // Original code had &format!() → keep the &
                                    format!("&{}", temp_name)
                                } else {
                                    // Original code had format!() → pass owned value
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();
                    
                    // Wrap in block: { let _temp0 = format!(...); obj.method(&_temp0, ...) }
                    format!(
                        "{{ {}{}{}{}{}({}) }}",
                        temp_decls,
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        fixed_args.join(", ")
                    )
                } else {
                    format!(
                        "{}{}{}{}({})",
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        processed_args.join(", ")
                    )
                };

                // AUTO-CLONE: Method call results are ALWAYS owned values.
                // Unlike field accesses (self.field borrows from self) or identifiers
                // (which may be borrowed), calling a method produces a fresh value.
                // The auto-clone analysis may flag the *object* for cloning, but that
                // doesn't mean the *result of the method call* needs cloning.
                //
                // Exception: methods that return references (get, first, last) are
                // handled separately by should_add_cloned().
                //
                // WINDJAMMER PHILOSOPHY: Only clone when semantically necessary.
                // Method call results are never borrowed — cloning them is pure noise.
                base_expr
            }
            Expression::FieldAccess { object, field, .. } => {
                // FIELD CHAIN OPTIMIZATION: If we're accessing a likely-Copy sub-field
                // (e.g., .x, .y, .width, .speed), suppress borrowed-iterator cloning
                // on the intermediate object. In Rust, (&enemy).velocity.y works fine
                // through auto-deref — no need to clone the intermediate Vec2.
                let field_is_likely_copy = matches!(
                    field.as_str(),
                    "x" | "y"
                        | "z"
                        | "w"
                        | "width"
                        | "height"
                        | "depth"
                        | "r"
                        | "g"
                        | "b"
                        | "a"
                        | "left"
                        | "right"
                        | "top"
                        | "bottom"
                        | "min"
                        | "max"
                        | "start"
                        | "end"
                        | "offset"
                        | "scale"
                        | "speed"
                        | "time"
                        | "delta"
                        | "angle"
                        | "radius"
                        | "distance"
                        | "visible"
                        | "enabled"
                        | "active"
                        | "selected"
                        | "focused"
                        | "id"
                        | "type"
                        | "kind"
                        | "priority"
                        | "level"
                        | "len"
                        | "count"
                        | "size"
                        | "index"
                        | "idx"
                        | "vx"
                        | "vy"
                        | "vz"
                        | "dx"
                        | "dy"
                        | "dz"
                        | "health"
                        | "damage"
                        | "score"
                        | "lives"
                        | "frame"
                );
                // Also check via type inference if the outer expression (self.obj.field) is Copy
                let field_is_copy_by_type = self
                    .infer_expression_type(expr_to_generate)
                    .as_ref()
                    .is_some_and(|t| self.is_type_copy(t));

                let prev_suppress = self.suppress_borrowed_clone;
                let prev_field_access = self.in_field_access_object;
                if field_is_likely_copy || field_is_copy_by_type {
                    self.suppress_borrowed_clone = true;
                }
                // Suppress Vec index clone when we're just accessing a field
                // e.g., players[i].score → no need to clone the whole Player
                self.in_field_access_object = true;
                let obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.suppress_borrowed_clone = prev_suppress;

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
                // CRITICAL: Never clone assignment targets (left side of `=`)
                // e.g., `emitter.lifetime = 1.0` must NOT become `emitter.clone().lifetime = 1.0`
                // DOUBLE-CLONE FIX: Skip auto-clone when we're inside an explicit .clone() call
                // The source already has .clone(), so we must not add another one.
                if !self.generating_assignment_target && !self.in_explicit_clone_call {
                    if let Some(path) = ast_utilities::extract_field_access_path(expr_to_generate) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                            {
                                // Skip .clone() for Copy types (f32, i32, bool, etc.)
                                // They are implicitly copied — .clone() is unnecessary noise.
                                let is_copy = self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    // Type inference failed — fall back to name heuristic
                                    // Fields like x, y, z, width, height are almost always Copy
                                    let is_likely_copy_field = matches!(
                                        field.as_str(),
                                        "x" | "y"
                                            | "z"
                                            | "w"
                                            | "width"
                                            | "height"
                                            | "depth"
                                            | "r"
                                            | "g"
                                            | "b"
                                            | "a"
                                            | "left"
                                            | "right"
                                            | "top"
                                            | "bottom"
                                            | "min"
                                            | "max"
                                            | "start"
                                            | "end"
                                            | "offset"
                                            | "scale"
                                            | "speed"
                                            | "time"
                                            | "delta"
                                            | "angle"
                                            | "radius"
                                            | "distance"
                                            | "visible"
                                            | "enabled"
                                            | "active"
                                            | "selected"
                                            | "focused"
                                            | "id"
                                            | "type"
                                            | "kind"
                                            | "priority"
                                            | "level"
                                            | "len"
                                            | "count"
                                            | "size"
                                            | "index"
                                            | "idx"
                                            | "vx"
                                            | "vy"
                                            | "vz"
                                            | "dx"
                                            | "dy"
                                            | "dz"
                                            | "health"
                                            | "damage"
                                            | "score"
                                            | "lives"
                                            | "frame"
                                    );
                                    if !is_likely_copy_field {
                                        return format!("{}.clone()", base_expr);
                                    }
                                }
                            }
                        }
                    }
                }

                // BORROWED ITERATOR: If accessing fields through a borrowed iterator variable,
                // we need to clone non-Copy fields since we can't move out of a reference
                // BUT: Don't clone for assignment targets (left side of =)
                // AND: Don't clone when a parent FieldAccess is reading a Copy sub-field
                //      (e.g., bullet.velocity.y → .y is Copy, so no need to clone velocity)
                // AND: Don't clone when inside an explicit .clone() call (prevents double clone)
                // AND: Don't clone when this is an intermediate object in a field access chain
                //      (e.g., stack.item.stats.armor → don't clone item, Rust auto-derefs through &)
                // AND: Don't clone in borrow context (&recipe.ingredients → reference is sufficient)
                // TDD FIX: Don't clone when generating call arguments (Call handler applies ownership)
                // WINDJAMMER PHILOSOPHY: Use type inference first, fall back to name heuristics
                if !self.generating_assignment_target
                    && !self.suppress_borrowed_clone
                    && !self.in_explicit_clone_call
                    && !self.in_field_access_object
                    && !self.in_borrow_context
                    && !self.in_call_argument_generation
                {
                    if let Expression::Identifier { name: var_name, .. } = &**object {
                        if self.borrowed_iterator_vars.contains(var_name) {
                            // First: use type inference to check if the field type is Copy
                            let is_copy = self
                                .infer_expression_type(expr_to_generate)
                                .as_ref()
                                .is_some_and(|t| self.is_type_copy(t));

                            if !is_copy {
                                // Fall back to name-based heuristics for fields we KNOW are Copy
                                let is_likely_copy_field = matches!(
                                    field.as_str(),
                                    "len" | "count" | "size" | "index" | "idx" | "i" | "j" | "k" |
                                    "x" | "y" | "z" | "w" | "width" | "height" | "depth" |
                                    "r" | "g" | "b" | "a" | "left" | "right" | "top" | "bottom" |
                                    "min" | "max" | "start" | "end" | "offset" | "scale" |
                                    "speed" | "time" | "delta" | "angle" | "radius" | "distance" |
                                    "visible" | "enabled" | "active" | "selected" | "focused" |
                                    "id" | "type" | "kind" | "priority" | "level" |
                                    // Method-like names that should NOT be cloned
                                    "as_str" | "to_string" | "clone" | "iter" | "iter_mut" | "is_empty"
                                );
                                if !is_likely_copy_field && !base_expr.ends_with(".clone()") {
                                    return format!("{}.clone()", base_expr);
                                }
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
                        // STRUCT LITERAL CONTEXT: Array literals in struct fields should use
                        // fixed-size [...] syntax, not vec![...], because struct fields have
                        // explicit type annotations (e.g., position: [f32; 3]).
                        let prev_in_struct_field = self.in_struct_literal_field;
                        self.in_struct_literal_field = true;

                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // In Windjammer, `string` type is always owned (maps to Rust String)
                        // So string literals in struct fields should be converted automatically
                        let mut expr_str = self.generate_expression(expr);

                        // Restore previous context
                        self.in_struct_literal_field = prev_in_struct_field;

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

                        // CRITICAL: Auto-convert &str parameters to String for struct fields
                        // Pattern: fn create(name: &str) -> User { User { name: name } }
                        // When struct field is String but parameter is &str, add .to_string()
                        if let Expression::Identifier { name: id, .. } = expr {
                            // Check if this identifier is a &str parameter
                            // In the AST, &str parameters have type Reference(Custom("str"))
                            let is_str_param = self.current_function_params.iter().any(|p| {
                                p.name == *id && matches!(
                                    &p.type_,
                                    crate::parser::Type::Reference(inner) if matches!(**inner, crate::parser::Type::Custom(ref name) if name == "str")
                                )
                            });

                            if is_str_param && !expr_str.contains(".to_string()") {
                                expr_str = format!("{}.to_string()", expr_str);
                            }
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

                        // Check for field shorthand: if expr is just the field name AND no conversion applied, use shorthand
                        // Only use shorthand if the generated expression exactly matches the field name
                        // (no .to_string(), .clone(), etc. conversions)
                        if let Expression::Identifier { name: id, .. } = expr {
                            if id == field_name && expr_str == *field_name {
                                // Shorthand: User { name } instead of User { name: name }
                                // Only safe when no type conversion was needed
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

                // THE WINDJAMMER WAY: Smart `move` inference for closures
                //
                // Add `move` automatically UNLESS the closure captures `self`.
                // Rationale:
                // 1. Simple closures that capture local variables → add `move` (safer, works for threads)
                // 2. Method closures that capture `self` → don't add `move` (UI callbacks need to borrow)
                //
                // This makes Windjammer code simpler while avoiding E0382 errors in UI code.

                // Check if the closure body references `self`
                let captures_self = self.expression_references_self(body);

                if captures_self {
                    // Don't add `move` - closure needs to borrow `self`
                    format!("|{}| {}", params, body_str)
                } else {
                    // Add `move` - closure can safely capture by value
                    format!("move |{}| {}", params, body_str)
                }
            }
            Expression::Index { object, index, .. } => {
                // INDEX CHAIN OPTIMIZATION: When generating the object of an Index expression,
                // suppress auto-clone. In `a[i][j]`, Rust auto-derefs `a[i]` (returns &Vec<T>)
                // to access [j]. Cloning the intermediate Vec is wasteful and wrong.
                // Same logic as in_field_access_object for FieldAccess chains.
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                let obj_str = self.generate_expression(object);
                self.in_field_access_object = prev_field_access;

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
                // 4. Parenthesized cast: arr[(x as int)] -> arr[x as usize]
                // 5. Already usize: don't double-cast
                let final_idx = if idx_str.ends_with("as i64)") || idx_str.ends_with("as int)") {
                    // Replace (... as i64/int) with (... as usize)
                    let base = idx_str
                        .trim_end_matches("as i64)")
                        .trim_end_matches("as int)")
                        .trim()
                        .trim_start_matches('(')
                        .trim();
                    format!("{} as usize", base)
                } else if idx_str.ends_with("as i64") || idx_str.ends_with("as int") {
                    // Replace ... as i64/int with ... as usize
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
                    // Skip cast if identifier is already usize (e.g. assigned from `expr as usize`)
                    if let Expression::Identifier { name, .. } = &**index {
                        if self.usize_variables.contains(name)
                            || self.expression_produces_usize(index)
                        {
                            idx_str // Already usize — no cast needed
                        } else {
                            format!("{} as usize", idx_str)
                        }
                    } else if let Expression::Literal {
                        value: Literal::Int(n),
                        ..
                    } = &**index
                    {
                        // Integer literal: Rust infers type from context in index position,
                        // so `arr[0]` works without `as usize`. Only cast if negative
                        // (which would be a logic error, but preserve the cast for clarity).
                        if *n < 0 {
                            format!("{} as usize", idx_str)
                        } else {
                            idx_str
                        }
                    } else {
                        format!("{} as usize", idx_str)
                    }
                } else {
                    idx_str
                };

                let base_expr = format!("{}[{}]", obj_str, final_idx);

                // WINDJAMMER PHILOSOPHY: Auto-clone Vec indexing for non-Copy types.
                // Rust doesn't allow moving out of a Vec index (E0507).
                // For Copy types: vec[idx] works directly (value is copied).
                // For non-Copy types: vec[idx].clone() is needed.
                //
                // CRITICAL: NEVER auto-clone in these contexts:
                // 1. Assignment target: vec[i] = value (can't assign to .clone())
                // 2. Borrow context: &vec[i] (want reference to original, not to clone)
                // 3. Field access: vec[i].field (Rust allows field access through ref)
                // 4. Comparison context: vec[i] == val (comparisons work on &T)
                let suppress_clone = self.generating_assignment_target
                    || self.in_borrow_context
                    || self.in_field_access_object
                    || self.suppress_borrowed_clone;

                if !suppress_clone {
                    // First check auto_clone_analysis (path-based analysis)
                    if let Some(path) = ast_utilities::extract_field_access_path(expr_to_generate) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                            {
                                let is_copy = self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    return format!("{}.clone()", base_expr);
                                }
                            }
                        }
                    }

                    // Fallback: Type-based auto-clone for Vec<NonCopy>[idx]
                    // If we can infer the collection's element type and it's not Copy, clone.
                    // This handles the common case: vec[i] passed to a function taking ownership.
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        let element_type = match &obj_type {
                            Type::Vec(inner) => Some(inner.as_ref()),
                            Type::Array(inner, _) => Some(inner.as_ref()),
                            _ => None,
                        };
                        if let Some(elem_type) = element_type {
                            if !self.is_type_copy(elem_type) {
                                return format!("{}.clone()", base_expr);
                            }
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

                // WINDJAMMER PHILOSOPHY: Array literal syntax determines Rust output.
                //
                // In WJ, `[a, b, c]` is a fixed-size array literal → generates `[a, b, c]` in Rust.
                // In WJ, `vec![a, b, c]` is an explicit Vec constructor → generates `vec![a, b, c]`.
                //
                // Empty arrays `[]` remain `vec![]` because Rust's empty `[]` can't infer its type.
                //
                // This distinction is critical: `painter.line_segment([p1, p2], stroke)` expects
                // `[Pos2; 2]`, not `Vec<Pos2>`. The developer chose `[...]` syntax intentionally.
                if exprs.is_empty() {
                    // Empty array [] → vec![] (Vec::new())
                    // Rust's [] is a fixed-size array and can't infer type from later usage.
                    "vec![]".to_string()
                } else {
                    // Non-empty array literals: generate fixed-size array [a, b, c]
                    // The developer uses `vec![...]` macro syntax when Vec is needed.
                    format!("[{}]", expr_strs.join(", "))
                }
            }
            Expression::MacroInvocation {
                is_repeat,
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
                        is_repeat: _,
                        args: format_args,
                        ..
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
                        vec!["\"{}\"".to_string(), self.generate_expression(args[0])]
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                };

                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };

                // WINDJAMMER FIX: vec![value; count] repeat syntax
                // The parser sets is_repeat=true for vec![x; n] syntax
                // Use semicolon for repeat, comma for regular args
                let separator = if *is_repeat { "; " } else { ", " };

                // WINDJAMMER FIX: String literal coercion in vec![]
                // In Windjammer, `string` maps to Rust `String`, so vec!["a", "b"] must
                // become vec!["a".to_string(), "b".to_string()] for Vec<String>.
                // Only apply when: macro is vec, brackets delimiter, has string literal args.
                let final_arg_strs: Vec<String> = if name == "vec"
                    && matches!(delimiter, MacroDelimiter::Brackets)
                    && !*is_repeat
                {
                    arg_strs
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| {
                            // Check if the original arg is a string literal
                            if idx < args.len() {
                                if let Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } = &args[idx]
                                {
                                    // Add .to_string() if not already present
                                    if !s.ends_with(".to_string()") {
                                        return format!("{}.to_string()", s);
                                    }
                                }
                            }
                            s.clone()
                        })
                        .collect()
                } else {
                    arg_strs
                };

                format!(
                    "{}!{}{}{}",
                    name,
                    open,
                    final_arg_strs.join(separator),
                    close
                )
            }
            Expression::Cast { expr, type_, .. } => {
                // Add parentheses around binary expressions for correct precedence
                // because `as` has higher precedence than arithmetic in Rust:
                // `a + b as usize` is parsed as `a + (b as usize)`, not `(a + b) as usize`
                let expr_str = match &**expr {
                    Expression::Binary { .. } => {
                        format!("({})", self.generate_expression(expr))
                    }
                    _ => self.generate_expression(expr),
                };
                let type_str = self.type_to_rust(type_);
                // TDD FIX: Do NOT wrap cast in outer parentheses.
                // `as` has higher precedence than comparison/arithmetic operators in Rust,
                // so `x as usize >= y` correctly parses as `(x as usize) >= y`.
                // Outer parens are ONLY needed when the cast is followed by `.method()`
                // or `.field` (handled at the MethodCall/FieldAccess generation sites).
                format!("{} as {}", expr_str, type_str)
            }
            Expression::Block {
                statements: stmts, ..
            } => {
                // Special case: if the block contains only a match statement, generate it as a match expression
                // BUT: Skip this optimization when the match is an if-let pattern (2 arms, last is wildcard with empty body)
                // In that case, fall through to normal block generation which will generate `if let` via Statement::Match handler
                if stmts.len() == 1 {
                    if let Statement::Match { value, arms, .. } = &stmts[0] {
                        // Check if this is an if-let pattern that should be generated as `if let`
                        let is_if_let_pattern = arms.len() == 2
                            && matches!(arms[1].pattern, Pattern::Wildcard)
                            && arms[1].guard.is_none()
                            && matches!(arms[1].body, Expression::Block { statements, .. } if statements.is_empty());

                        if is_if_let_pattern {
                            // Fall through to normal block generation — generate_statement will emit `if let`
                            let mut output = String::from("{\n");
                            self.indent_level += 1;
                            for stmt in stmts {
                                output.push_str(&self.generate_statement(stmt));
                            }
                            self.indent_level -= 1;
                            output.push_str(&self.indent());
                            output.push('}');
                            return output;
                        }

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
                                        string_analysis::expression_produces_string(arm.body)
                                            || arm_string_analysis::arm_returns_converted_string(
                                                arm.body,
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
                                let body_str = self.generate_expression(arm.body);
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
                        // Last statement is an expression, thread/async block - generate as implicit return
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

                                // TDD FIX: In statement-context matches, add semicolons to all statements
                                // even if they're the last expression (match arms that return void)
                                if self.in_statement_match {
                                    output.push_str(";\n");
                                } else {
                                    output.push('\n');
                                }
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
                    } else if !is_last {
                        // TDD FIX: Non-last statements in a block expression ALWAYS need
                        // semicolons, even in expression context (e.g., match arm body
                        // inside `let _ = match ... { Arm => { expr1; expr2 } }`).
                        let old_expr_ctx = self.in_expression_context;
                        self.in_expression_context = false;
                        output.push_str(&self.generate_statement(stmt));
                        self.in_expression_context = old_expr_ctx;
                    } else {
                        // Last statement of a non-Expression type (e.g., Statement::If used as block value):
                        // Preserve in_expression_context so inner branches retain correct semicolon behavior
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
    fn generate_string_concat(
        &mut self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
    ) -> String {
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

    /// WINDJAMMER LIFETIME INFERENCE: Determine if a function needs explicit lifetime annotations.
    ///
    /// Rust's lifetime elision rules handle most cases:
    ///   1. Single input reference → output gets that lifetime
    ///   2. &self/&mut self → output gets self's lifetime
    ///   3. Multiple input references with no self → MUST be explicit
    ///
    /// We only add 'a when case 3 applies AND the return type contains references.
    fn function_needs_lifetime_annotations(
        &self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> bool {
        use crate::codegen::rust::types::type_contains_reference;

        // First check: does the return type contain any references?
        let return_has_ref = match &func.return_type {
            Some(ret_type) => type_contains_reference(ret_type),
            None => false,
        };

        if !return_has_ref {
            return false;
        }

        // Check if there's a self parameter (explicit or inferred)
        let has_self = func.parameters.iter().any(|p| p.name == "self")
            || analyzed.inferred_ownership.contains_key("self");

        if has_self {
            // &self/&mut self methods: Rust elision rule 2 handles this
            return false;
        }

        // Count the number of reference parameters (explicit refs + analyzer-inferred refs)
        let ref_param_count = func
            .parameters
            .iter()
            .enumerate()
            .filter(|(param_idx, param)| {
                if param.name == "self" {
                    return false;
                }

                // Check if the parameter type is already a reference
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(*param_idx)
                    .unwrap_or(&param.type_);

                if matches!(
                    inferred_type,
                    Type::Reference(_) | Type::MutableReference(_)
                ) {
                    return true;
                }

                // Check explicit ownership hints
                if matches!(
                    param.ownership,
                    crate::parser::OwnershipHint::Ref | crate::parser::OwnershipHint::Mut
                ) {
                    return true;
                }

                // Check analyzer-inferred ownership
                if let Some(ownership) = analyzed.inferred_ownership.get(&param.name) {
                    matches!(
                        ownership,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    )
                } else {
                    false
                }
            })
            .count();

        // Need explicit lifetime when 2+ reference params and reference return
        ref_param_count >= 2
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

    // Format type parameters with trait bounds for Rust output
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
}
