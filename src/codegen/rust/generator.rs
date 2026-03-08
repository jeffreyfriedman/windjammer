#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Rust code generator
use crate::analyzer::*;
use crate::codegen::rust::expression_helpers;
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
    needs_serde_imports: bool,           // For JSON support
    pub(crate) needs_write_import: bool, // For string capacity optimization (write! macro)
    needs_smallvec_import: bool,         // For Phase 8 SmallVec optimization
    pub(crate) needs_cow_import: bool,   // For Phase 9 Cow optimization
    needs_hashmap_import: bool,          // Auto-detect HashMap usage
    needs_hashset_import: bool,          // Auto-detect HashSet usage
    pub(crate) target: CompilationTarget,
    pub(crate) is_module: bool, // true if generating code for a reusable module (not main file)
    source_map: crate::source_map::SourceMap,
    pub(crate) current_output_file: std::path::PathBuf, // Path to the Rust file being generated
    current_rust_line: usize, // Current line number in generated Rust code (1-indexed)
    pub(crate) current_wj_file: std::path::PathBuf, // Path to the Windjammer file being compiled
    pub(crate) inferred_bounds: std::collections::HashMap<String, crate::inference::InferredBounds>,
    pub(crate) needs_trait_imports: std::collections::HashSet<String>, // Tracks which traits need imports
    bound_aliases: std::collections::HashMap<String, Vec<String>>,     // bound Name = Trait + Trait
    // PHASE 2 OPTIMIZATION: Track variables that can avoid cloning
    pub(crate) clone_optimizations: std::collections::HashSet<String>, // Variables that don't need .clone()
    // PHASE 3 OPTIMIZATION: Track struct mapping optimizations
    pub(crate) struct_mapping_hints:
        std::collections::HashMap<String, crate::analyzer::MappingStrategy>, // Struct name -> strategy
    // PHASE 4 OPTIMIZATION: Track string operation optimizations
    pub(crate) string_capacity_hints: std::collections::HashMap<usize, usize>, // Statement idx -> capacity
    // PHASE 5 OPTIMIZATION: Track assignment operations that can use compound operators
    pub(crate) assignment_optimizations:
        std::collections::HashMap<String, crate::analyzer::CompoundOp>, // Variable -> compound op
    // PHASE 6 OPTIMIZATION: Track defer drop optimizations
    pub(crate) defer_drop_optimizations: Vec<crate::analyzer::DeferDropOptimization>,
    // PHASE 8 OPTIMIZATION: Track SmallVec optimizations
    pub(crate) smallvec_optimizations:
        std::collections::HashMap<String, crate::analyzer::SmallVecOptimization>, // Variable -> SmallVec config
    // PHASE 9 OPTIMIZATION: Track Cow optimizations
    pub(crate) cow_optimizations: std::collections::HashSet<String>, // Variables that can use Cow
    // AUTO-CLONE: Track where to automatically insert clones
    pub(crate) auto_clone_analysis: Option<crate::auto_clone::AutoCloneAnalysis>,
    // Track current statement index for optimization hints
    pub(crate) current_statement_idx: usize,
    // IMPLICIT SELF SUPPORT: Track struct fields for implicit self references
    pub(crate) current_struct_fields: std::collections::HashSet<String>, // Field names in current impl block
    pub(crate) current_struct_name: Option<String>, // Name of struct in current impl block
    pub(crate) current_impl_methods: std::collections::HashSet<String>, // Method names in current impl block
    pub(crate) current_impl_instance_methods: std::collections::HashSet<String>, // Methods that take self
    pub(crate) in_impl_block: bool, // true if currently generating code for an impl block
    // USIZE DETECTION: Track which struct fields have type usize (for auto-casting)
    pub(crate) usize_struct_fields:
        std::collections::HashMap<String, std::collections::HashSet<String>>, // Struct name -> usize field names
    // METHOD RETURN TYPES: Track which methods return usize (for auto-casting in comparisons)
    // Maps method name -> return type. Used by infer_expression_type for MethodCall.
    pub(crate) method_return_types: std::collections::HashMap<String, Type>,
    // FUNCTION CONTEXT: Track current function parameters for compound assignment optimization
    pub(crate) current_function_params: Vec<crate::parser::Parameter<'ast>>, // Parameters of the current function
    // FUNCTION CONTEXT: Track current function return type for string literal conversion
    pub(crate) current_function_return_type: Option<Type>,
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
    pub(crate) suppress_string_conversion: bool,
    // LOCAL VARIABLE TRACKING: Stack of scopes, each scope contains local variable names
    // Enables proper variable shadowing of field names
    pub(crate) local_variable_scopes: Vec<std::collections::HashSet<String>>,
    // EXPRESSION CONTEXT: Track if we're generating code whose value will be used
    // Prevents adding semicolons to final expressions in if-else/match when used as values
    pub(crate) in_expression_context: bool,
    // TDD: Track if we're generating the top-level function body (enables return optimization)
    pub(crate) in_function_body: bool,
    // TDD: Track if the current statement being generated is the last in its block
    pub(crate) current_is_last_statement: bool,
    // TRAIT TRACKING: Track which custom types support PartialEq
    pub(crate) partial_eq_types: std::collections::HashSet<String>,
    // MATCH ARM CONTEXT: Force string conversion in match arm blocks
    pub(crate) in_match_arm_needing_string: bool,
    // MATCH STATEMENT CONTEXT: Track if we're in a match used as a statement (not expression)
    // In statement-context matches, arm blocks should have semicolons on all statements
    pub(crate) in_statement_match: bool,
    // FOR-LOOP AUTO-BORROW: Track local variables that need `&` in for-loops
    // because they are used after the loop (pre-computed per function body)
    pub(crate) for_loop_borrow_needed: std::collections::HashSet<String>,
    // BORROWED ITERATOR VARIABLES: Track variables that are iterating over borrowed collections
    // These variables are references, so accessing their fields requires .clone()
    pub(crate) borrowed_iterator_vars: std::collections::HashSet<String>,
    // OWNED STRING ITERATOR VARIABLES: Track variables from for-loops over Vec<String>
    // These need to be borrowed when used in String += operations
    pub(crate) owned_string_iterator_vars: std::collections::HashSet<String>,
    // USIZE VARIABLES: Track variables assigned from .len() for auto-casting
    pub(crate) usize_variables: std::collections::HashSet<String>,
    // UNUSED LET BINDINGS: Track let bindings whose variable is never used after declaration.
    // Keyed by (line, column) of the let statement's source location.
    // These will be prefixed with `_` in the generated Rust to suppress "unused variable" warnings.
    pub(crate) unused_let_bindings: std::collections::HashSet<(usize, usize)>,
    // INFERRED BORROWED PARAMS: Parameters inferred to be borrowed (for field access cloning)
    pub(crate) inferred_borrowed_params: std::collections::HashSet<String>,
    // INFERRED MUT BORROWED PARAMS: Parameters inferred to be &mut (for avoiding double &mut in passthrough)
    pub(crate) inferred_mut_borrowed_params: std::collections::HashSet<String>,
    // USER-WRITTEN CLOSURE: When true, suppress auto-borrowing transformations (preserve user intent)
    pub(crate) in_user_written_closure: bool,
    // USER CLOSURE PARAMS: Track parameters of current user-written closure
    pub(crate) user_closure_params: std::collections::HashSet<String>,
    // ASSIGNMENT TARGET: Flag to suppress auto-clone when generating assignment targets
    pub(crate) generating_assignment_target: bool,
    // VOID BLOCK: When true, last expression in a block gets a semicolon (if-without-else bodies)
    pub(crate) in_void_block: bool,
    // EXPLICIT CLONE SUPPRESSION: When the source has `.clone()` (MethodCall with method "clone"),
    // suppress auto-clone on the object expression to prevent double .clone().clone()
    pub(crate) in_explicit_clone_call: bool,
    // FIELD CHAIN OPTIMIZATION: When accessing a Copy sub-field (e.g., .y on Vec2),
    // suppress borrowed-iterator cloning on the intermediate object.
    // e.g., enemy.velocity.y → no need to clone velocity just to read .y
    pub(crate) suppress_borrowed_clone: bool,
    // TDD FIX: When true, suppress .clone() for borrowed iterator field access in call arguments
    // The Call handler will add .clone() or & based on parameter ownership signature
    pub(crate) in_call_argument_generation: bool,
    // VEC INDEX CONTEXT: When generating the object of a FieldAccess, suppress Vec index
    // auto-clone since Rust allows field access on &T returned by Vec indexing.
    // e.g., players[i].score → no clone needed, just accesses the field through the ref.
    pub(crate) in_field_access_object: bool,
    // BORROW CONTEXT: When generating the operand of & or &mut, suppress Vec index
    // auto-clone since we want a reference to the original, not a reference to a clone.
    // e.g., &self.items[i] → reference to element, NOT &self.items[i].clone()
    pub(crate) in_borrow_context: bool,
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
    pub(crate) struct_field_types:
        std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
    // USER-DEFINED COPY TYPES: Registry of structs/enums with @derive(Copy)
    // Enables is_copy_type to recognize types like VoxelType as Copy, preventing unnecessary .clone()
    pub(crate) copy_types_registry: std::collections::HashSet<String>,
    // STRUCT LITERAL CONTEXT: When generating values for struct literal fields,
    // array literals should use fixed-size [...] syntax instead of vec![...],
    // since struct fields have explicit type annotations (e.g., [f32; 3]).
    pub(crate) in_struct_literal_field: bool,
    // STRUCT LITERAL CONTEXT: Track which struct we're currently constructing
    // Enables context-sensitive float type inference (f32 vs f64) for struct fields
    pub(crate) current_struct_literal_name: Option<String>,
    // STRUCT LITERAL CONTEXT: Track which field we're currently generating
    // Enables lookup of field type from struct_field_types for literal inference
    pub(crate) current_struct_field_name: Option<String>,
    // ENUM VARIANT TYPE TRACKING: Map "EnumName::VariantName" to field types
    // Enables string literal to String coercion in enum variant constructors
    pub(crate) enum_variant_types: std::collections::HashMap<String, Vec<Type>>,
    // EXPRESSION-LEVEL FLOAT TYPE INFERENCE: Results from constraint-based type inference
    // Maps expression locations to inferred float types (f32 vs f64)
    // Enables accurate float literal suffix generation without mixing errors
    pub(crate) float_inference: Option<crate::type_inference::FloatInference>,
}

// RECURSION GUARD MACRO: Check depth before entering recursive functions
const MAX_RECURSION_DEPTH: usize = 500; // Conservative limit to prevent stack overflow

impl<'ast> CodeGenerator<'ast> {
    /// Increment recursion depth and check if we've exceeded the limit
    pub(super) fn enter_recursion(&mut self, context: &str) -> Result<(), String> {
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
    pub(super) fn exit_recursion(&mut self) {
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
            in_user_written_closure: false,
            user_closure_params: std::collections::HashSet::new(),
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
            current_struct_literal_name: None,
            current_struct_field_name: None,
            float_inference: None,
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

    /// Set expression-level float type inference results
    /// Enables accurate f32/f64 suffix generation based on constraint solving
    pub fn set_float_inference(&mut self, inference: crate::type_inference::FloatInference) {
        self.float_inference = Some(inference);
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
    pub(super) fn record_mapping(&mut self, wj_location: &crate::source_map::Location) {
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
    pub(super) fn track_generated_lines(&mut self, code: &str) {
        let newline_count = code.matches('\n').count();
        if newline_count > 0 {
            self.increment_rust_lines(newline_count);
        }
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

    pub(crate) fn type_to_rust(&self, type_: &Type) -> String {
        crate::codegen::rust::type_to_rust(type_)
    }

    pub(super) fn is_type_copy(&self, ty: &Type) -> bool {
        crate::codegen::rust::type_analysis::is_copy_type(ty)
            || match ty {
                Type::Custom(name) => self.copy_types_registry.contains(name.as_str()),
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
