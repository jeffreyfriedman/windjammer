#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Rust code generator
use crate::analyzer::*;
use crate::parser::*;
use crate::CompilationTarget;
use std::cell::Cell;

pub use crate::codegen::rust::method_signature::MethodSignature;

pub struct CodeGenerator<'ast> {
    pub(crate) indent_level: usize,
    pub(crate) signature_registry: SignatureRegistry,
    pub(crate) in_wasm_bindgen_impl: bool,
    pub(crate) in_trait_impl: bool, // true if currently generating code for a trait implementation
    /// When in a trait impl, the trait name (for looking up analyzed_trait_methods)
    pub(crate) current_trait_impl_name: Option<String>,
    pub(crate) needs_wasm_imports: bool,
    pub(crate) needs_web_imports: bool,
    pub(crate) needs_js_imports: bool,
    pub(crate) needs_serde_imports: bool,   // For JSON support
    pub(crate) needs_write_import: bool,    // For string capacity optimization (write! macro)
    pub(crate) needs_smallvec_import: bool, // For Phase 8 SmallVec optimization
    pub(crate) needs_cow_import: bool,      // For Phase 9 Cow optimization
    pub(crate) needs_hashmap_import: bool,  // Auto-detect HashMap usage
    pub(crate) needs_hashset_import: bool,  // Auto-detect HashSet usage
    pub(crate) target: CompilationTarget,
    pub(crate) is_module: bool, // true if generating code for a reusable module (not main file)
    source_map: crate::source_map::SourceMap,
    pub(crate) current_output_file: std::path::PathBuf, // Path to the Rust file being generated
    current_rust_line: usize, // Current line number in generated Rust code (1-indexed)
    pub(crate) current_wj_file: std::path::PathBuf, // Path to the Windjammer file being compiled
    pub(crate) inferred_bounds: std::collections::HashMap<String, crate::inference::InferredBounds>,
    pub(crate) needs_trait_imports: std::collections::HashSet<String>, // Tracks which traits need imports
    pub(crate) bound_aliases: std::collections::HashMap<String, Vec<String>>, // bound Name = Trait + Trait
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
    // Global monotonic counter mirroring auto_clone::build_usage_map's counter.
    // Used for needs_clone() lookups to match indices in clone_sites.
    pub(crate) current_statement_idx: usize,
    pub(crate) auto_clone_counter: usize,
    // Local index within the current block (0-based enumerate index).
    // Used by variable_is_only_field_accessed and other block-relative analyses.
    pub(crate) current_block_local_idx: usize,
    // OPTION TAKE/REPLACE: Block-local indices of statements to skip because
    // they were folded into a preceding `.take()` or `.replace()`.
    pub(crate) skip_block_indices: std::collections::HashSet<usize>,
    // IMPLICIT SELF SUPPORT: Track struct fields for implicit self references
    pub(crate) current_struct_fields: std::collections::HashSet<String>, // Field names in current impl block
    pub(crate) current_struct_name: Option<String>, // Name of struct in current impl block
    pub(crate) current_impl_methods: std::collections::HashSet<String>, // Method names in current impl block
    pub(crate) current_impl_instance_methods: std::collections::HashSet<String>, // Methods that take self
    /// Same-impl methods that codegen will emit with owned/`mut self` (consuming receiver).
    pub(crate) current_impl_consuming_self_methods: std::collections::HashSet<String>,
    /// `TypeName::method` keys for zero-arg methods that only return `self.method` Copy field.
    pub(crate) trivial_copy_field_accessors: std::collections::HashSet<String>,
    /// Generic type parameter names from the current impl block (for per-method where clauses).
    pub(crate) current_impl_generic_type_params: Vec<String>,
    pub(crate) in_impl_block: bool, // true if currently generating code for an impl block
    // USIZE DETECTION: Track which struct fields have type usize (for auto-casting)
    pub(crate) usize_struct_fields:
        std::collections::HashMap<String, std::collections::HashSet<String>>, // Struct name -> usize field names
    // METHOD RETURN TYPES: Track which methods return usize (for auto-casting in comparisons)
    // Maps method name -> return type. Used by infer_expression_type for MethodCall.
    pub(crate) method_return_types: std::collections::HashMap<String, Type>,
    // FUNCTION CONTEXT: Track current function parameters for compound assignment optimization
    pub(crate) current_function_params: Vec<crate::parser::Parameter<'ast>>,
    pub(crate) current_function_type_bounds: Vec<(String, Vec<String>)>,
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
    // Cell for interior mutability (needed for call-site optimization in immutable context)
    pub(crate) suppress_string_conversion: Cell<bool>,
    /// When true, string literals emit `"...".to_string()` (owned String contexts: match arms, returns, if values, etc.)
    pub(crate) coerce_string_literals_to_owned: bool,
    // LOCAL VARIABLE TRACKING: Stack of scopes, each scope contains local variable names
    // Enables proper variable shadowing of field names
    pub(crate) local_variable_scopes: Vec<std::collections::HashSet<String>>,
    // EXPRESSION CONTEXT: Track if we're generating code whose value will be used
    // Prevents adding semicolons to final expressions in if-else/match when used as values
    pub(crate) in_expression_context: bool,
    // Suppress Vec::<T>::new() turbofish when let binding already has type ascription
    pub(crate) suppress_collection_turbofish: bool,
    // TDD: Track if we're generating the top-level function body (enables return optimization)
    pub(crate) in_function_body: bool,
    // TDD: Track if the current statement being generated is the last in its block
    pub(crate) current_is_last_statement: bool,
    // TRAIT TRACKING: Track which custom types support PartialEq
    pub(crate) partial_eq_types: std::collections::HashSet<String>,
    /// Struct (and struct-only) names that transitively contain a trait object (`dyn` / `trait X` field).
    /// Used by `type_contains_trait_object` for `Type::Custom` so outer structs skip `Debug`/`Clone`.
    pub(crate) trait_object_types: std::collections::HashSet<String>,
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
    // Track variables bound in for-loops with &mut iteration (need * for compound assignments)
    pub(crate) mut_borrowed_iterator_vars: std::collections::HashSet<String>,
    // When true, emit `get_mut` instead of `get` for the next HashMap method call.
    // Set by statement_generation when a let-binding from .get() has a mutated downstream value.
    pub(crate) upgrade_get_to_get_mut: bool,
    // OWNED STRING ITERATOR VARIABLES: Track variables from for-loops over Vec<String>
    // These need to be borrowed when used in String += operations
    pub(crate) owned_string_iterator_vars: std::collections::HashSet<String>,
    // MATCH ARM BINDINGS: Track variables bound in match arm patterns (EnumVariant bindings)
    // These are OWNED values extracted from enums, NOT references (even with .clone())
    // TDD FIX for E0614: prevent adding * to Copy type match bindings in comparisons
    pub(crate) match_arm_bindings: std::collections::HashSet<String>,
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
    // PHASE 2 STRING OPTIMIZATION: Track string parameters optimized to &str
    // These need .to_string() when passed to methods expecting owned String
    pub(crate) str_ref_optimized_params: std::collections::HashSet<String>,
    // USER-WRITTEN CLOSURE: When true, suppress auto-borrowing transformations (preserve user intent)
    pub(crate) in_user_written_closure: bool,
    // USER CLOSURE PARAMS: Track parameters of current user-written closure
    pub(crate) user_closure_params: std::collections::HashSet<String>,
    // ASSIGNMENT TARGET: Flag to suppress auto-clone when generating assignment targets
    pub(crate) generating_assignment_target: bool,
    /// While generating an assignment RHS, use this LHS type for float literal suffixes when
    /// FloatInference returns Unknown (multipass ExprId mismatch, etc.).
    pub(crate) assignment_float_target_type: Option<Type>,
    /// When a let-binding has an explicit type annotation, this provides the target type
    /// for `.collect()` turbofish generation (e.g., `let x: Vec<char> = ...collect()`).
    pub(crate) collect_target_type: Option<Type>,
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
    // STRING COMPARISON CONTEXT: Track when generating operands of string comparisons
    // Used to skip explicit * deref of &String (which becomes &str, breaking comparisons)
    // e.g., *id == flag_id → id == flag_id (both &String)
    pub(crate) in_string_comparison: bool,
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
    // TUPLE STRUCT NAMES: Track names of tuple structs (struct Point(i32, i32))
    // Enables ownership conversion in constructor calls (Point(x, y) needs owned args)
    pub(crate) tuple_struct_names: std::collections::HashSet<String>,
    // USER-DEFINED COPY TYPES: Registry of structs/enums with @derive(Copy)
    // Enables is_copy_type to recognize types like VoxelType as Copy, preventing unnecessary .clone()
    pub(crate) copy_types_registry: std::collections::HashSet<String>,
    /// Enums known to be non-Copy from library scan (e.g. `Value` with `String` variants).
    pub(crate) non_copy_types_registry: std::collections::HashSet<String>,
    // Types that implement Drop - cannot derive Copy (Rust E0184)
    pub(crate) types_with_drop: std::collections::HashSet<String>,
    // STRUCT LITERAL CONTEXT: When generating values for struct literal fields,
    // array literals should use fixed-size [...] syntax instead of vec![...],
    // since struct fields have explicit type annotations (e.g., [f32; 3]).
    pub(crate) in_struct_literal_field: bool,
    pub(crate) in_owned_value_context: bool,
    pub(crate) in_unsafe_block: bool,
    // STRUCT LITERAL CONTEXT: Track which struct we're currently constructing
    // Enables context-sensitive float type inference (f32 vs f64) for struct fields
    pub(crate) current_struct_literal_name: Option<String>,
    // STRUCT LITERAL CONTEXT: Track which field we're currently generating
    // Enables lookup of field type from struct_field_types for literal inference
    pub(crate) current_struct_field_name: Option<String>,
    // METHOD PARAM OWNERSHIP: Track analyzed ownership of each method's parameters.
    // Populated during function generation; used at call sites to auto-borrow arguments.
    // Key: method_name, Value: vec of (param_name, OwnershipMode).
    pub(crate) method_param_ownership:
        std::collections::HashMap<String, Vec<(String, OwnershipMode)>>,
    // METHOD SIGNATURES BY TYPE: Enhanced type-based method resolution
    // Maps ReceiverType → MethodName → Full Signature (params, return type, ownership)
    // Enables proper type-based decisions without hard-coded method name heuristics
    // Example: "Inventory" → "has_item" → MethodSignature { params: [("item_id", &str), ("qty", i32)], ... }
    pub(crate) method_signatures_by_type:
        std::collections::HashMap<String, std::collections::HashMap<String, MethodSignature>>,
    // STDLIB METHOD SIGNATURES: Preloaded signatures for Vec, String, HashMap, etc.
    // Enables correct parameter type checking for stdlib methods without hard-coding method names
    pub(crate) stdlib_method_signatures:
        std::collections::HashMap<String, std::collections::HashMap<String, MethodSignature>>,
    // ENUM VARIANT TYPE TRACKING: Map "EnumName::VariantName" to field types
    // Enables string literal to String coercion in enum variant constructors
    pub(crate) enum_variant_types: std::collections::HashMap<String, Vec<Type>>,
    /// Struct-like enum variants: same key as `enum_variant_types`, preserves field names for
    /// `infer_match_bound_types` when matching on `&vec[i]` (Rust binds `&T` per field).
    pub(crate) enum_variant_struct_fields: std::collections::HashMap<String, Vec<(String, Type)>>,
    // EXPRESSION-LEVEL FLOAT TYPE INFERENCE: Results from constraint-based type inference
    // Maps expression locations to inferred float types (f32 vs f64)
    // Enables accurate float literal suffix generation without mixing errors
    pub(crate) float_inference: Option<std::sync::Arc<crate::type_inference::FloatInference>>,
    // Enables accurate integer literal suffix generation (i32, i64, u32, etc.)
    pub(crate) int_inference: Option<std::sync::Arc<crate::type_inference::IntInference>>,
    /// Full-crate converged registry for multipass library codegen (avoids cloning into every file).
    global_signature_registry: Option<std::sync::Arc<SignatureRegistry>>,
    /// Library `.wj` root (multipass) for resolving submodule paths in auto-imports.
    pub(crate) library_source_root: Option<std::path::PathBuf>,
    /// Maps locally defined type names to Rust module paths (multiple entries when names collide).
    pub(crate) type_defining_modules: std::collections::HashMap<String, Vec<Vec<String>>>,
    /// `(parent_module, symbol)` → child module segment defining that symbol (multipass FFI layout).
    pub(crate) extern_submodule_qualifiers: std::collections::HashMap<(String, String), String>,
    /// Import aliases: maps alias name → original path.
    /// e.g., `use std::collections::HashMap as Map` → { "Map": "std::collections::HashMap" }
    /// Prevents stdlib type mappings from overriding user-defined aliases.
    pub(crate) import_aliases: std::collections::HashSet<String>,
    /// Module alias map: alias → last segment of the original module path.
    /// e.g., `use crate::ffi::gpu_safe as gpu` → { "gpu": "gpu_safe" }
    /// Used to resolve qualified calls through aliases for signature lookup.
    pub(crate) module_alias_map: std::collections::HashMap<String, String>,
    /// Names imported via `use std::strings` (etc.) that map to windjammer_runtime modules.
    /// Used to emit `module::fn` instead of `module.fn` for free functions.
    pub(crate) runtime_std_module_imports: std::collections::HashSet<String>,
    /// Simple names of all extern (FFI) functions across all modules.
    /// Used by codegen to wrap calls in `unsafe {}` even when signature lookup fails.
    pub(crate) extern_function_names: std::collections::HashSet<String>,
    /// Names of inline modules declared in the current program (Item::Mod).
    /// Used by generate_use to add `self::` prefix for `pub use` re-exports
    /// of items from inline sibling modules (Rust requires `self::` for these).
    pub(crate) inline_module_names: std::collections::HashSet<String>,
    /// Methods whose self receiver was upgraded from Borrowed to MutBorrowed
    /// during codegen (body-modification analysis). Used to update registry
    /// before writing metadata so cross-file builds see correct ownership.
    /// Key: qualified method name (e.g., "UnifiedRenderer::render_mesh").
    pub(crate) self_receiver_upgrades: std::collections::HashMap<String, OwnershipMode>,
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

    /// True when `name` refers to an imported `use std::…` runtime module (not a local variable).
    pub(in crate::codegen::rust) fn is_imported_runtime_std_module(&self, name: &str) -> bool {
        if self.runtime_std_module_imports.contains(name) {
            return true;
        }
        if let Some(original) = self.module_alias_map.get(name) {
            return self.runtime_std_module_imports.contains(original);
        }
        false
    }

    pub fn new(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        let extern_fn_names: std::collections::HashSet<String> = registry
            .signatures
            .iter()
            .filter(|(_, sig)| sig.is_extern)
            .map(|(name, _)| name.rsplit("::").next().unwrap_or(name).to_string())
            .collect();
        CodeGenerator {
            indent_level: 0,
            signature_registry: registry,
            in_wasm_bindgen_impl: false,
            in_trait_impl: false,
            current_trait_impl_name: None, // Set when generating trait impl methods
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
            auto_clone_counter: 0,
            current_block_local_idx: 0,
            skip_block_indices: std::collections::HashSet::new(),
            current_struct_fields: std::collections::HashSet::new(),
            current_struct_name: None,
            current_impl_methods: std::collections::HashSet::new(),
            current_impl_instance_methods: std::collections::HashSet::new(),
            current_impl_consuming_self_methods: std::collections::HashSet::new(),
            trivial_copy_field_accessors: std::collections::HashSet::new(),
            current_impl_generic_type_params: Vec::new(),
            in_impl_block: false,
            usize_struct_fields: std::collections::HashMap::new(),
            method_return_types: std::collections::HashMap::new(),
            current_function_params: Vec::new(),
            current_function_type_bounds: Vec::new(),
            current_function_return_type: None,
            current_function_body: Vec::new(),
            workspace_root: None,
            suppress_string_conversion: Cell::new(false),
            coerce_string_literals_to_owned: false,
            for_loop_borrow_needed: std::collections::HashSet::new(),
            borrowed_iterator_vars: std::collections::HashSet::new(),
            mut_borrowed_iterator_vars: std::collections::HashSet::new(),
            upgrade_get_to_get_mut: false,
            match_arm_bindings: std::collections::HashSet::new(),
            owned_string_iterator_vars: std::collections::HashSet::new(),
            usize_variables: std::collections::HashSet::new(),
            unused_let_bindings: std::collections::HashSet::new(),
            inferred_borrowed_params: std::collections::HashSet::new(),
            inferred_mut_borrowed_params: std::collections::HashSet::new(),
            str_ref_optimized_params: std::collections::HashSet::new(),
            in_user_written_closure: false,
            user_closure_params: std::collections::HashSet::new(),
            generating_assignment_target: false,
            assignment_float_target_type: None,
            collect_target_type: None,
            in_void_block: false,
            in_explicit_clone_call: false,
            suppress_borrowed_clone: false,
            in_field_access_object: false,
            in_call_argument_generation: false,
            in_borrow_context: false,
            in_string_comparison: false,
            partial_eq_types: std::collections::HashSet::new(),
            trait_object_types: std::collections::HashSet::new(),
            in_match_arm_needing_string: false,
            in_statement_match: false,
            local_variable_scopes: Vec::new(),
            in_expression_context: false,
            suppress_collection_turbofish: false,
            in_function_body: false,
            current_is_last_statement: false,
            analyzed_trait_methods: std::collections::HashMap::new(),
            generating_traits: std::collections::HashSet::new(),
            recursion_depth: 0,
            local_var_types: std::collections::HashMap::new(),
            struct_field_types: std::collections::HashMap::new(),
            tuple_struct_names: std::collections::HashSet::new(),
            copy_types_registry: std::collections::HashSet::new(),
            non_copy_types_registry: std::collections::HashSet::new(),
            types_with_drop: std::collections::HashSet::new(),
            in_struct_literal_field: false,
            in_owned_value_context: false,
            in_unsafe_block: false,
            current_struct_literal_name: None,
            current_struct_field_name: None,
            float_inference: None,
            int_inference: None,
            method_param_ownership: std::collections::HashMap::new(),
            method_signatures_by_type: std::collections::HashMap::new(),
            stdlib_method_signatures:
                crate::codegen::rust::stdlib_method_signatures::init_stdlib_method_signatures(),
            enum_variant_types: std::collections::HashMap::new(),
            enum_variant_struct_fields: std::collections::HashMap::new(),
            library_source_root: None,
            global_signature_registry: None,
            type_defining_modules: std::collections::HashMap::new(),
            extern_submodule_qualifiers: std::collections::HashMap::new(),
            import_aliases: std::collections::HashSet::new(),
            module_alias_map: std::collections::HashMap::new(),
            runtime_std_module_imports: std::collections::HashSet::new(),
            extern_function_names: extern_fn_names,
            inline_module_names: std::collections::HashSet::new(),
            self_receiver_upgrades: std::collections::HashMap::new(),
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
        // Track simple names → all qualified sources for disambiguation.
        // When two structs share a simple name (e.g., rpg::item::ItemStack vs
        // inventory::item_stack::ItemStack), we only store field types under the
        // simple name when ALL sources agree on a given field's type.
        let mut simple_name_fields: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<crate::parser::Type>>,
        > = std::collections::HashMap::new();

        for (struct_name, fields) in &field_types {
            // Always insert under qualified name
            self.struct_field_types
                .entry(struct_name.clone())
                .or_default()
                .extend(fields.clone());

            if let Some(base) = struct_name.rsplit("::").next() {
                if base != struct_name.as_str() {
                    let entry = simple_name_fields.entry(base.to_string()).or_default();
                    for (field_name, field_type) in fields {
                        entry
                            .entry(field_name.clone())
                            .or_default()
                            .push(field_type.clone());
                    }
                }
            }
        }

        // For simple name entries, only store fields where ALL sources agree on the type.
        // This prevents e.g. ItemStack.quantity being incorrectly resolved as u32 when
        // one definition has i32 and another has u32.
        for (base_name, field_sources) in simple_name_fields {
            let mut safe_fields = std::collections::HashMap::new();
            for (field_name, types) in field_sources {
                if types.len() == 1 || types.windows(2).all(|w| w[0] == w[1]) {
                    safe_fields.insert(field_name, types.into_iter().next().unwrap());
                }
                // If types disagree for this field, skip it (ambiguous)
            }
            if !safe_fields.is_empty() {
                self.struct_field_types
                    .entry(base_name)
                    .or_default()
                    .extend(safe_fields);
            }
        }
    }

    /// Set Copy types registry from the global compiler state.
    /// This enables is_copy_type to recognize user-defined types with @derive(Copy)
    /// (e.g., VoxelType, FaceDirection) in addition to primitive Copy types.
    pub fn set_copy_types_registry(&mut self, registry: std::collections::HashSet<String>) {
        self.copy_types_registry = registry;
    }

    pub fn set_non_copy_types_registry(&mut self, registry: std::collections::HashSet<String>) {
        self.non_copy_types_registry = registry;
    }

    /// Pre-populate enum variant payload types from the whole library (cross-module factory helpers).
    pub fn set_global_enum_variant_types(
        &mut self,
        variant_types: std::collections::HashMap<String, Vec<crate::parser::Type>>,
    ) {
        self.enum_variant_types.extend(variant_types);
    }

    /// Look up a method signature by receiver type and method name
    /// This is the PROPER way to determine parameter types/ownership
    /// REPLACES all hard-coded method name heuristics ("push", "has_item", etc.)
    pub fn lookup_method_signature(
        &self,
        receiver_type: &str,
        method_name: &str,
    ) -> Option<&MethodSignature> {
        // First check user-defined methods (populated during function generation)
        if let Some(methods) = self.method_signatures_by_type.get(receiver_type) {
            if let Some(sig) = methods.get(method_name) {
                return Some(sig);
            }
        }

        // Then check stdlib methods (Vec, String, HashMap, etc.)
        if let Some(methods) = self.stdlib_method_signatures.get(receiver_type) {
            if let Some(sig) = methods.get(method_name) {
                return Some(sig);
            }
        }

        // Check with generic type parameters stripped (Vec<T> → Vec)
        if let Some(base) = receiver_type.split('<').next() {
            if base != receiver_type {
                // Try again with just the base type
                if let Some(methods) = self.stdlib_method_signatures.get(base) {
                    if let Some(sig) = methods.get(method_name) {
                        return Some(sig);
                    }
                }
            }
        }

        None
    }

    /// Register a user-defined method signature
    /// Called during function generation to build the method registry
    pub fn register_method_signature(&mut self, sig: MethodSignature) {
        self.method_signatures_by_type
            .entry(sig.receiver_type.clone())
            .or_default()
            .insert(sig.method_name.clone(), sig);
    }

    /// Resolve the type of a receiver expression for method calls
    /// Example: `self.inventory.has_item(...)` → resolve type of `self.inventory`
    /// This enables looking up the correct method signature
    #[allow(dead_code)] // Reserved for future type resolution
    pub(crate) fn resolve_receiver_type(&self, receiver: &Expression) -> Option<String> {
        match receiver {
            Expression::Identifier { name, .. } => {
                // Check local variables
                if let Some(ty) = self.local_var_types.get(name.as_str()) {
                    return Some(self.type_to_simple_name(ty));
                }

                // Check function parameters
                for param in &self.current_function_params {
                    if param.name == *name {
                        return Some(self.type_to_simple_name(&param.type_));
                    }
                }

                None
            }
            Expression::FieldAccess { object, field, .. } => {
                // Recursively resolve object type, then look up field type
                let object_type = self.resolve_receiver_type(object)?;

                // Look up field type in struct_field_types
                let field_types = self.struct_field_types.get(&object_type)?;
                let field_type = field_types.get(field.as_str())?;

                Some(self.type_to_simple_name(field_type))
            }
            _ => None,
        }
    }

    /// Convert a Type to a simple name for signature lookup
    /// Example: Type::Custom("Vec") → "Vec", Type::Reference(Box(Custom("String"))) → "String"
    #[allow(dead_code)] // Reserved for future type conversion
    fn type_to_simple_name(&self, ty: &Type) -> String {
        match ty {
            Type::Custom(name) => name.clone(),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.type_to_simple_name(inner)
            }
            Type::Vec(_) => "Vec".to_string(),
            Type::Option(_) => "Option".to_string(),
            Type::Result(_, _) => "Result".to_string(),
            Type::Parameterized(base, _) => base.clone(),
            _ => "Unknown".to_string(),
        }
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
        self.float_inference = Some(std::sync::Arc::new(inference));
    }

    /// Share one global float inference across many library codegen passes (avoids cloning 668-file state).
    pub fn set_shared_float_inference(
        &mut self,
        inference: std::sync::Arc<crate::type_inference::FloatInference>,
    ) {
        self.float_inference = Some(inference);
    }

    /// Enables accurate integer literal suffix generation (i32, i64, u32, etc.)
    pub fn set_int_inference(&mut self, inference: crate::type_inference::IntInference) {
        self.int_inference = Some(std::sync::Arc::new(inference));
    }

    pub fn set_shared_int_inference(
        &mut self,
        inference: std::sync::Arc<crate::type_inference::IntInference>,
    ) {
        self.int_inference = Some(inference);
    }

    /// Attach the converged crate-wide registry for lookup fallback (library multipass codegen).
    pub fn set_global_signature_registry(&mut self, registry: std::sync::Arc<SignatureRegistry>) {
        self.global_signature_registry = Some(registry);
    }

    pub(crate) fn get_signature_with_global(&self, name: &str) -> Option<&FunctionSignature> {
        if let Some(sig) = self.signature_registry.get_signature(name) {
            return Some(sig);
        }
        self.global_signature_registry
            .as_ref()?
            .get_signature(name)
    }

    pub(crate) fn find_method_on_receiver_with_global(
        &self,
        type_name: &str,
        method: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        if let Some(sig) = self
            .signature_registry
            .find_method_on_receiver_type(type_name, method, arg_count)
        {
            return Some(sig);
        }
        self.global_signature_registry.as_ref().and_then(|g| {
            g.find_method_on_receiver_type(type_name, method, arg_count)
        })
    }

    pub(crate) fn find_signature_by_name_and_arg_count_with_global(
        &self,
        name: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        if let Some(sig) = self
            .signature_registry
            .find_signature_by_name_and_arg_count(name, arg_count)
        {
            return Some(sig);
        }
        self.global_signature_registry
            .as_ref()?
            .find_signature_by_name_and_arg_count(name, arg_count)
    }

    pub(crate) fn global_signature_registry(&self) -> Option<&SignatureRegistry> {
        self.global_signature_registry.as_deref()
    }

    pub(in crate::codegen::rust) fn mc_method_param_expects_owned_string_from_global(
        &self,
        object: &Expression<'_>,
        method: &str,
        arg_idx: usize,
        arg_count: usize,
    ) -> bool {
        let Some(type_name) = self.infer_type_name(object) else {
            return false;
        };
        let Some(global) = self.global_signature_registry.as_ref() else {
            return false;
        };
        let qualified = format!("{type_name}::{method}");
        let Some(sig) = global.get_signature(&qualified) else {
            return false;
        };
        if !crate::codegen::rust::call_signature_resolution::validate_arg_count(sig, arg_count) {
            return false;
        }
        sig.param_type_for_arg(arg_idx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_owned_string_type(t)
        })
    }

    pub(crate) fn resolve_call_signature_with_global(
        &self,
        func_name: &str,
        receiver_type: Option<&str>,
        arg_count: usize,
    ) -> Option<crate::codegen::rust::call_signature_resolution::ResolvedSignature> {
        let caller_module = self.library_source_root.as_ref().and_then(|root| {
            if self.current_wj_file.as_os_str().is_empty() {
                None
            } else {
                crate::analyzer::type_collector::wj_file_to_module_path(root, &self.current_wj_file)
                    .map(|parts| parts.join("::"))
            }
        });
        let local = crate::codegen::rust::call_signature_resolution::resolve_call_signature(
            &self.signature_registry,
            func_name,
            receiver_type,
            arg_count,
            &self.module_alias_map,
            caller_module.as_deref(),
        );
        let global = self.global_signature_registry.as_ref().and_then(|global| {
            crate::codegen::rust::call_signature_resolution::resolve_call_signature(
                global,
                func_name,
                receiver_type,
                arg_count,
                &self.module_alias_map,
                caller_module.as_deref(),
            )
        });
        let picked = crate::codegen::rust::call_signature_resolution::pick_best_resolved_signature(
            local, global,
        );
        if let Some(ref resolved) = picked {
            if crate::codegen::rust::call_signature_resolution::has_stale_owned_non_copy_params(
                &resolved.sig,
            ) {
                if let Some(global_reg) = self.global_signature_registry.as_ref() {
                    if let Some(global_only) =
                        crate::codegen::rust::call_signature_resolution::resolve_call_signature(
                            global_reg,
                            func_name,
                            receiver_type,
                            arg_count,
                            &self.module_alias_map,
                            caller_module.as_deref(),
                        )
                    {
                        if !crate::codegen::rust::call_signature_resolution::has_stale_owned_non_copy_params(
                            &global_only.sig,
                        ) {
                            return Some(global_only);
                        }
                    }
                }
            }
        }
        picked
    }

    /// Resolve `Type::method` for call-site borrow lowering (Self:: and instance calls).
    pub(in crate::codegen::rust) fn lookup_method_signature_on_receiver_type(
        &self,
        receiver_type: &str,
        method: &str,
        arg_count: usize,
    ) -> Option<crate::analyzer::FunctionSignature> {
        use crate::codegen::rust::call_signature_resolution::{
            accept_method_resolution_for_receiver, validate_arg_count,
        };

        // Prefer signatures registered during codegen (analyzed ownership/types) over
        // declaration stubs in the registry (often all-Owned before convergence).
        if let Some(ms) = self.lookup_method_signature(receiver_type, method) {
            let sig = ms.to_function_signature();
            if validate_arg_count(&sig, arg_count) {
                return Some(sig);
            }
        }

        let qualified = format!("{receiver_type}::{method}");
        if let Some(resolved) =
            self.resolve_call_signature_with_global(&qualified, Some(receiver_type), arg_count)
        {
            if accept_method_resolution_for_receiver(&resolved, receiver_type, method) {
                return Some(resolved.sig);
            }
        }

        if let Some(sig) = self
            .signature_registry
            .find_method_on_receiver_type(receiver_type, method, arg_count)
        {
            return Some(sig.clone());
        }
        if let Some(global) = &self.global_signature_registry {
            if let Some(sig) = global.find_method_on_receiver_type(receiver_type, method, arg_count)
            {
                return Some(sig.clone());
            }
        }

        // Module-path qualified keys from library multipass (e.g. `foo::Type::method`).
        let suffix = format!("::{receiver_type}::{method}");
        for (key, sig) in self.signature_registry.all_signatures() {
            if key.ends_with(&suffix)
                && crate::codegen::rust::call_signature_resolution::validate_arg_count(sig, arg_count)
            {
                return Some(sig.clone());
            }
        }
        if let Some(global) = &self.global_signature_registry {
            for (key, sig) in global.all_signatures() {
                if key.ends_with(&suffix)
                    && crate::codegen::rust::call_signature_resolution::validate_arg_count(
                        sig, arg_count,
                    )
                {
                    return Some(sig.clone());
                }
            }
        }
        None
    }

    pub(crate) fn has_collision_with_global(&self, name: &str) -> bool {
        self.signature_registry.has_collision(name)
            || self
                .global_signature_registry
                .as_ref()
                .is_some_and(|g| g.has_collision(name))
    }

    pub(crate) fn should_skip_int_to_float_auto_cast_with_global(
        &self,
        type_name: Option<&str>,
        method: &str,
        qualified_key: Option<&str>,
    ) -> bool {
        if self
            .signature_registry
            .should_skip_int_to_float_auto_cast(type_name, method, qualified_key)
        {
            return true;
        }
        self.global_signature_registry.as_ref().is_some_and(|g| {
            g.should_skip_int_to_float_auto_cast(type_name, method, qualified_key)
        })
    }

    /// Used with multipass library builds to resolve `use super::...::Type` across sibling `.wj` modules.
    pub fn set_library_source_root(&mut self, root: std::path::PathBuf) {
        self.library_source_root = Some(root);
    }

    pub fn set_type_defining_modules(
        &mut self,
        map: std::collections::HashMap<String, Vec<Vec<String>>>,
    ) {
        self.type_defining_modules = map;
    }

    /// Multipass: parent-module `use` + `parent::symbol` call sites when `symbol` is defined in
    /// `parent/child/*.wj` (e.g. `ffi/api.wj`).
    pub fn set_extern_submodule_qualifiers(
        &mut self,
        map: std::collections::HashMap<(String, String), String>,
    ) {
        self.extern_submodule_qualifiers = map;
    }

    pub(crate) fn qualify_external_path_identifier(&self, name: &str) -> String {
        if self.extern_submodule_qualifiers.is_empty() || !name.contains("::") {
            return name.to_string();
        }
        let normalized = name.replace('.', "::");
        crate::codegen::rust::codegen_helpers::qualify_parent_child_external_path(
            &self.extern_submodule_qualifiers,
            &normalized,
        )
    }

    pub fn new_for_module(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        let mut gen = Self::new(registry, target);
        gen.is_module = true;
        gen
    }

    /// Apply codegen self-receiver upgrades to a registry snapshot.
    /// When codegen determines a method needs `&mut self` (via body-modification
    /// analysis) but the analyzer only inferred `Borrowed`, update the registry
    /// so metadata reflects the actual generated code for cross-file builds.
    pub fn apply_self_receiver_upgrades(&self, registry: &mut SignatureRegistry) {
        for (qualified_name, upgrade_mode) in &self.self_receiver_upgrades {
            if let Some(sig) = registry.signatures.get_mut(qualified_name) {
                sig.has_self_receiver = true;
                if sig.param_ownership.is_empty() {
                    sig.param_ownership.push(*upgrade_mode);
                } else if sig.param_ownership[0] != *upgrade_mode {
                    sig.param_ownership[0] = *upgrade_mode;
                }
            }
        }
    }

    pub(crate) fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    /// Generate an item inside an inline module
    pub(crate) fn generate_inline_module_item(
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

    /// Whether a named identifier (from `current_function_params`) already generates
    /// as a Rust reference, accounting for all three ref-tracking systems:
    ///  - `inferred_borrowed_params` (analyzer ownership inference)
    ///  - `str_ref_optimized_params` (Phase 2 string→&str optimization)
    ///  - explicit `Reference`/`MutableReference`/`Custom("str")` AST types
    pub(crate) fn identifier_already_ref(&self, name: &str) -> bool {
        if self.borrowed_iterator_vars.contains(name) {
            return true;
        }
        if self.str_ref_optimized_params.contains(name) {
            return true;
        }
        self.current_function_params.iter().any(|p| {
            p.name == name
                && (matches!(
                    p.ownership,
                    crate::parser::OwnershipHint::Ref | crate::parser::OwnershipHint::Mut
                ) || crate::codegen::rust::types::param_generates_as_rust_ref(
                    &p.type_,
                    &p.name,
                    &self.inferred_borrowed_params,
                ))
        })
    }

    /// Whether a named identifier already generates as `&mut T` in Rust (explicit or inferred).
    pub(crate) fn identifier_already_mut_ref(&self, name: &str) -> bool {
        if self.inferred_mut_borrowed_params.contains(name) {
            return true;
        }
        self.current_function_params.iter().any(|p| {
            p.name == name
                && (matches!(p.ownership, crate::parser::OwnershipHint::Mut)
                    || matches!(&p.type_, Type::MutableReference(_)))
        })
    }

    /// Check if a binding needs `.clone()` per auto-clone analysis and apply it.
    ///
    /// Returns the (possibly cloned) expression string. Skips the clone when:
    /// - The binding is already cloned (ends with `.clone()`)
    /// - The binding's type implements `Copy`
    ///
    /// This consolidates the identical check previously duplicated in
    /// `regular_call_arguments`, `function_call_generation`, and other
    /// argument-generation paths.
    pub(crate) fn maybe_auto_clone(&self, name: &str, arg_str: &str) -> String {
        let dominated = self
            .auto_clone_analysis
            .as_ref()
            .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());

        if !dominated || arg_str.ends_with(".clone()") {
            return arg_str.to_string();
        }

        let binding_is_copy = self
            .current_function_params
            .iter()
            .find(|p| p.name == name)
            .is_some_and(|p| self.is_type_copy(&p.type_))
            || self
                .local_var_types
                .get(name)
                .is_some_and(|t| self.is_type_copy(t));

        if binding_is_copy {
            return arg_str.to_string();
        }

        if arg_str.contains(" as ") && !arg_str.starts_with('(') {
            format!("({}).clone()", arg_str)
        } else {
            format!("{}.clone()", arg_str)
        }
    }

    /// Deref `&Copy` / `&mut Copy` expressions when the function returns an owned Copy type.
    /// Handles `.get().unwrap()` chains and other reference-producing expressions.
    pub(crate) fn coerce_return_ref_to_owned_copy(
        &self,
        expr_str: &mut String,
        expr: &crate::parser::Expression,
    ) {
        if expr_str.starts_with('*') || expr_str.ends_with(".clone()") {
            return;
        }
        let expects_owned = !matches!(
            &self.current_function_return_type,
            Some(Type::Reference(_)) | Some(Type::MutableReference(_))
        );
        if !expects_owned {
            return;
        }
        if let Expression::Identifier { name, .. } = expr {
            if (self.inferred_mut_borrowed_params.contains(name)
                || self.inferred_borrowed_params.contains(name))
                && self
                    .current_function_return_type
                    .as_ref()
                    .is_some_and(|t| self.is_type_copy(t))
                && !expr_str.starts_with('*')
            {
                *expr_str = format!("*{}", expr_str);
                return;
            }
        }
        if let Some(Type::Reference(inner) | Type::MutableReference(inner)) =
            self.infer_expression_type(expr)
        {
            if self.is_type_copy(inner.as_ref()) {
                *expr_str = format!("*{}", expr_str);
            }
        }
    }

    /// Apply owned-String tail coercion to an implicit-return or explicit-return expression.
    ///
    /// When a function returns `String`, this converts string literals to owned form,
    /// rewrites borrowed-param `.clone()` to `.to_string()`, and clones `self.field`
    /// when `self` is borrowed. Used by block implicit returns and `return` statements.
    ///
    /// `respect_suppress`: if true, checks `suppress_string_conversion` and `.as_str()` usage.
    pub(crate) fn apply_owned_string_tail_coercion(
        &self,
        expr_str: &mut String,
        expr: &crate::parser::Expression,
        respect_suppress: bool,
    ) {
        let returns_string = super::string_utilities::return_type_expects_owned_string(
            &self.current_function_return_type,
        );
        let in_match_needing_string = self.in_match_arm_needing_string;

        if !returns_string && !in_match_needing_string {
            return;
        }

        if respect_suppress {
            if expr_str.contains(".as_str()") {
                return;
            }
            if self.suppress_string_conversion.get() {
                return;
            }
        }

        if matches!(
            expr,
            crate::parser::Expression::Literal {
                value: crate::parser::Literal::String(_),
                ..
            }
        ) && !super::string_utilities::already_owned_string_expr(expr_str)
        {
            *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
        } else {
            super::string_utilities::rewrite_borrowed_str_clone_to_to_string(
                expr_str,
                expr,
                &self.inferred_borrowed_params,
                &self.current_function_params,
            );
            if !super::string_utilities::already_owned_string_expr(expr_str) {
                if let crate::parser::Expression::Identifier { name, .. } = expr {
                    let is_ref_text = self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                        matches!(
                            t,
                            Type::Reference(inner) | Type::MutableReference(inner)
                                if super::types::is_windjammer_text_type(inner)
                        )
                    });
                    let is_string_param = self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && (super::types::is_windjammer_text_type(&p.type_)
                                || matches!(
                                    &p.type_,
                                    Type::Reference(inner)
                                        if super::types::is_windjammer_text_type(inner)
                                ))
                    });
                    if is_ref_text || is_string_param {
                        *expr_str =
                            super::string_utilities::coerce_expr_to_owned_string(expr_str);
                    }
                }
            }
        }

        self.maybe_clone_borrowed_self_field(expr_str, expr);
    }

    /// If `expr` is `self.field` and `self` is borrowed, append `.clone()` for non-Copy fields.
    pub(crate) fn maybe_clone_borrowed_self_field(
        &self,
        expr_str: &mut String,
        expr: &crate::parser::Expression,
    ) {
        if let crate::parser::Expression::FieldAccess { object, .. } = expr {
            if let crate::parser::Expression::Identifier { name: obj_name, .. } = &**object {
                if obj_name == "self" && !expr_str.ends_with(".clone()") {
                    let self_is_borrowed = self.current_function_params.iter().any(|p| {
                        p.name == "self" && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                    });
                    if self_is_borrowed {
                        let is_copy = self
                            .infer_expression_type(expr)
                            .as_ref()
                            .is_some_and(|t| self.is_type_copy(t));
                        if !is_copy {
                            *expr_str = format!("{}.clone()", expr_str);
                        }
                    }
                }
            }
        }
    }

    /// Clone a FieldAccess argument whose root identifier is borrowed when the callee expects Owned.
    ///
    /// Traces through nested field accesses (e.g. `stack.item.id`) to find the root,
    /// then checks if it's borrowed (iterator var, inferred borrow, or explicit `Ref` hint).
    /// Appends `.clone()` for non-Copy types that don't already have it.
    pub(crate) fn maybe_clone_borrowed_field_for_owned_param(
        &self,
        arg: &crate::parser::Expression,
        arg_str: &mut String,
    ) -> bool {
        if let crate::parser::Expression::FieldAccess { object, .. } = arg {
            let root_name = self.extract_root_identifier(arg);
            if let Some(ref name) = root_name {
                let is_self_field = matches!(&**object, crate::parser::Expression::Identifier { name: n, .. } if n == "self");
                let is_borrowed_iter = self.borrowed_iterator_vars.contains(name);
                let is_explicit_ref = self.current_function_params.iter().any(|p| {
                    p.name == *name && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                });
                let is_inferred_borrowed = self.inferred_borrowed_params.contains(name);

                if (is_self_field
                    || is_borrowed_iter
                    || is_explicit_ref
                    || is_inferred_borrowed)
                    && !arg_str.ends_with(".clone()")
                {
                    let is_copy = self
                        .infer_expression_type(arg)
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));
                    if !is_copy {
                        *arg_str = format!("{}.clone()", arg_str);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Clone a Vec-index expression (`&vec[i]`) when the callee expects Owned and the element is non-Copy.
    pub(crate) fn maybe_clone_index_for_owned_param(
        &self,
        arg: &crate::parser::Expression,
        arg_str: &mut String,
    ) -> bool {
        if let crate::parser::Expression::Index { .. } = arg {
            if arg_str.starts_with('&') && !arg_str.ends_with(".clone()") {
                if let Some(inner) = self.infer_expression_type(arg) {
                    if !self.is_type_copy(&inner) {
                        *arg_str = format!("({}).clone()", arg_str);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Enter argument-generation scope. Saves context flags that must be
    /// restored after `generate_expression` returns so that nested calls
    /// don't leak context into the outer expression.
    ///
    /// Drop the returned guard to restore the previous flag values.
    pub(crate) fn arg_gen_scope(&mut self) -> ArgGenScope {
        let saved = ArgGenScope {
            in_field_access_object: self.in_field_access_object,
            in_call_argument_generation: self.in_call_argument_generation,
            coerce_string_literals_to_owned: self.coerce_string_literals_to_owned,
            in_match_arm_needing_string: self.in_match_arm_needing_string,
        };
        self.in_field_access_object = false;
        self.in_call_argument_generation = true;
        self.coerce_string_literals_to_owned = false;
        self.in_match_arm_needing_string = false;
        saved
    }

    /// Restore context flags saved by `arg_gen_scope`.
    pub(crate) fn restore_arg_gen_scope(&mut self, scope: ArgGenScope) {
        self.in_field_access_object = scope.in_field_access_object;
        self.in_call_argument_generation = scope.in_call_argument_generation;
        self.coerce_string_literals_to_owned = scope.coerce_string_literals_to_owned;
        self.in_match_arm_needing_string = scope.in_match_arm_needing_string;
    }
}

/// Saved state of argument-generation context flags.
/// Created by `CodeGenerator::arg_gen_scope()` and consumed by `restore_arg_gen_scope()`.
pub(crate) struct ArgGenScope {
    in_field_access_object: bool,
    in_call_argument_generation: bool,
    coerce_string_literals_to_owned: bool,
    in_match_arm_needing_string: bool,
}
