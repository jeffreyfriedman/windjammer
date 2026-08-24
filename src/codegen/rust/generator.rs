#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Rust code generator
use crate::analyzer::*;
use crate::parser::*;
use crate::CompilationTarget;
use std::cell::Cell;

pub use crate::codegen::rust::method_signature::MethodSignature;

/// Convert an IR `OwnedType` to the legacy `OwnershipMode` used by codegen.
pub(crate) fn owned_type_to_ownership_mode(
    owned: &crate::ir::safety_type::OwnedType,
) -> OwnershipMode {
    match owned {
        crate::ir::safety_type::OwnedType::Owned => OwnershipMode::Owned,
        crate::ir::safety_type::OwnedType::Ref(_) => OwnershipMode::Borrowed,
        crate::ir::safety_type::OwnedType::MutRef(_) => OwnershipMode::MutBorrowed,
        crate::ir::safety_type::OwnedType::Copy => OwnershipMode::Owned,
        crate::ir::safety_type::OwnedType::Inferred => OwnershipMode::Owned,
    }
}

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
    pub(crate) serde_available: bool, // Project-level: serde dependency exists (auto-derive Serialize)
    pub(crate) needs_write_import: bool, // For string capacity optimization (write! macro)
    pub(crate) needs_smallvec_import: bool, // For Phase 8 SmallVec optimization
    pub(crate) needs_cow_import: bool, // For Phase 9 Cow optimization
    pub(crate) needs_hashmap_import: bool, // Auto-detect HashMap usage
    pub(crate) needs_hashset_import: bool, // Auto-detect HashSet usage
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
    /// Impl types whose sibling methods were preregistered across all impl blocks.
    pub(crate) preregistered_impl_sibling_types: std::collections::HashSet<String>,
    /// WJ source non-`self` formal types per method, merged across impl blocks for one struct.
    pub(crate) struct_method_ast_formal_param_types:
        std::collections::HashMap<String, std::collections::HashMap<String, Vec<Type>>>,
    /// WJ source formal types per free function (authoritative before registry convergence).
    pub(crate) free_function_ast_formal_param_types:
        std::collections::HashMap<String, Vec<Type>>,
    /// Parallel to `free_function_ast_formal_param_types`: true when the WJ body assigns
    /// through that param's fields/indexes (take/restore → emit `&mut`, not owned).
    pub(crate) free_function_ast_param_field_written:
        std::collections::HashMap<String, Vec<bool>>,
    /// Structs with ≥1 method that field-reads an owned non-Copy custom formal (lookup facade).
    pub(crate) struct_has_owned_key_field_lookup: std::collections::HashSet<String>,
    /// Structs with ≥1 method that only forwards an owned custom formal to a self sibling.
    pub(crate) struct_has_owned_key_sibling_wrapper: std::collections::HashSet<String>,
    /// Parallel to method AST formals: field-write flags per non-self param.
    pub(crate) struct_method_ast_param_field_written:
        std::collections::HashMap<String, std::collections::HashMap<String, Vec<bool>>>,
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
    pub(crate) current_function_name: Option<String>,
    pub(crate) current_function_type_bounds: Vec<(String, Vec<String>)>,
    pub(crate) current_function_return_type: Option<Type>,
    // WINDJAMMER TRAIT INFERENCE: Analyzed trait methods with inferred signatures from ALL impls
    pub(crate) analyzed_trait_methods: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::analyzer::AnalyzedFunction<'ast>>,
    >,
    // FUNCTION CONTEXT: Track current function body for data flow analysis
    pub(crate) current_function_body: Vec<&'ast Statement<'ast>>, // Body of the current function being generated
    /// Snapshot of the outer function body at prepare time — nested blocks temporarily
    /// replace `current_function_body`; pure-forwarding refresh must not run on those.
    pub(crate) full_function_body_snapshot: Vec<&'ast Statement<'ast>>,
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
    /// Module-level `const NAME: string = "…"` identifiers (lower to `&'static str` in Rust).
    pub(crate) module_string_consts: std::collections::HashSet<String>,
    /// Nesting depth of loop bodies — explicit `.clone()` in loops must be preserved (WDB-105).
    pub(crate) loop_body_depth: u32,
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
    /// Owned `String` formals that only flow into map/set key lookups (`get(&key)` at call sites).
    pub(crate) collection_key_owned_params: std::collections::HashSet<String>,
    /// Formals whose emitted Rust signature is already `&T` / `&mut T` (synced during param emission).
    pub(crate) emitted_rust_ref_formals: std::collections::HashSet<String>,
    /// Per-function call-site arg indices that need `&mut` (from emitted `&mut T` formals).
    pub(crate) function_emitted_mut_arg_indices:
        std::collections::HashMap<String, std::collections::HashSet<usize>>,
    /// Arg indices recorded while emitting the current function's `&mut T` formals.
    pub(crate) current_fn_emitted_mut_arg_indices: std::collections::HashSet<usize>,
    /// Params that keep owned Rust formals but borrow at select call sites (dogfood `put_value`).
    pub(crate) current_fn_mixed_forwarder_params: std::collections::HashSet<String>,
    /// Params that borrow at self/sibling calls inside if conditions (forward-ref guard).
    pub(crate) current_fn_forward_ref_if_params: std::collections::HashSet<String>,
    /// True when the current function body is a single `self[.field].method(...)` forward.
    pub(crate) current_func_is_pure_forwarding_delegate: bool,
    // USER-WRITTEN CLOSURE: When true, suppress auto-borrowing transformations (preserve user intent)
    pub(crate) in_user_written_closure: bool,
    // USER CLOSURE PARAMS: Track parameters of current user-written closure
    pub(crate) user_closure_params: std::collections::HashSet<String>,
    /// Iterator predicate methods (`filter`, `find`, …): typed closure params become `&T` in Rust.
    pub(crate) closure_predicate_typed_params: bool,
    // ASSIGNMENT TARGET: Flag to suppress auto-clone when generating assignment targets
    pub(crate) generating_assignment_target: bool,
    /// While generating an assignment RHS, use this LHS type for float literal suffixes when
    /// numeric inference returns Unknown (multipass ExprId mismatch, etc.).
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
    /// While generating an array/Vec index expression — Rust infers literal `usize`.
    pub(crate) in_index_context: bool,
    // BORROW CONTEXT: When generating the operand of & or &mut, suppress Vec index
    // auto-clone since we want a reference to the original, not a reference to a clone.
    // e.g., &self.items[i] → reference to element, NOT &self.items[i].clone()
    pub(crate) in_borrow_context: bool,
    /// True while generating an `if`/`while` condition expression (not branch bodies).
    pub(crate) in_if_condition: bool,
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
    /// Types explicitly annotated with `@derive(Copy)` by the user.
    /// Distinguished from auto-derived Copy types to preserve `&mut` semantics
    /// for auto-derived types while allowing value semantics for explicit ones.
    pub(crate) explicit_copy_types_registry: std::collections::HashSet<String>,
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
    /// Stdlib type name → fully-qualified Rust path (`HttpMethod` → `windjammer_runtime::http::HttpMethod`).
    /// Used when string→unit-enum coercion must emit without a sibling `use`.
    pub(crate) stdlib_type_rust_paths: std::collections::HashMap<String, String>,
    /// Struct-like enum variants: same key as `enum_variant_types`, preserves field names for
    /// `infer_match_bound_types` when matching on `&vec[i]` (Rust binds `&T` per field).
    pub(crate) enum_variant_struct_fields: std::collections::HashMap<String, Vec<(String, Type)>>,
    pub(crate) numeric_inference:
        Option<std::sync::Arc<crate::ir::numeric_bridge::UnifiedNumericInference>>,
    /// Full-crate converged registry for multipass library codegen (avoids cloning into every file).
    pub(crate) global_signature_registry: Option<std::sync::Arc<SignatureRegistry>>,
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
    /// First segment of non-`std`/`crate`/`super`/`self` `use` paths (`tokio`, `axum`, `serde`).
    /// MethodCall on these identifiers uses `::`, not a hardcoded crate-name list.
    pub(crate) imported_path_roots: std::collections::HashSet<String>,
    /// Simple names of all extern (FFI) functions across all modules.
    /// Used by codegen to wrap calls in `unsafe {}` even when signature lookup fails.
    pub(crate) extern_function_names: std::collections::HashSet<String>,
    /// Module aliases that resolve to `ffi` paths (e.g., `use engine::ffi::input` → "input").
    /// Calls through these modules are assumed to be extern C and wrapped in `unsafe {}`.
    pub(crate) ffi_module_aliases: std::collections::HashSet<String>,
    /// Names of inline modules declared in the current program (Item::Mod).
    /// Used by generate_use to add `self::` prefix for `pub use` re-exports
    /// of items from inline sibling modules (Rust requires `self::` for these).
    pub(crate) inline_module_names: std::collections::HashSet<String>,
    /// Methods whose self receiver was upgraded from Borrowed to MutBorrowed
    /// during codegen (body-modification analysis). Used to update registry
    /// before writing metadata so cross-file builds see correct ownership.
    /// Key: qualified method name (e.g., "UnifiedRenderer::render_mesh").
    pub(crate) self_receiver_upgrades: std::collections::HashMap<String, OwnershipMode>,
    /// IR cutover configuration: which categories read from IR SafetyType instead of AnalyzedFunction.
    /// When all flags are true, the IR pipeline is the sole source of truth.
    pub(crate) ir_cutover: IrCutoverConfig,
    /// Per-function IR data, populated when the IR pipeline runs alongside legacy codegen.
    /// When set and a cutover flag is enabled, the corresponding codegen reads from here.
    pub(crate) current_ir_function: Option<crate::ir::IrFunction>,
    /// Full IR module (all functions), set when cutover is active. Functions are looked up
    /// by name when codegen begins processing each AnalyzedFunction.
    pub(crate) ir_module_functions: Vec<crate::ir::IrFunction>,
    /// Hard errors for boundary calls with no registry signature (fail closed — no guesses).
    pub(crate) boundary_signature_errors: std::cell::RefCell<Vec<String>>,
}

/// Configuration for incremental IR cutover.
/// Each flag controls whether a specific category of codegen decisions reads from
/// the IR `SafetyType` (true) or the legacy `AnalyzedFunction` fields (false).
#[derive(Debug, Clone, Default)]
pub struct IrCutoverConfig {
    /// Read parameter ownership from `IrFunction.param_types[name].ownership`
    pub ownership: bool,
    /// Read clone requirements from `IrFunction.optimizations.clone_annotations`
    pub clones: bool,
    /// Read parameter types from `IrFunction.param_types[name].base`
    pub param_types: bool,
    /// Read str_ref optimization from `IrFunction.str_ref_params`
    pub str_ref: bool,
    /// Use IR SafetyType for call-site argument coercions (via `ir::coercion`)
    pub call_sites: bool,
    /// Read local variable types from solver-resolved IR
    pub locals: bool,
}

impl IrCutoverConfig {
    pub fn all_enabled(&self) -> bool {
        self.ownership
            && self.clones
            && self.param_types
            && self.str_ref
            && self.call_sites
            && self.locals
    }

    pub fn all_disabled(&self) -> bool {
        !self.ownership
            && !self.clones
            && !self.param_types
            && !self.str_ref
            && !self.call_sites
            && !self.locals
    }

    /// Load configuration. Production flags default on via env (except `call_sites`,
    /// which is always on — the `!call_sites` heuristic tails have been deleted).
    /// Set `WJ_IR_CUTOVER_DISABLE_<FLAG>=1` to disable individual remaining flags.
    /// `Default` still keeps every flag off for isolated unit tests.
    pub fn from_env() -> Self {
        Self {
            ownership: !std::env::var("WJ_IR_CUTOVER_DISABLE_OWNERSHIP").is_ok_and(|v| v == "1"),
            clones: !std::env::var("WJ_IR_CUTOVER_DISABLE_CLONES").is_ok_and(|v| v == "1"),
            param_types: !std::env::var("WJ_IR_CUTOVER_DISABLE_PARAM_TYPES")
                .is_ok_and(|v| v == "1"),
            str_ref: !std::env::var("WJ_IR_CUTOVER_DISABLE_STR_REF").is_ok_and(|v| v == "1"),
            call_sites: true,
            locals: !std::env::var("WJ_IR_CUTOVER_DISABLE_LOCALS").is_ok_and(|v| v == "1"),
        }
    }
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

    /// Bare `error` after `use std::log` → `log_mod::error` when that import uniquely provides it.
    pub(in crate::codegen::rust) fn imported_runtime_qualified_callee(
        &self,
        func_name: &str,
    ) -> Option<String> {
        use crate::analyzer::stdlib_method_traits::unique_imported_runtime_callee_key;
        let imports = &self.runtime_std_module_imports;
        unique_imported_runtime_callee_key(func_name, imports, &self.signature_registry)
            .or_else(|| {
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| unique_imported_runtime_callee_key(func_name, imports, g))
            })
            .or_else(|| {
                unique_imported_runtime_callee_key(
                    func_name,
                    imports,
                    crate::analyzer::SignatureRegistry::stdlib(),
                )
            })
    }

    /// Bare `error()` after `use std::log::*` must use `log::error` / `log_mod::error`,
    /// not a colliding runtime homonym (`http::error`, `dialog::error`).
    pub(in crate::codegen::rust) fn imported_runtime_std_signature(
        &self,
        registry: &crate::analyzer::SignatureRegistry,
        callee_name: &str,
    ) -> Option<crate::analyzer::FunctionSignature> {
        let key = self.imported_runtime_qualified_callee(callee_name)?;
        registry.get_signature(&key).cloned().or_else(|| {
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(&key).cloned())
        })
    }

    /// True when `name` is a parameter, `let` binding, or match-arm binding in the current fn.
    pub(in crate::codegen::rust) fn identifier_is_local_binding(&self, name: &str) -> bool {
        self.local_var_types.contains_key(name)
            || self.match_arm_bindings.contains(name)
            || self.current_function_params.iter().any(|p| p.name == name)
    }

    /// Identifier should use `::` for associated/static calls (`tokio::spawn`, `Vec::new`).
    /// Import- and declaration-driven — not a crate/method name list.
    pub(in crate::codegen::rust) fn identifier_is_static_call_root(&self, name: &str) -> bool {
        if self.identifier_is_local_binding(name) {
            return false;
        }
        if name == "std" || name == "Self" || name.contains('.') {
            return true;
        }
        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
            return true;
        }
        self.is_imported_runtime_std_module(name)
            || self.imported_path_roots.contains(name)
            || self.inline_module_names.contains(name)
            || self.module_alias_map.contains_key(name)
            || self.ffi_module_aliases.contains(name)
    }

    /// Single-file inline `mod gpu { … }` callees (`gpu::load_shader`) — registry cannot
    /// prove module origin, so string literal coercion stays conservative (no `.to_string()`).
    pub(in crate::codegen::rust) fn inline_module_qualified_call(&self, func_name: &str) -> bool {
        func_name
            .split("::")
            .next()
            .is_some_and(|module| self.inline_module_names.contains(module))
    }

    pub fn new(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        let extern_fn_names = registry.collect_all_extern_names();
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
            serde_available: false,
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
            preregistered_impl_sibling_types: std::collections::HashSet::new(),
            struct_method_ast_formal_param_types: std::collections::HashMap::new(),
            free_function_ast_formal_param_types: std::collections::HashMap::new(),
            free_function_ast_param_field_written: std::collections::HashMap::new(),
            struct_has_owned_key_field_lookup: std::collections::HashSet::new(),
            struct_has_owned_key_sibling_wrapper: std::collections::HashSet::new(),
            struct_method_ast_param_field_written: std::collections::HashMap::new(),
            current_impl_instance_methods: std::collections::HashSet::new(),
            current_impl_consuming_self_methods: std::collections::HashSet::new(),
            trivial_copy_field_accessors: std::collections::HashSet::new(),
            current_impl_generic_type_params: Vec::new(),
            in_impl_block: false,
            usize_struct_fields: std::collections::HashMap::new(),
            method_return_types: std::collections::HashMap::new(),
            current_function_params: Vec::new(),
            current_function_name: None,
            current_function_type_bounds: Vec::new(),
            current_function_return_type: None,
            current_function_body: Vec::new(),
            full_function_body_snapshot: Vec::new(),
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
            module_string_consts: std::collections::HashSet::new(),
            loop_body_depth: 0,
            unused_let_bindings: std::collections::HashSet::new(),
            inferred_borrowed_params: std::collections::HashSet::new(),
            inferred_mut_borrowed_params: std::collections::HashSet::new(),
            str_ref_optimized_params: std::collections::HashSet::new(),
            collection_key_owned_params: std::collections::HashSet::new(),
            emitted_rust_ref_formals: std::collections::HashSet::new(),
            function_emitted_mut_arg_indices: std::collections::HashMap::new(),
            current_fn_emitted_mut_arg_indices: std::collections::HashSet::new(),
            current_fn_mixed_forwarder_params: std::collections::HashSet::new(),
            current_fn_forward_ref_if_params: std::collections::HashSet::new(),
            current_func_is_pure_forwarding_delegate: false,
            in_user_written_closure: false,
            user_closure_params: std::collections::HashSet::new(),
            closure_predicate_typed_params: false,
            generating_assignment_target: false,
            assignment_float_target_type: None,
            collect_target_type: None,
            in_void_block: false,
            in_explicit_clone_call: false,
            suppress_borrowed_clone: false,
            in_field_access_object: false,
            in_index_context: false,
            in_call_argument_generation: false,
            in_borrow_context: false,
            in_if_condition: false,
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
            explicit_copy_types_registry: std::collections::HashSet::new(),
            non_copy_types_registry: std::collections::HashSet::new(),
            types_with_drop: std::collections::HashSet::new(),
            in_struct_literal_field: false,
            in_owned_value_context: false,
            in_unsafe_block: false,
            current_struct_literal_name: None,
            current_struct_field_name: None,
            numeric_inference: None,
            method_param_ownership: std::collections::HashMap::new(),
            method_signatures_by_type: std::collections::HashMap::new(),
            stdlib_method_signatures:
                crate::codegen::rust::stdlib_method_signatures::init_stdlib_method_signatures(),
            enum_variant_types: std::collections::HashMap::new(),
            stdlib_type_rust_paths: std::collections::HashMap::new(),
            enum_variant_struct_fields: std::collections::HashMap::new(),
            library_source_root: None,
            global_signature_registry: None,
            type_defining_modules: std::collections::HashMap::new(),
            extern_submodule_qualifiers: std::collections::HashMap::new(),
            import_aliases: std::collections::HashSet::new(),
            module_alias_map: std::collections::HashMap::new(),
            runtime_std_module_imports: std::collections::HashSet::new(),
            imported_path_roots: std::collections::HashSet::new(),
            extern_function_names: extern_fn_names,
            ffi_module_aliases: std::collections::HashSet::new(),
            inline_module_names: std::collections::HashSet::new(),
            self_receiver_upgrades: std::collections::HashMap::new(),
            ir_cutover: IrCutoverConfig::from_env(),
            current_ir_function: None,
            ir_module_functions: Vec::new(),
            boundary_signature_errors: std::cell::RefCell::new(Vec::new()),
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
    pub fn set_serde_available(&mut self, available: bool) {
        self.serde_available = available;
    }

    pub fn set_copy_types_registry(&mut self, registry: std::collections::HashSet<String>) {
        self.copy_types_registry = registry;
    }

    pub fn set_explicit_copy_types_registry(
        &mut self,
        registry: std::collections::HashSet<String>,
    ) {
        self.explicit_copy_types_registry = registry;
    }

    pub fn is_explicitly_copy_type(&self, ty: &crate::parser::ast::types::Type) -> bool {
        if let crate::parser::ast::types::Type::Custom(name) = ty {
            self.explicit_copy_types_registry.contains(name.as_str())
                || name
                    .split("::")
                    .last()
                    .is_some_and(|b| self.explicit_copy_types_registry.contains(b))
        } else {
            false
        }
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

    /// Register stdlib type → Rust path mappings for FQ enum/struct references.
    pub fn set_stdlib_type_rust_paths(&mut self, paths: std::collections::HashMap<String, String>) {
        self.stdlib_type_rust_paths.extend(paths);
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

        // Try leaf type name for cross-crate qualified types (e.g. "engine::bt::BehaviorTree" → "BehaviorTree")
        let leaf = receiver_type.rsplit("::").next().unwrap_or(receiver_type);
        if leaf != receiver_type {
            if let Some(methods) = self.method_signatures_by_type.get(leaf) {
                if let Some(sig) = methods.get(method_name) {
                    return Some(sig);
                }
            }
        }

        // Then check stdlib methods (Vec, String, HashMap, etc.)
        let base = receiver_type.split('<').next().unwrap_or(receiver_type);
        let short = base.rsplit("::").next().unwrap_or(base);
        for key in [receiver_type, base, short] {
            if let Some(methods) = self.stdlib_method_signatures.get(key) {
                if let Some(sig) = methods.get(method_name) {
                    return Some(sig);
                }
            }
        }

        None
    }

    /// Map `Self::method` qualifiers to the enclosing impl struct for signature lookup.
    pub(crate) fn signature_lookup_receiver_type(&self, qualifier: &str) -> String {
        if qualifier == "Self" {
            self.current_struct_name
                .clone()
                .unwrap_or_else(|| qualifier.to_string())
        } else {
            qualifier.to_string()
        }
    }

    /// Method signature for call-site lowering: local registry, then cross-module global.
    pub(crate) fn resolve_method_function_signature(
        &self,
        receiver_type: &str,
        method: &str,
        arg_count: usize,
    ) -> Option<crate::analyzer::FunctionSignature> {
        let qualified = format!("{receiver_type}::{method}");
        let finalize =
            crate::codegen::rust::call_signature_resolution::finalize_call_site_signature;

        let local = self.signature_registry.get_signature(&qualified);
        let global_only = self
            .global_signature_registry
            .as_ref()
            .and_then(|g| g.get_signature(&qualified));

        let pick = |local: Option<&crate::analyzer::FunctionSignature>,
                    global: Option<&crate::analyzer::FunctionSignature>|
         -> Option<crate::analyzer::FunctionSignature> {
            match (local, global) {
                (Some(l), Some(g)) if g.emitted_rust_ref_params.is_some()
                    && l.emitted_rust_ref_params.is_none() =>
                {
                    Some(finalize(g.clone()))
                }
                (Some(l), Some(g)) if l.emitted_rust_ref_params.is_some()
                    && g.emitted_rust_ref_params.is_none() =>
                {
                    Some(finalize(l.clone()))
                }
                (Some(l), Some(g))
                    if crate::codegen::rust::signature_promotion::emitted_owned_beats_stale_global_borrow(
                        g, l,
                    ) =>
                {
                    Some(finalize(g.clone()))
                }
                (Some(l), Some(g))
                    if crate::codegen::rust::signature_promotion::emitted_owned_beats_stale_global_borrow(
                        l, g,
                    ) =>
                {
                    Some(finalize(l.clone()))
                }
                (Some(l), Some(g))
                    if crate::codegen::rust::signature_promotion::converged_has_reference_params_over_bare(
                        &l, &g,
                    ) =>
                {
                    Some(finalize(g.clone()))
                }
                (Some(l), Some(g))
                    if crate::codegen::rust::signature_promotion::prefer_converged_over_stub(l, g) =>
                {
                    Some(finalize(g.clone()))
                }
                (Some(l), Some(g))
                    if crate::codegen::rust::signature_promotion::prefer_converged_over_stub(g, l) =>
                {
                    Some(finalize(l.clone()))
                }
                // Caller-file stubs can carry `emitted_rust_ref_params = [false, …]` while
                // the defining module's global refresh recorded `[false, true]` for unused
                // string formals (`Squad::new` leader_id → `&str`). Merge global refresh
                // instead of blindly preferring the importer's all-false stub.
                (Some(l), Some(g))
                    if l.emitted_rust_ref_params.is_some()
                        && g.emitted_rust_ref_params.is_some() =>
                {
                    let mut merged = l.clone();
                    crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                        &mut merged, g,
                    );
                    Some(finalize(merged))
                }
                (Some(l), _) => Some(finalize(l.clone())),
                (None, Some(g)) => Some(finalize(g.clone())),
                (None, None) => None,
            }
        };

        let mut resolved = if let Some(sig) = pick(local, global_only) {
            Some(sig)
        } else if let Some(sig) =
            self.find_method_on_receiver_with_global(receiver_type, method, arg_count)
        {
            Some(finalize(sig.clone()))
        } else if let Some(ms) = self.lookup_method_signature(receiver_type, method) {
            Some(finalize(ms.to_function_signature()))
        } else {
            None
        }?;

        if let Some(recv_ty) =
            crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                Some(receiver_type),
                None,
                self.current_function_return_type.as_ref(),
            )
        {
            crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                &mut resolved,
                &recv_ty,
            );
        }
        // Importer stubs often lack `emitted_rust_ref_params` while the defining module
        // published `[true]` for `&Vec` / `&str` formals (regression-049 `from_bytes`).
        if let Some(g) = global_only {
            if g.emitted_rust_ref_params
                .as_ref()
                .is_some_and(|flags| flags.iter().any(|&f| f))
            {
                crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                    &mut resolved,
                    g,
                );
            } else {
                for pidx in 0..g.param_ownership.len() {
                    if let Some(upgraded) =
                        crate::codegen::rust::signature_promotion::prefer_shared_ref_signature(
                            Some(resolved.clone()),
                            Some(g),
                            pidx,
                        )
                    {
                        resolved = upgraded;
                    }
                }
            }
        }
        if let Some(refreshed) =
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                global_only.cloned(),
                Some(resolved.clone()),
                local.cloned(),
            ])
        {
            resolved = refreshed;
        }
        Some(resolved)
    }

    /// Resolve a method `FunctionSignature` and specialize stdlib generics from a
    /// concrete receiver `Type` (e.g. `Vec<String>` → `push(String)`).
    pub(crate) fn resolve_method_function_signature_specialized(
        &self,
        receiver_type_name: &str,
        method: &str,
        arg_count: usize,
        receiver_ty: Option<&Type>,
    ) -> Option<crate::analyzer::FunctionSignature> {
        let mut sig =
            self.resolve_method_function_signature(receiver_type_name, method, arg_count)?;
        if let Some(recv_ty) =
            crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                Some(receiver_type_name),
                receiver_ty,
                self.current_function_return_type.as_ref(),
            )
        {
            crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                &mut sig,
                &recv_ty,
            );
        }
        Some(sig)
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

    /// Set unified numeric inference for expression-level float/int type resolution.
    pub fn set_numeric_inference(
        &mut self,
        inference: crate::ir::numeric_bridge::UnifiedNumericInference,
    ) {
        self.numeric_inference = Some(std::sync::Arc::new(inference));
    }

    /// Share one global unified numeric inference across library codegen passes.
    pub fn set_shared_numeric_inference(
        &mut self,
        inference: std::sync::Arc<crate::ir::numeric_bridge::UnifiedNumericInference>,
    ) {
        self.numeric_inference = Some(inference);
    }

    /// Backwards-compatible: wrap a FloatInference into UnifiedNumericInference.
    pub fn set_float_inference(&mut self, float_inf: crate::type_inference::FloatInference) {
        let unified =
            crate::ir::numeric_bridge::UnifiedNumericInference::from_float_only(float_inf);
        self.numeric_inference = Some(std::sync::Arc::new(unified));
    }

    /// Backwards-compatible: wrap an IntInference into UnifiedNumericInference.
    pub fn set_int_inference(&mut self, int_inf: crate::type_inference::IntInference) {
        let unified = crate::ir::numeric_bridge::UnifiedNumericInference::from_int_only(int_inf);
        self.numeric_inference = Some(std::sync::Arc::new(unified));
    }

    /// Set the IR function data for the current function being generated.
    /// When IR cutover flags are enabled, codegen reads from this instead of AnalyzedFunction.
    pub fn set_current_ir_function(&mut self, ir_fn: Option<crate::ir::IrFunction>) {
        self.current_ir_function = ir_fn;
    }

    /// Set IR cutover configuration explicitly (for testing).
    pub fn set_ir_cutover_config(&mut self, config: IrCutoverConfig) {
        self.ir_cutover = config;
    }

    /// Store an IR module for cutover. The codegen will look up the right IrFunction
    /// by name when processing each AnalyzedFunction.
    pub fn set_ir_module(&mut self, module: crate::ir::pipeline::IrModule) {
        self.ir_module_functions = module.functions;
    }

    /// Attach solver-resolved IR functions for this file (multipass library builds).
    pub fn set_ir_functions(&mut self, functions: Vec<crate::ir::node::IrFunction>) {
        self.ir_module_functions = functions;
    }

    /// Select the IrFunction matching the current function being generated.
    /// Called at the start of each function's codegen.
    pub(crate) fn select_ir_function_for(&mut self, func_name: &str) {
        if self.ir_module_functions.is_empty() {
            self.current_ir_function = None;
            return;
        }
        let qualified = self
            .current_struct_name
            .as_ref()
            .map(|t| format!("{t}::{func_name}"))
            .unwrap_or_else(|| func_name.to_string());
        self.current_ir_function = self
            .ir_module_functions
            .iter()
            .find(|f| f.name == qualified || f.name == func_name)
            .cloned();
    }

    /// Get parameter ownership, preferring IR data when cutover is enabled.
    /// Falls back to `AnalyzedFunction.inferred_ownership` when IR data is unavailable
    /// or the ownership cutover flag is off.
    pub(crate) fn get_param_ownership<'a>(
        &self,
        param_name: &str,
        analyzed: &'a AnalyzedFunction<'_>,
    ) -> Option<OwnershipMode> {
        if self.ir_cutover.ownership {
            if let Some(ir_fn) = &self.current_ir_function {
                if let Some(safety_ty) = ir_fn.param_types.get(param_name) {
                    return Some(owned_type_to_ownership_mode(&safety_ty.ownership));
                }
            }
        }
        analyzed.inferred_ownership.get(param_name).copied()
    }

    /// Definitive IR param ownership (excludes `OwnedType::Inferred`).
    /// When present, formals must not re-walk the body for keep-owned heuristics.
    pub(crate) fn ir_param_ownership_definitive(&self, param_name: &str) -> Option<OwnershipMode> {
        if !self.ir_cutover.ownership {
            return None;
        }
        let ir_fn = self.current_ir_function.as_ref()?;
        let safety_ty = ir_fn.param_types.get(param_name)?;
        if matches!(
            safety_ty.ownership,
            crate::ir::safety_type::OwnedType::Inferred
        ) {
            return None;
        }
        Some(owned_type_to_ownership_mode(&safety_ty.ownership))
    }

    /// Ownership for formal demotion / str_ref.
    ///
    /// Prefer definitive IR borrows (`Ref` / `MutRef`). Analyzer `Borrowed` /
    /// `MutBorrowed` still beats stale IR `Owned` (solver often leaves Owned
    /// before multipass convergence). Body-walk keep-owned must not override
    /// IR borrows — see `ir_param_ownership_definitive` at formal emit.
    pub(crate) fn param_ownership_for_formal_demotion(
        &self,
        param_name: &str,
        analyzed: &AnalyzedFunction<'_>,
    ) -> Option<OwnershipMode> {
        let analyzer = analyzed.inferred_ownership.get(param_name).copied();
        let ir = self.ir_param_ownership_definitive(param_name);
        // Mutated+returned / identity formals: Owned beats stale IR MutRef (solver lattice).
        if analyzed.returned_parameters.contains(param_name) {
            return Some(OwnershipMode::Owned);
        }
        if matches!(analyzer, Some(OwnershipMode::Owned))
            && matches!(ir, Some(OwnershipMode::MutBorrowed) | None)
        {
            return analyzer;
        }
        match (analyzer, ir) {
            (_, Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)) => ir,
            (
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed),
                Some(OwnershipMode::Owned) | None,
            ) => analyzer,
            (_, Some(OwnershipMode::Owned)) => ir,
            _ => analyzer.or_else(|| self.get_param_ownership(param_name, analyzed)),
        }
    }

    /// Record a hard error for a module-qualified call with no registry signature.
    pub(crate) fn report_missing_boundary_signature(&self, callee_name: &str) {
        let msg = format!("missing boundary signature for `{callee_name}`");
        let mut errors = self.boundary_signature_errors.borrow_mut();
        if !errors.iter().any(|e| e == &msg) {
            errors.push(msg);
        }
    }

    /// Module-qualified free-function path (not `Type::method` / `Self::method`).
    /// Same-crate `crate::mod::fn` paths are not external boundaries — signatures
    /// register under the bare function name within the crate multipass.
    pub(crate) fn is_module_boundary_callee(callee_name: &str) -> bool {
        let Some((qualifier, _)) = callee_name.rsplit_once("::") else {
            return false;
        };
        if qualifier == "Self" || qualifier.ends_with("::Self") {
            return false;
        }
        if qualifier == "crate" || qualifier.starts_with("crate::") {
            return false;
        }
        let last = qualifier.rsplit("::").next().unwrap_or(qualifier);
        last.chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
    }

    pub fn ir_cutover_call_sites_enabled(&self) -> bool {
        self.ir_cutover.call_sites
    }

    /// Check if a parameter name has ownership info (in IR or analyzer).
    pub(crate) fn has_param_ownership(
        &self,
        param_name: &str,
        analyzed: &AnalyzedFunction<'_>,
    ) -> bool {
        if self.ir_cutover.ownership {
            if let Some(ir_fn) = &self.current_ir_function {
                if ir_fn.param_types.contains_key(param_name) {
                    return true;
                }
            }
        }
        analyzed.inferred_ownership.contains_key(param_name)
    }

    pub(crate) fn infer_call_arg_actual_safety_type(
        &self,
        arg_expr: &Expression<'ast>,
        coerced: &str,
    ) -> crate::ir::safety_type::SafetyType {
        self.infer_actual_safety_type(arg_expr, coerced)
    }

    /// Get all param ownership entries, preferring IR when cutover is enabled.
    pub(crate) fn get_all_param_ownership(
        &self,
        analyzed: &AnalyzedFunction<'_>,
    ) -> Vec<(String, OwnershipMode)> {
        if self.ir_cutover.ownership {
            if let Some(ir_fn) = &self.current_ir_function {
                return ir_fn
                    .param_types
                    .iter()
                    .map(|(name, st)| (name.clone(), owned_type_to_ownership_mode(&st.ownership)))
                    .collect();
            }
        }
        analyzed
            .inferred_ownership
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Get the effective inferred param type at a given index, preferring IR when cutover is enabled.
    ///
    /// When `param_types` cutover is on and IR has a resolved base type, emit from the
    /// analyzer's inferred parser Type that matches that IR binding (same source of truth
    /// as ownership cutover). Falls back to analyzer / declared type otherwise.
    pub(crate) fn get_effective_param_type<'b>(
        &self,
        param_idx: usize,
        param: &'b Parameter<'ast>,
        analyzed: &'b AnalyzedFunction<'ast>,
    ) -> &'b Type {
        let analyzer_ty = analyzed
            .inferred_param_types
            .get(param_idx)
            .unwrap_or(&param.type_);
        if !self.ir_cutover.param_types {
            return analyzer_ty;
        }
        let Some(ir_fn) = self.current_ir_function.as_ref() else {
            return analyzer_ty;
        };
        let Some(safety_ty) = ir_fn.param_types.get(&param.name) else {
            return analyzer_ty;
        };
        if safety_ty.base == crate::ir::safety_type::BaseType::Inferred {
            return analyzer_ty;
        }
        // IR resolved — analyzer inferred type is the parser projection of the same binding.
        analyzer_ty
    }

    /// Attach the converged crate-wide registry for lookup fallback (library multipass codegen).
    pub fn set_global_signature_registry(&mut self, registry: std::sync::Arc<SignatureRegistry>) {
        self.extern_function_names
            .extend(registry.collect_all_extern_names());
        self.global_signature_registry = Some(registry);
    }

    /// Seed mut-arg indices from prior files' codegen refresh (cross-file `&mut` call sites).
    pub fn merge_function_emitted_mut_arg_indices(
        &mut self,
        indices: &std::collections::HashMap<String, std::collections::HashSet<usize>>,
    ) {
        for (name, set) in indices {
            self.function_emitted_mut_arg_indices
                .entry(name.clone())
                .or_default()
                .extend(set.iter().copied());
        }
    }

    pub(crate) fn get_signature_with_global(&self, name: &str) -> Option<&FunctionSignature> {
        let local = self.signature_registry.get_signature(name);
        let global = self
            .global_signature_registry
            .as_ref()
            .and_then(|g| g.get_signature(name));
        match (local, global) {
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::emitted_owned_beats_stale_global_borrow(
                    g, l,
                ) =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::emitted_owned_beats_stale_global_borrow(
                    l, g,
                ) =>
            {
                Some(l)
            }
            (Some(l), Some(g)) if g.emitted_rust_ref_params.is_some()
                && l.emitted_rust_ref_params.is_none() =>
            {
                Some(g)
            }
            (Some(l), Some(g)) if l.emitted_rust_ref_params.is_some()
                && g.emitted_rust_ref_params.is_none() =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::codegen_refreshed_beats_analysis_only(
                    g, l,
                ) =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::codegen_refreshed_beats_analysis_only(
                    l, g,
                ) =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::shared_ref_emission_beats(g, l) =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::shared_ref_emission_beats(l, g) =>
            {
                Some(l)
            }
            (Some(l), _) => Some(l),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        }
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
        self.global_signature_registry
            .as_ref()
            .and_then(|g| g.find_method_on_receiver_type(type_name, method, arg_count))
    }

    /// Whether converged registry metadata expects `&T` for a method argument.
    pub(crate) fn method_registry_arg_expects_shared_borrow(
        &self,
        receiver_type: &str,
        method: &str,
        arg_index: usize,
        user_arg_count: usize,
    ) -> bool {
        let Some(sig) =
            self.resolve_method_function_signature(receiver_type, method, user_arg_count)
        else {
            return false;
        };
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&sig, pidx) {
            return false;
        }
        crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(&sig, pidx)
    }

    pub(crate) fn find_signature_by_name_and_arg_count_with_global(
        &self,
        name: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        let local = self
            .signature_registry
            .find_signature_by_name_and_arg_count(name, arg_count);
        let global = self
            .global_signature_registry
            .as_ref()
            .and_then(|g| g.find_signature_by_name_and_arg_count(name, arg_count));
        // Same refresh preference as `get_signature_with_global`: defining-module
        // owned emission must beat importer analysis stubs (create→post AppDeps).
        match (local, global) {
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::emitted_owned_beats_stale_global_borrow(
                    g, l,
                ) =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::emitted_owned_beats_stale_global_borrow(
                    l, g,
                ) =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if g.emitted_rust_ref_params.is_some() && l.emitted_rust_ref_params.is_none() =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if l.emitted_rust_ref_params.is_some() && g.emitted_rust_ref_params.is_none() =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::codegen_refreshed_beats_analysis_only(
                    g, l,
                ) =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::codegen_refreshed_beats_analysis_only(
                    l, g,
                ) =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if !crate::codegen::rust::signature_promotion::signature_is_wj_std_stub_or_runtime_qualified(
                    l,
                )
                    && !l.formal_param_types.is_empty()
                    && crate::codegen::rust::signature_promotion::method_registry_reflects_emitted_owned(
                        l,
                    )
                    && crate::codegen::rust::signature_promotion::signature_is_wj_std_stub_or_runtime_qualified(
                        g,
                    ) =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::shared_ref_emission_beats(g, l) =>
            {
                Some(g)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::shared_ref_emission_beats(l, g) =>
            {
                Some(l)
            }
            (Some(l), Some(g))
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(g, 0)
                    && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(l, 0) =>
            {
                Some(g)
            }
            (Some(l), _) => Some(l),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        }
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
        let idx = sig.arg_param_index(arg_idx);
        matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, idx),
            crate::analyzer::OwnershipMode::Owned,
        ) && sig.formal_param_type(idx).is_some_and(|t| {
            !matches!(
                t,
                crate::parser::Type::Reference(_) | crate::parser::Type::MutableReference(_)
            ) && crate::codegen::rust::types::is_windjammer_text_type(t)
        })
    }

    pub(crate) fn resolve_call_signature_with_global(
        &self,
        func_name: &str,
        receiver_type: Option<&str>,
        arg_count: usize,
    ) -> Option<crate::codegen::rust::call_signature_resolution::ResolvedSignature> {
        let owned_name = self.imported_runtime_qualified_callee(func_name);
        let func_name = owned_name.as_deref().unwrap_or(func_name);
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
        let codegen_refresh_source = global
            .clone()
            .filter(|r| r.sig.emitted_rust_ref_params.is_some())
            .or_else(|| {
                local
                    .clone()
                    .filter(|r| r.sig.emitted_rust_ref_params.is_some())
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
        picked.map(|mut resolved| {
            if resolved.sig.emitted_rust_ref_params.is_none() {
                if let Some(alt) = &codegen_refresh_source {
                    crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                        &mut resolved.sig,
                        &alt.sig,
                    );
                }
            }
            resolved.sig =
                crate::codegen::rust::call_signature_resolution::finalize_call_site_signature(
                    resolved.sig,
                );
            resolved
        })
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

        if let Some(resolved) =
            crate::codegen::rust::call_signature_resolution::resolve_method_for_call_site(
                &self.signature_registry,
                self.global_signature_registry.as_deref(),
                receiver_type,
                method,
                arg_count,
            )
        {
            if accept_method_resolution_for_receiver(&resolved, receiver_type, method) {
                return Some(resolved.sig);
            }
        }

        if self.global_signature_registry.is_none() {
            let qualified = format!("{receiver_type}::{method}");
            if let Some(resolved) =
                self.resolve_call_signature_with_global(&qualified, Some(receiver_type), arg_count)
            {
                if accept_method_resolution_for_receiver(&resolved, receiver_type, method) {
                    return Some(resolved.sig);
                }
            }
        }

        if let Some(sig) =
            self.signature_registry
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
                && crate::codegen::rust::call_signature_resolution::validate_arg_count(
                    sig, arg_count,
                )
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

    /// Narrower collision check: only explicit ownership_collision_keys, avoiding
    /// false positives from `has_method_name_collision` on common names like "get".
    pub(crate) fn has_explicit_ownership_collision_with_global(&self, name: &str) -> bool {
        self.signature_registry
            .has_explicit_ownership_collision(name)
            || self
                .global_signature_registry
                .as_ref()
                .is_some_and(|g| g.has_explicit_ownership_collision(name))
    }

    pub(crate) fn should_skip_int_to_float_auto_cast_with_global(
        &self,
        type_name: Option<&str>,
        method: &str,
        qualified_key: Option<&str>,
    ) -> bool {
        if self.signature_registry.should_skip_int_to_float_auto_cast(
            type_name,
            method,
            qualified_key,
        ) {
            return true;
        }
        self.global_signature_registry
            .as_ref()
            .is_some_and(|g| g.should_skip_int_to_float_auto_cast(type_name, method, qualified_key))
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
            Item::Macro {
                doc_comment, expr, ..
            } => {
                let mut out = String::new();
                if let Some(doc) = doc_comment {
                    for line in doc.lines() {
                        out.push_str(&format!("// {}\n", line));
                    }
                }
                out.push_str(&self.generate_expression(expr));
                out.push('\n');
                out
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
    /// Strip a leading `&ident` when `ident` already emits as a Rust shared-ref binding.
    /// Prevents `map.get(&key)` → `&&str` for `key: &str` (Call(FieldAccess) and MethodCall).
    pub(crate) fn strip_stale_amp_on_already_ref_arg(
        &self,
        arg_expr: &Expression<'_>,
        arg_str: &mut String,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            crate::codegen::rust::call_site_borrow::strip_double_ref_on_shared_binding(
                arg_expr, arg_str, false,
            );
            return;
        };
        if self.collection_key_owned_params.contains(name.as_str()) {
            return;
        }
        // `&mut T` is not a shared ref — keep `query(&sql)` reborrows into AsRef/&str.
        if self.identifier_already_mut_ref(name) {
            return;
        }
        let text_borrowed_formal = self.inferred_borrowed_params.contains(name.as_str())
            && self.current_function_params.iter().any(|p| {
                p.name == *name && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
            });
        let already = self.emitted_rust_ref_formals.contains(name.as_str())
            || self.str_ref_optimized_params.contains(name.as_str())
            || self.identifier_already_ref(name)
            || text_borrowed_formal;
        crate::codegen::rust::call_site_borrow::strip_double_ref_on_shared_binding(
            arg_expr, arg_str, already,
        );
    }

    pub(crate) fn identifier_already_ref(&self, name: &str) -> bool {
        // `&mut T` is not a shared ref. Callers that mean "any ref" must also check
        // `identifier_already_mut_ref`. Treating mut as shared suppresses `query(&sql)`.
        if self.identifier_already_mut_ref(name) {
            return false;
        }
        // Emitted Rust `&T` formals always generate as references — even for mixed
        // forwarders that keep an owned outer formal elsewhere in the call graph.
        if self.emitted_rust_ref_formals.contains(name) {
            return true;
        }
        if self.str_ref_optimized_params.contains(name)
            && self.current_function_params.iter().any(|p| p.name == name)
        {
            return true;
        }
        if self.current_fn_mixed_forwarder_params.contains(name) {
            return false;
        }
        if self.inferred_borrowed_params.contains(name) {
            // Current fn param with owned emitted formal (e.g. `key: Key`) may appear in
            // inferred_borrowed_params for callee-forward analysis — not a Rust `&` binding.
            if let Some(p) = self.current_function_params.iter().find(|p| p.name == name) {
                if self.str_ref_optimized_params.contains(name)
                    || self.emitted_rust_ref_formals.contains(name)
                {
                    return true;
                }
                // Read-only WJ `string` keys demoted to `&str` / `&String` are already
                // shared refs — do not wait solely on emitted_rust_ref (avoids
                // `map.get(&key)` → `&&str` when the formal is `key: &str`).
                // MutBorrowed text (`sql: &mut String`) is NOT a shared ref — AsRef/&str
                // callees still need `query(&sql)`.
                if crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    && !self.collection_key_owned_params.contains(name)
                {
                    if self.inferred_mut_borrowed_params.contains(name) {
                        return false;
                    }
                    return true;
                }
                return false;
            }
            return true;
        }
        // method_param_ownership is keyed by method name — only honor params for the
        // function currently being codegen'd (not sibling methods with the same param name).
        if self.current_function_params.iter().any(|p| p.name == name) {
            if let Some(fn_name) = self.current_function_name.as_deref() {
                if let Some(pairs) = self.method_param_ownership.get(fn_name) {
                    if pairs.iter().any(|(n, o)| {
                        n == name
                            && matches!(
                                o,
                                crate::analyzer::OwnershipMode::Borrowed
                                    | crate::analyzer::OwnershipMode::MutBorrowed
                            )
                    }) {
                        // Owned emitted formal beats stale analyzer borrow metadata.
                        return self.emitted_rust_ref_formals.contains(name);
                    }
                }
            }
        }
        if self.borrowed_iterator_vars.contains(name) {
            return true;
        }
        if self.str_ref_optimized_params.contains(name) {
            // Phase-2 `&str` emission: the binding is already a shared ref at call sites.
            // Only treat as owned when an explicit keep-owned mark wins (payload stores).
            if self.collection_key_owned_params.contains(name) {
                return false;
            }
            return true;
        }
        self.current_function_params.iter().any(|p| {
            p.name == name
                && (matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                    || matches!(&p.type_, Type::Reference(_))
                    || (crate::codegen::rust::types::param_generates_as_rust_ref(
                        &p.type_,
                        &p.name,
                        &self.inferred_borrowed_params,
                    ) && !matches!(&p.type_, Type::MutableReference(_))))
        })
    }

    /// Whether an identifier already lowers as a Rust shared-reference binding (`&T`).
    ///
    /// Narrower than [`Self::identifier_already_ref`]: match/`let` bindings may appear in
    /// `inferred_borrowed_params` for callee-flow analysis without being emitted as `&` in Rust.
    pub(crate) fn binding_emits_as_rust_shared_ref(&self, name: &str) -> bool {
        if self.emitted_rust_ref_formals.contains(name) {
            return true;
        }
        if self.str_ref_optimized_params.contains(name) {
            return self.current_function_params.iter().any(|p| p.name == name);
        }
        if self.borrowed_iterator_vars.contains(name) {
            return true;
        }
        self.current_function_params.iter().any(|p| {
            p.name == name
                && (matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                    || matches!(&p.type_, Type::Reference(_)))
        })
    }

    /// Whether a named identifier already generates as `&mut T` in Rust (explicit or inferred).
    pub(crate) fn identifier_already_mut_ref(&self, name: &str) -> bool {
        if self.inferred_mut_borrowed_params.contains(name) {
            return true;
        }
        self.current_function_params
            .iter()
            .any(|p| p.name == name && matches!(&p.type_, Type::MutableReference(_)))
    }

    /// Whether `name` already codegen's as any Rust reference binding (`&T` or `&mut T`).
    ///
    /// Use at call sites when deciding whether to prefix `&` / `&mut`: existing refs
    /// reborrow/coerce (e.g. `&mut DenseCsr` → `&DenseCsr`) and must not get `&` stacked.
    pub(crate) fn identifier_binding_already_rust_ref(&self, name: &str) -> bool {
        self.identifier_already_mut_ref(name)
            || self.identifier_already_ref(name)
            || self.emitted_rust_ref_formals.contains(name)
            || self.binding_emits_as_rust_shared_ref(name)
    }

    /// Peel spurious leading `&` when a binding is already emitted as `&T` / `&mut T` and the
    /// callee slot is not owned (signature-driven; avoids `&&mut` / redundant `&csr`).
    pub(crate) fn peel_stacked_amp_on_emitted_ref_binding(
        &self,
        coerced: &mut String,
        arg_expr: &crate::parser::Expression<'ast>,
        sig: Option<&crate::analyzer::FunctionSignature>,
        arg_index: usize,
        require_leading_amp: bool,
    ) {
        let crate::parser::Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        if !self.identifier_binding_already_rust_ref(name) {
            return;
        }
        if require_leading_amp && !coerced.starts_with('&') {
            return;
        }
        let owned_slot = sig.is_some_and(|sig| {
            let pidx = sig.arg_param_index(arg_index);
            crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx)
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
        });
        if sig.is_none() || !owned_slot {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
    }

    /// True when `name` was already passed to a field-extract callee earlier in this fn body.
    pub(in crate::codegen::rust) fn param_used_in_prior_field_extract_call(
        &self,
        name: &str,
    ) -> bool {
        use crate::parser::Expression;
        if self.current_function_body.is_empty() {
            return false;
        }
        let limit = self.current_statement_idx;
        for (idx, stmt) in self.current_function_body.iter().enumerate() {
            if idx >= limit {
                break;
            }
            if self.statement_passes_param_to_field_extract_callee(stmt, name) {
                return true;
            }
        }
        false
    }

    fn statement_passes_param_to_field_extract_callee(
        &self,
        stmt: &crate::parser::Statement,
        param_name: &str,
    ) -> bool {
        use crate::parser::{Expression, Statement};
        let check_expr = |expr: &Expression| -> bool {
            match expr {
                Expression::Call {
                    function,
                    arguments,
                    ..
                } => {
                    let Some(callee_name) = (match &**function {
                        Expression::Identifier { name, .. } => Some(name.as_str()),
                        _ => None,
                    }) else {
                        return false;
                    };
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            && self.callee_param_field_extracts_by_name(callee_name, i)
                        {
                            return true;
                        }
                    }
                    false
                }
                _ => false,
            }
        };
        match stmt {
            Statement::Let { value, .. } => check_expr(value),
            Statement::Expression { expr, .. } => check_expr(expr),
            Statement::Assignment { value, .. } => check_expr(value),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn callee_param_field_extracts_by_name(
        &self,
        callee_name: &str,
        arg_index: usize,
    ) -> bool {
        let registry = match self.global_signature_registry.as_ref() {
            Some(g) => g,
            None => &self.signature_registry,
        };
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let Some(sig) = registry
            .get_signature(callee_name)
            .or_else(|| registry.lookup_method(callee_name))
            .or_else(|| registry.find_signature_ending_with(simple))
            .or_else(|| {
                self.signature_registry
                    .get_signature(callee_name)
                    .or_else(|| self.signature_registry.lookup_method(callee_name))
                    .or_else(|| self.signature_registry.find_signature_ending_with(simple))
            })
        else {
            return false;
        };
        let param_idx = sig.arg_param_index(arg_index);
        // Match `auto_clone::callee_arg_field_extracts`: field-extract demotes Move→Read
        // only for shared-ref formals. Owned WJ formals that match/project
        // (`value_tag(value: Value)`) still move — callers must `.clone()` on reuse (regression-063).
        let field_extract = sig
            .field_extract_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            .unwrap_or(false);
        if !field_extract {
            return false;
        }
        match sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
        {
            Some(true) => return true,
            Some(false) => return false,
            None => {}
        }
        // Bare WJ source formals still move. Only explicit `&T` in `formal_param_types`
        // field-extracts as Read — never trust converged `param_types` Reference wraps.
        if !sig.formal_param_types.is_empty() {
            sig.formal_param_types
                .get(param_idx)
                .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
        } else {
            false
        }
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
    /// Forward-ref guard: params that keep owned formals but borrow at self/sibling calls.
    pub(crate) fn should_borrow_forward_ref_param_at_call(
        &self,
        param_name: &str,
        receiver: &Expression<'ast>,
    ) -> bool {
        self.in_if_condition
            && self.current_fn_forward_ref_if_params.contains(param_name)
            && !self.emitted_rust_ref_formals.contains(param_name)
            && crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(receiver)
    }

    pub(crate) fn should_borrow_owned_param_in_if_condition(
        &self,
        param_name: &str,
        receiver: &Expression<'ast>,
    ) -> bool {
        self.should_borrow_forward_ref_param_at_call(param_name, receiver)
    }

    pub(crate) fn caller_keeps_owned_outer_formal(&self, param_name: &str) -> bool {
        self.current_function_params
            .iter()
            .any(|p| p.name == param_name && !self.emitted_rust_ref_formals.contains(param_name))
    }

    /// Owned non-Copy outer formals pass by move at call sites; Rust auto-borrows for `&T` callees.
    pub(crate) fn caller_owned_non_copy_formal(&self, name: &str) -> bool {
        self.current_function_params.iter().any(|p| {
            p.name == name
                && !self.emitted_rust_ref_formals.contains(name)
                && !self.is_type_copy(&p.type_)
        })
    }

    /// Owned non-Copy outer formals pass by move at call sites; Rust auto-borrows for `&T` callees
    /// unless the param is a forward-ref (used in `if` conditions / mixed forwarder branches).
    pub(crate) fn callee_call_uses_rust_auto_borrow_for_owned_struct(
        &self,
        arg_expr: &Expression<'ast>,
    ) -> bool {
        match arg_expr {
            Expression::Identifier { name, .. } => {
                if !self.caller_owned_non_copy_formal(name) {
                    return false;
                }
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                !self.current_fn_forward_ref_if_params.contains(name)
                    && !self.param_used_in_if_with_condition_and_branches(&body, name)
                    && !self.param_used_in_any_if_condition(&body, name)
            }
            _ => false,
        }
    }

    /// Borrow/clone coercion for forward-ref and mixed-forwarder facade params at self calls.
    pub(crate) fn apply_forward_ref_and_mixed_forwarder_call_coercion(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        receiver: Option<&Expression<'ast>>,
        callee_wants_shared_borrow: bool,
        callee_wants_owned: bool,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        let Some(recv) = receiver else {
            return;
        };
        if !crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(recv) {
            return;
        }
        if self.emitted_rust_ref_formals.contains(name) {
            if coerced.ends_with(".clone()") {
                *coerced = coerced.trim_end_matches(".clone()").trim().to_string();
            }
            return;
        }
        let caller_owned = self.caller_keeps_owned_outer_formal(name);
        if self.in_if_condition && caller_owned {
            // Copy / owned-pass formals must stay by-value (`search.update(dt)` not `&dt`).
            // Forward-ref borrowing in `if` is only for non-Copy values that would move.
            // Copy aggregates (`through: Lsn`) are not scalar pass-by-value but still
            // must not gain `&through` into owned formals (regression-060).
            let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                p.name == *name
                    && self.is_type_copy(&p.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
            });
            if self.caller_param_is_copy_pass_by_value(name) || caller_copy_aggregate {
                return;
            }
            if coerced.ends_with(".clone()") {
                let base = coerced.trim_end_matches(".clone()").trim();
                *coerced = format!("&{base}");
            } else if !coerced.starts_with('&') {
                *coerced = format!("&{coerced}");
            }
            return;
        }
        let body: Vec<_> = self.current_function_body.iter().copied().collect();
        let is_forward_ref = self.current_fn_forward_ref_if_params.contains(name)
            || self.param_used_in_if_with_condition_and_branches(&body, name)
            || self.param_used_in_any_if_condition(&body, name);
        let is_mixed = self.current_fn_mixed_forwarder_params.contains(name) || is_forward_ref;
        if !caller_owned || (!is_forward_ref && !is_mixed) {
            if !self.in_if_condition
                && caller_owned
                && !is_forward_ref
                && !self.current_fn_forward_ref_if_params.is_empty()
                && callee_wants_owned
                && !callee_wants_shared_borrow
                && !coerced.starts_with('&')
            {
                // Do not borrow an owned arg for an owned callee just because a
                // *sibling* param is a forward-ref — that forces a later `.clone()`.
                return;
            }
            if !self.in_if_condition
                && caller_owned
                && callee_wants_shared_borrow
                && !coerced.starts_with('&')
                && !self.callee_call_uses_rust_auto_borrow_for_owned_struct(arg_expr)
            {
                *coerced = format!("&{coerced}");
                return;
            }
            if !self.in_if_condition
                && caller_owned
                && callee_wants_owned
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                let base = coerced.trim_start_matches('&');
                *coerced = if base.ends_with(".clone()") {
                    base.to_string()
                } else {
                    format!("{base}.clone()")
                };
            }
            return;
        }
        if self.in_if_condition && is_forward_ref {
            if self.caller_param_is_copy_pass_by_value(name) {
                return;
            }
            if coerced.ends_with(".clone()") {
                let base = coerced.trim_end_matches(".clone()").trim();
                *coerced = format!("&{base}");
            } else if !coerced.starts_with('&') {
                *coerced = format!("&{coerced}");
            }
            return;
        }
        if self.in_if_condition {
            if callee_wants_owned {
                if let Expression::Identifier { name, .. } = arg_expr {
                    if self.caller_owned_non_copy_formal(name) && !coerced.ends_with(".clone()") {
                        *coerced = format!("{coerced}.clone()");
                    }
                }
            }
            return;
        }
        if callee_wants_shared_borrow {
            if !coerced.starts_with('&')
                && !(self.callee_call_uses_rust_auto_borrow_for_owned_struct(arg_expr)
                    && !is_forward_ref)
            {
                if coerced.ends_with(".clone()") {
                    let base = coerced.trim_end_matches(".clone()").trim();
                    *coerced = format!("&{base}");
                } else {
                    *coerced = format!("&{coerced}");
                }
            }
        } else if callee_wants_owned
            && (coerced.starts_with("&mut ")
                || (coerced.starts_with('&') && !coerced.starts_with("&mut ")))
        {
            *coerced =
                crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(coerced);
        }
    }

    /// Final call-site pass for owned outer formals: borrow in `if` conditions, clone for owned callees elsewhere.
    pub(crate) fn finalize_owned_outer_formal_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_wants_shared_borrow: bool,
        callee_wants_owned: bool,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        if !self.caller_keeps_owned_outer_formal(name) {
            return;
        }
        if self.in_if_condition {
            // Signature-driven: never borrow Copy / owned-pass args in `if` conditions.
            // The forward-ref heuristic exists to avoid moving non-Copy formals; Copy
            // and callee-owned contracts must remain by-value (`update(dt)` not `&dt`).
            // Copy aggregates (`through: Lsn`) are not `is_copy_pass_by_value_formal`
            // but still pass by value into owned formals (regression-060).
            let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                p.name == *name
                    && self.is_type_copy(&p.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
            });
            if self.caller_param_is_copy_pass_by_value(name) || caller_copy_aggregate {
                return;
            }
            // Callee owned formal: shared `&binding` / clone→borrow is never valid
            // (ReBAC `contains_string(out)` into `items: Vec<String>` inside `if`).
            // Reuse after the condition requires `.clone()`, not `&`.
            if callee_wants_owned && !callee_wants_shared_borrow {
                if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                    *coerced =
                        crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            coerced,
                        );
                } else if !coerced.ends_with(".clone()")
                    && !coerced.starts_with("&mut ")
                    && (self.current_fn_forward_ref_if_params.contains(name)
                        || self.param_used_in_if_with_condition_and_branches(
                            &self
                                .current_function_body
                                .iter()
                                .copied()
                                .collect::<Vec<_>>(),
                            name,
                        ))
                {
                    *coerced = format!("{coerced}.clone()");
                }
                return;
            }
            if !callee_wants_shared_borrow
                && !self.current_fn_forward_ref_if_params.contains(name)
                && !self.param_used_in_if_with_condition_and_branches(
                    &self
                        .current_function_body
                        .iter()
                        .copied()
                        .collect::<Vec<_>>(),
                    name,
                )
            {
                return;
            }
            if !coerced.starts_with('&') {
                if coerced.ends_with(".clone()") {
                    let base = coerced.trim_end_matches(".clone()").trim();
                    *coerced = format!("&{base}");
                } else {
                    *coerced = format!("&{coerced}");
                }
            }
            return;
        }
        if callee_wants_owned
            && !callee_wants_shared_borrow
            && (coerced.starts_with("&mut ") || coerced.starts_with('&'))
        {
            // Match-arm bindings into owned formals must move (strip `&`), same as
            // other owned slots — do not preserve a stale shared borrow.
            *coerced =
                crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(coerced);
        } else if callee_wants_owned
            && !callee_wants_shared_borrow
            && !coerced.ends_with(".clone()")
            && !coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && (self.current_fn_forward_ref_if_params.contains(name)
                || self.current_fn_mixed_forwarder_params.contains(name))
        {
            // Bare owned formals move by value. Only clone when auto-clone says this
            // statement reuses the binding — never unconditional `.clone()` on
            // `local.merge(remote)` / other single-use owned method args.
            let needs_reuse = self
                .auto_clone_analysis
                .as_ref()
                .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());
            if needs_reuse {
                *coerced = format!("{coerced}.clone()");
            }
        } else if callee_wants_shared_borrow
            && !callee_wants_owned
            && !coerced.starts_with('&')
            && !self.callee_call_uses_rust_auto_borrow_for_owned_struct(arg_expr)
        {
            *coerced = format!("&{coerced}");
        }
    }

    /// Strip spurious borrow/clone for pure forwarding delegates at the final call arg.
    pub(crate) fn pure_forwarding_strip_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
    ) {
        if !self.current_func_is_pure_forwarding_delegate {
            return;
        }
        let forwarded = match arg_expr {
            Expression::Identifier { name, .. } => self
                .current_function_params
                .iter()
                .any(|p| p.name == *name && p.name != "self"),
            Expression::FieldAccess { object, .. } => {
                crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(object)
            }
            _ => false,
        };
        if !forwarded {
            return;
        }
        if matches!(arg_expr, Expression::Identifier { .. }) {
            if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                *coerced = coerced[1..].to_string();
            }
        }
        if coerced.ends_with(".clone()") {
            *coerced = coerced.trim_end_matches(".clone()").trim().to_string();
        }
    }

    /// Pure-forwarding strip, but keep callee-required borrows (asymmetric facade calls).
    pub(crate) fn maybe_pure_forwarding_strip_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        _receiver_type: Option<&str>,
        _method: Option<&str>,
        arg_index: Option<usize>,
        _user_arg_count: Option<usize>,
        callee_sig: Option<&crate::analyzer::FunctionSignature>,
    ) {
        if !self.current_func_is_pure_forwarding_delegate {
            return;
        }
        if let (Some(sig), Some(arg_idx)) = (callee_sig, arg_index) {
            let pidx = sig.arg_param_index(arg_idx);
            if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, pidx,
            ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx)
                || crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                    sig, arg_idx,
                )
            {
                return;
            }
        }
        // Single-expression owned-formal delegates (TxnManager::seed_write) must not keep
        // stale registry `&` from callee body analysis (MemoryEngine::seed_write key.bytes.len()).
        self.pure_forwarding_strip_call_arg(coerced, arg_expr);
    }

    /// AST-driven fallback: borrow owned forward-ref params in `if` condition call sites.
    pub(crate) fn coerce_forward_ref_params_in_if_condition(
        &self,
        condition: &Expression<'ast>,
        mut cond_str: String,
    ) -> String {
        let body: Vec<_> = self.current_function_body.iter().copied().collect();
        for param in &self.current_function_params {
            if param.name == "self" || self.emitted_rust_ref_formals.contains(&param.name) {
                continue;
            }
            // Copy scalars and Copy aggregates (Lsn, …) pass by value at call sites —
            // do not rewrite to `(&param)` in if conditions (regression-060).
            if self.is_type_copy(&param.type_) {
                continue;
            }
            let forward_ref = self.current_fn_forward_ref_if_params.contains(&param.name)
                || self.current_fn_mixed_forwarder_params.contains(&param.name)
                || self.param_used_in_any_if_condition(&body, &param.name);
            if !forward_ref {
                continue;
            }
            if !self.expr_mentions_param_as_call_arg_in_expr(&param.name, condition) {
                continue;
            }
            // Only rewrite when a callee in this condition expects a shared `&T` for
            // this binding. Blind `policy,` → `&policy,` breaks owned recursive calls
            // (ReBAC `resolve_check(policy: Policy)` inside `if`).
            if !self.expr_call_expects_shared_borrow_for_param(condition, &param.name) {
                continue;
            }
            let bare = format!("({})", param.name);
            let borrowed = format!("(&{})", param.name);
            if cond_str.contains(&bare) && !cond_str.contains(&borrowed) {
                cond_str = cond_str.replace(&bare, &borrowed);
            }
            let bare_comma = format!("({},", param.name);
            let borrowed_comma = format!("(&{},", param.name);
            if cond_str.contains(&bare_comma) && !cond_str.contains(&borrowed_comma) {
                cond_str = cond_str.replace(&bare_comma, &borrowed_comma);
            }
        }
        cond_str
    }

    /// When an if-condition passes an owned outer param into an owned callee formal,
    /// clone at the call site so the then-branch can reuse the binding (list_unique / ReBAC).
    pub(crate) fn coerce_owned_params_clone_in_if_condition(
        &self,
        condition: &Expression<'ast>,
        mut cond_str: String,
    ) -> String {
        for param in &self.current_function_params {
            if param.name == "self" || self.is_type_copy(&param.type_) {
                continue;
            }
            if !self.caller_owned_non_copy_formal(&param.name) {
                continue;
            }
            if !self.expr_mentions_param_as_call_arg_in_expr(&param.name, condition) {
                continue;
            }
            if !self.expr_call_expects_owned_formal_for_param(condition, &param.name) {
                continue;
            }
            let clone_comma = format!("{}.clone(),", param.name);
            if cond_str.contains(&clone_comma) {
                continue;
            }
            let bare_comma = format!("({},", param.name);
            let cloned_comma = format!("({}.clone(),", param.name);
            if cond_str.contains(&bare_comma) {
                cond_str = cond_str.replace(&bare_comma, &cloned_comma);
            }
            let bare_fn = format!("({}", param.name);
            let cloned_fn = format!("({}.clone()", param.name);
            if cond_str.contains(&bare_fn) && !cond_str.contains(&cloned_fn) {
                cond_str = cond_str.replace(&bare_fn, &cloned_fn);
            }
        }
        cond_str
    }

    /// True when some call/method in `expr` takes `param_name` into an owned formal.
    fn expr_call_expects_owned_formal_for_param(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
    ) -> bool {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let func_name = match &**function {
                    Expression::Identifier { name, .. } => Some(name.as_str()),
                    Expression::FieldAccess { field, .. } => Some(field.as_str()),
                    _ => None,
                };
                if let Some(fname) = func_name {
                    let simple = fname.rsplit("::").next().unwrap_or(fname);
                    let sig =
                        crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature(
                            [
                                self.signature_registry.get_signature(fname).cloned(),
                                self.signature_registry.get_signature(simple).cloned(),
                                self.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(fname).cloned()),
                                self.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple).cloned()),
                            ],
                        );
                    if let Some(sig) = sig.as_ref() {
                        for (i, (_, arg)) in arguments.iter().enumerate() {
                            if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            {
                                let pidx = sig.arg_param_index(i);
                                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    sig, pidx,
                                ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                                    sig, pidx,
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                arguments.iter().any(|(_, arg)| {
                    self.expr_call_expects_owned_formal_for_param(arg, param_name)
                }) || self.expr_call_expects_owned_formal_for_param(function, param_name)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let recv_ty = self.infer_expression_type(object).and_then(|t| match t {
                    Type::Custom(name) => Some(name),
                    Type::Reference(inner) | Type::MutableReference(inner) => match *inner {
                        Type::Custom(name) => Some(name),
                        _ => None,
                    },
                    _ => None,
                });
                if let Some(rt) = recv_ty.as_deref() {
                    if let Some(sig) =
                        self.resolve_method_function_signature(rt, method, arguments.len())
                    {
                        for (i, (_, arg)) in arguments.iter().enumerate() {
                            if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            {
                                let pidx = sig.arg_param_index(i);
                                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &sig, pidx,
                                ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                                    &sig, pidx,
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                arguments.iter().any(|(_, arg)| {
                    self.expr_call_expects_owned_formal_for_param(arg, param_name)
                }) || self.expr_call_expects_owned_formal_for_param(object, param_name)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_call_expects_owned_formal_for_param(left, param_name)
                    || self.expr_call_expects_owned_formal_for_param(right, param_name)
            }
            Expression::Unary { operand, .. } => {
                self.expr_call_expects_owned_formal_for_param(operand, param_name)
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_call_expects_owned_formal_for_param(object, param_name)
            }
            _ => false,
        }
    }

    /// True when some call/method in `expr` takes `param_name` into a shared-ref formal.
    fn expr_call_expects_shared_borrow_for_param(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
    ) -> bool {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let func_name = match &**function {
                    Expression::Identifier { name, .. } => Some(name.as_str()),
                    Expression::FieldAccess { field, .. } => Some(field.as_str()),
                    _ => None,
                };
                if let Some(fname) = func_name {
                    let simple = fname.rsplit("::").next().unwrap_or(fname);
                    let sig =
                        crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature(
                            [
                                self.signature_registry.get_signature(fname).cloned(),
                                self.signature_registry.get_signature(simple).cloned(),
                                self.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(fname).cloned()),
                                self.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple).cloned()),
                            ],
                        );
                    if let Some(sig) = sig.as_ref() {
                        for (i, (_, arg)) in arguments.iter().enumerate() {
                            if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            {
                                let pidx = sig.arg_param_index(i);
                                if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    sig, pidx,
                                ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                    sig, pidx,
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                arguments
                    .iter()
                    .any(|(_, arg)| self.expr_call_expects_shared_borrow_for_param(arg, param_name))
                    || self.expr_call_expects_shared_borrow_for_param(function, param_name)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let recv_ty = self.infer_expression_type(object).and_then(|t| match t {
                    Type::Custom(name) => Some(name),
                    Type::Reference(inner) | Type::MutableReference(inner) => match *inner {
                        Type::Custom(name) => Some(name),
                        _ => None,
                    },
                    _ => None,
                });
                if let Some(rt) = recv_ty.as_deref() {
                    if let Some(sig) =
                        self.resolve_method_function_signature(rt, method, arguments.len())
                    {
                        for (i, (_, arg)) in arguments.iter().enumerate() {
                            if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            {
                                let pidx = sig.arg_param_index(i);
                                if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    &sig, pidx,
                                ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                    &sig, pidx,
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                arguments
                    .iter()
                    .any(|(_, arg)| self.expr_call_expects_shared_borrow_for_param(arg, param_name))
                    || self.expr_call_expects_shared_borrow_for_param(object, param_name)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_call_expects_shared_borrow_for_param(left, param_name)
                    || self.expr_call_expects_shared_borrow_for_param(right, param_name)
            }
            Expression::Unary { operand, .. } => {
                self.expr_call_expects_shared_borrow_for_param(operand, param_name)
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_call_expects_shared_borrow_for_param(object, param_name)
            }
            _ => false,
        }
    }

    /// True when the named caller formal is a Copy pass-by-value type (`f32`, `i32`, …).
    fn caller_param_is_copy_pass_by_value(&self, name: &str) -> bool {
        self.current_function_params.iter().any(|p| {
            p.name == name && crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
        })
    }

    fn expr_mentions_param_as_call_arg_in_expr(
        &self,
        param_name: &str,
        expr: &Expression<'ast>,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == param_name,
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                arguments.iter().any(|(_, arg)| {
                    matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        || self.expr_mentions_param_as_call_arg_in_expr(param_name, arg)
                })
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_param_as_call_arg_in_expr(param_name, left)
                    || self.expr_mentions_param_as_call_arg_in_expr(param_name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expr_mentions_param_as_call_arg_in_expr(param_name, operand)
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_mentions_param_as_call_arg_in_expr(param_name, object)
            }
            _ => false,
        }
    }

    pub(crate) fn param_used_in_any_if_condition(
        &self,
        body: &[&'ast crate::parser::Statement<'ast>],
        param_name: &str,
    ) -> bool {
        body.iter().any(|stmt| match stmt {
            crate::parser::Statement::If { condition, .. } => {
                self.expr_mentions_param_name_in_if_scan(param_name, condition)
            }
            crate::parser::Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.param_used_in_any_if_condition(then_block.as_slice(), param_name)
                    || else_block.as_ref().is_some_and(|block| {
                        self.param_used_in_any_if_condition(block.as_slice(), param_name)
                    })
            }
            crate::parser::Statement::While { body, .. }
            | crate::parser::Statement::Loop { body, .. }
            | crate::parser::Statement::Thread { body, .. }
            | crate::parser::Statement::Async { body, .. } => {
                self.param_used_in_any_if_condition(body.as_slice(), param_name)
            }
            crate::parser::Statement::For { body, .. } => {
                self.param_used_in_any_if_condition(body.as_slice(), param_name)
            }
            _ => false,
        })
    }

    fn expr_mentions_param_name_in_if_scan(
        &self,
        param_name: &str,
        expr: &Expression<'ast>,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == param_name,
            Expression::FieldAccess { object, .. } => {
                self.expr_mentions_param_name_in_if_scan(param_name, object)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_mentions_param_name_in_if_scan(param_name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_mentions_param_name_in_if_scan(param_name, arg))
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_mentions_param_name_in_if_scan(param_name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_mentions_param_name_in_if_scan(param_name, arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_param_name_in_if_scan(param_name, left)
                    || self.expr_mentions_param_name_in_if_scan(param_name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expr_mentions_param_name_in_if_scan(param_name, operand)
            }
            _ => false,
        }
    }

    /// Wrap bare function identifiers when callee params are borrowed but the call site
    /// passes owned values (e.g. `serve(handle)` → `serve(|req| handle(&req))`).
    pub(crate) fn maybe_wrap_fn_pointer_callback_bridge(
        &self,
        arg: &Expression<'ast>,
        arg_str: &str,
    ) -> String {
        let Expression::Identifier { name, .. } = arg else {
            return arg_str.to_string();
        };
        let is_local_variable = self.local_var_types.contains_key(name.as_str())
            || self.match_arm_bindings.contains(name.as_str())
            || self.inferred_borrowed_params.contains(name.as_str())
            || self.inferred_mut_borrowed_params.contains(name.as_str())
            || self.current_function_params.iter().any(|p| p.name == *name);
        if is_local_variable {
            return arg_str.to_string();
        }
        let Some(func_sig) = self.signature_registry.get_signature(name) else {
            return arg_str.to_string();
        };
        if func_sig.has_self_receiver || func_sig.is_extern {
            return arg_str.to_string();
        }
        let has_borrowed: Vec<usize> = func_sig
            .param_ownership
            .iter()
            .enumerate()
            .filter(|(_, o)| matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed))
            .map(|(idx, _)| idx)
            .collect();
        if has_borrowed.is_empty() {
            return arg_str.to_string();
        }
        let n = func_sig.param_ownership.len();
        let wrapper: Vec<String> = (0..n).map(|j| format!("__cb{j}")).collect();
        let call: Vec<String> = (0..n)
            .map(|j| match func_sig.param_ownership[j] {
                OwnershipMode::MutBorrowed => format!("&mut __cb{j}"),
                OwnershipMode::Borrowed => format!("&__cb{j}"),
                _ => format!("__cb{j}"),
            })
            .collect();
        format!("|{}| {}({})", wrapper.join(", "), name, call.join(", "))
    }

    /// Copy payload from a `match` / `if let` pattern (enum tuple field, etc.).
    pub(crate) fn copy_match_payload_binding(&self, name: &str) -> bool {
        self.match_arm_bindings.contains(name) && self.binding_name_is_copy(name)
    }

    /// Owned Copy match payloads pass by value at call sites — emit bare `qty`, not `*qty`.
    pub(crate) fn normalize_owned_copy_match_binding_call_arg(
        &self,
        arg_expr: &Expression<'ast>,
        coerced: &str,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> String {
        let Expression::Identifier { name, .. } = arg_expr else {
            return coerced.to_string();
        };
        if !self.copy_match_payload_binding(name) {
            return coerced.to_string();
        }
        let pidx = sig.arg_param_index(arg_index);
        if crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx) {
            return coerced.to_string();
        }
        // Enum/`match` Copy payloads are owned bindings — pass by value (`qty`), not `*qty`.
        name.to_string()
    }

    /// HashSet/HashMap loop elements are `&T` — deref when callee expects owned Copy scalar.
    ///
    /// Never emit `*binding.clone()`: `&Copy::clone` autoderefs to owned Copy, so a leading
    /// `*` would deref the owned value (E0614). Strip redundant `.clone()` then emit `*binding`.
    pub(crate) fn normalize_borrowed_iter_elem_for_owned_copy_scalar(
        &self,
        arg_expr: &Expression<'ast>,
        coerced: &str,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> String {
        let Expression::Identifier { name, .. } = arg_expr else {
            return coerced.to_string();
        };
        if !self.borrowed_iterator_vars.contains(name) {
            return coerced.to_string();
        }
        let pidx = sig.arg_param_index(arg_index);
        if crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx) {
            return coerced.to_string();
        }
        let wants_owned_copy_scalar = sig
            .formal_param_type(pidx)
            .or_else(|| sig.param_types.get(pidx))
            .is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                crate::type_classification::is_copy_pass_by_value_formal(bare)
            });
        if !wants_owned_copy_scalar {
            return coerced.to_string();
        }
        let mut base = coerced.to_string();
        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut base);
        if let Some(rest) = base.strip_prefix("&mut ") {
            base = rest.to_string();
        } else if let Some(rest) = base.strip_prefix('&') {
            base = rest.to_string();
        }
        if base.starts_with('*') {
            return base;
        }
        format!("*{base}")
    }

    /// Whether a binding (param, local, or implicit struct field) is Copy.
    pub(crate) fn binding_name_is_copy(&self, name: &str) -> bool {
        if self
            .current_function_params
            .iter()
            .find(|p| p.name == name)
            .is_some_and(|p| self.is_type_copy(&p.type_))
        {
            return true;
        }
        if self
            .local_var_types
            .get(name)
            .is_some_and(|t| self.is_type_copy(t))
        {
            return true;
        }
        if self.local_var_types.get(name).is_some_and(|t| {
            matches!(
                t,
                Type::Reference(inner) | Type::MutableReference(inner)
                    if self.is_type_copy(inner.as_ref())
            )
        }) {
            return true;
        }
        if self.in_impl_block && self.current_struct_fields.contains(name) {
            if let Some(struct_name) = &self.current_struct_name {
                if let Some(fields) = self.lookup_struct_field_types(struct_name) {
                    if let Some(field_ty) = fields.get(name) {
                        return self.is_type_copy(field_ty);
                    }
                }
            }
        }
        false
    }

    /// Whether an expression's inferred type is Copy (including through references).
    pub(crate) fn expression_is_copy(&self, expr: &Expression<'ast>) -> bool {
        self.infer_expression_type(expr).is_some_and(|t| {
            let pointee = match &t {
                Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                other => other,
            };
            self.is_type_copy(pointee)
        })
    }

    pub(crate) fn maybe_auto_clone(&self, name: &str, arg_str: &str) -> String {
        if self.match_arm_bindings.contains(name) {
            return arg_str.to_string();
        }
        if self.in_user_written_closure && self.user_closure_params.contains(name) {
            return arg_str.to_string();
        }
        if self.current_func_is_pure_forwarding_delegate {
            return arg_str.to_string();
        }
        if self.current_fn_mixed_forwarder_params.contains(name) && self.in_if_condition {
            return arg_str.to_string();
        }
        if self.param_used_in_prior_field_extract_call(name) {
            return arg_str.to_string();
        }

        if self.current_struct_name.as_ref().is_some_and(|sn| {
            self.current_function_params
                .iter()
                .find(|p| p.name == name)
                .is_some_and(|p| self.struct_is_owned_engine_key_facade(sn, p))
        }) {
            return arg_str.to_string();
        }

        let dominated = self
            .auto_clone_analysis
            .as_ref()
            .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());

        if !dominated || arg_str.ends_with(".clone()") || arg_str.ends_with(".to_string()") {
            return arg_str.to_string();
        }

        // Borrowed *emitted* formals (`&T`) don't move — skip clone. Owned formals that
        // the analyzer still marks Borrowed (Copy aggregates / match-project callees)
        // still move at owned call sites and need `.clone()` on reuse (regression-063 Value).
        if (self.inferred_borrowed_params.contains(name)
            || self.inferred_mut_borrowed_params.contains(name))
            && (self.emitted_rust_ref_formals.contains(name)
                || self.binding_emits_as_rust_shared_ref(name))
        {
            return arg_str.to_string();
        }

        if self.borrowed_iterator_vars.contains(name)
            && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                self.current_function_return_type.as_ref(),
            )
        {
            return arg_str.to_string();
        }

        // Borrowed iterator elements that are Copy scalars (`&i64` from HashSet)
        // must not get `.clone()` — IR deref (`*post`) owns the value; `*post.clone()`
        // is E0614 because `&T::clone` autoderefs to owned T.
        if self.borrowed_iterator_vars.contains(name)
            && self.binding_is_copy_pass_by_value_scalar(name)
        {
            return arg_str.to_string();
        }

        if self
            .local_var_types
            .get(name)
            .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
            && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                self.current_function_return_type.as_ref(),
            )
        {
            return arg_str.to_string();
        }

        // Only scalar Copy formals (i64/bool/…) skip clone. Copy aggregates/enums
        // (Value, Lsn, …) still need `.clone()` for multi-use owned moves (regression-063).
        if self
            .current_function_params
            .iter()
            .find(|p| p.name == name)
            .is_some_and(|p| self.is_type_copy(&p.type_))
            || self
                .local_var_types
                .get(name)
                .is_some_and(|t| self.is_type_copy(t))
        {
            return arg_str.to_string();
        }
        if self.binding_is_copy_pass_by_value_scalar(name) {
            return arg_str.to_string();
        }

        if arg_str.contains(" as ") && !arg_str.starts_with('(') {
            format!("({}).clone()", arg_str)
        } else {
            format!("{}.clone()", arg_str)
        }
    }

    /// True when `name` is a scalar Copy pass-by-value binding (`i64`, `bool`, …),
    /// not a Copy aggregate/enum that still moves at the Rust ABI.
    pub(crate) fn binding_is_copy_pass_by_value_scalar(&self, name: &str) -> bool {
        if let Some(p) = self.current_function_params.iter().find(|p| p.name == name) {
            return crate::type_classification::is_copy_pass_by_value_formal(&p.type_);
        }
        if let Some(t) = self.local_var_types.get(name) {
            let bare = match t {
                Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                other => other,
            };
            return crate::type_classification::is_copy_pass_by_value_formal(bare);
        }
        false
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

    /// When returning owned `Option<T>` but the expression yields `Option<&T>`
    /// (e.g. `HashMap::get`), append `.copied()` / `.cloned()` from inferred types —
    /// not method-name lists.
    pub(crate) fn coerce_option_ref_return_to_owned(
        &self,
        expr_str: &mut String,
        expr: &crate::parser::Expression,
    ) {
        if !self.returns_option_owned_type() {
            return;
        }
        if !self.expression_type_contains_reference(expr) {
            return;
        }
        if expr_str.ends_with(".cloned()")
            || expr_str.ends_with(".copied()")
            || expr_str.ends_with(".clone()")
            || expr_str.contains(".map(|v| v.clone())")
        {
            return;
        }
        let Some(ty) = self.infer_expression_type(expr) else {
            *expr_str = format!("{}.cloned()", expr_str);
            return;
        };
        if Self::type_contains_mut_reference_static(&ty) {
            *expr_str = format!("{}.map(|v| v.clone())", expr_str);
            return;
        }
        let copy_payload = matches!(
            &ty,
            Type::Option(inner)
                if matches!(
                    inner.as_ref(),
                    Type::Reference(r) | Type::MutableReference(r) if self.is_type_copy(r.as_ref())
                )
        );
        if copy_payload {
            *expr_str = format!("{}.copied()", expr_str);
        } else {
            *expr_str = format!("{}.cloned()", expr_str);
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
                if let crate::parser::Expression::FieldAccess { field, .. } = expr {
                    if self.module_string_consts.contains(field) {
                        *expr_str =
                            super::string_utilities::coerce_expr_to_owned_string(expr_str);
                        return;
                    }
                }
                if let crate::parser::Expression::Identifier { name, .. } = expr {
                    if self.module_string_consts.contains(name) {
                        *expr_str =
                            super::string_utilities::coerce_expr_to_owned_string(expr_str);
                        return;
                    }
                    let is_ref_text = self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                        matches!(
                            t,
                            Type::Reference(inner) | Type::MutableReference(inner)
                                if super::types::is_windjammer_text_type(inner)
                        )
                    });
                    let is_borrowed_string_param = self.inferred_borrowed_params.contains(name)
                        || self.str_ref_optimized_params.contains(name)
                        || self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && matches!(
                                    &p.type_,
                                    Type::Reference(inner)
                                        if super::types::is_windjammer_text_type(inner)
                                )
                        });
                    if is_ref_text || is_borrowed_string_param {
                        *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
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
                if obj_name == "self"
                    && !expr_str.ends_with(".clone()")
                    && !self.suppress_borrowed_clone
                {
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
    /// Whether a type represents a Copy value when moved out from behind a reference.
    pub(crate) fn is_copy_move_out_type(&self, ty: &crate::parser::Type) -> bool {
        match ty {
            crate::parser::Type::Reference(inner)
            | crate::parser::Type::MutableReference(inner) => self.is_type_copy(inner),
            other => self.is_type_copy(other),
        }
    }

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
                let is_inferred_borrowed = self.inferred_borrowed_params.contains(name)
                    || self.emitted_rust_ref_formals.contains(name);
                let root_behind_ref = self.field_access_root_is_behind_reference(arg);

                if (is_self_field
                    || is_borrowed_iter
                    || is_explicit_ref
                    || is_inferred_borrowed
                    || root_behind_ref)
                    && !arg_str.ends_with(".clone()")
                {
                    // Owned callee formals cannot reborrow `&self.field` / `&param.field`
                    // (E0507). Always clone non-Copy fields out of a borrowed root.
                    // Mut-passthrough (`&mut self.field` → `&mut T`) is handled by the
                    // mut-borrow call-site path and never reaches this helper.
                    let is_copy = self
                        .infer_expression_type(arg)
                        .as_ref()
                        .is_some_and(|t| self.is_copy_move_out_type(t));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::safety_type::{OwnedType, Region};

    #[test]
    fn test_owned_type_to_ownership_mode_mapping() {
        assert_eq!(
            owned_type_to_ownership_mode(&OwnedType::Owned),
            OwnershipMode::Owned
        );
        assert_eq!(
            owned_type_to_ownership_mode(&OwnedType::Ref(Region::fresh(0))),
            OwnershipMode::Borrowed
        );
        assert_eq!(
            owned_type_to_ownership_mode(&OwnedType::MutRef(Region::fresh(1))),
            OwnershipMode::MutBorrowed
        );
        assert_eq!(
            owned_type_to_ownership_mode(&OwnedType::Copy),
            OwnershipMode::Owned
        );
        assert_eq!(
            owned_type_to_ownership_mode(&OwnedType::Inferred),
            OwnershipMode::Owned
        );
    }

    #[test]
    fn test_ir_cutover_config_derive_default_all_off() {
        let config = IrCutoverConfig::default();
        assert!(!config.ownership);
        assert!(!config.clones);
        assert!(!config.param_types);
        assert!(!config.str_ref);
        assert!(!config.call_sites);
        assert!(!config.locals);
        assert!(!config.all_enabled());
    }

    #[test]
    fn static_call_root_is_import_driven_not_crate_name_list() {
        let mut gen = CodeGenerator::new(SignatureRegistry::empty(), CompilationTarget::Rust);
        assert!(gen.identifier_is_static_call_root("Vec"));
        assert!(gen.identifier_is_static_call_root("std"));
        assert!(
            !gen.identifier_is_static_call_root("tokio"),
            "unimported crate names must not be a hardcoded module list"
        );
        gen.imported_path_roots.insert("tokio".into());
        assert!(gen.identifier_is_static_call_root("tokio"));
        assert!(
            !gen.identifier_is_static_call_root("log"),
            "short names used as locals must stay instance receivers until imported"
        );
        gen.runtime_std_module_imports.insert("log".into());
        assert!(gen.identifier_is_static_call_root("log"));
    }

    #[test]
    fn static_call_root_imported_module_shadowed_by_local_binding() {
        let mut gen = CodeGenerator::new(SignatureRegistry::empty(), CompilationTarget::Rust);
        gen.imported_path_roots.insert("tree".into());
        assert!(gen.identifier_is_static_call_root("tree"));
        gen.match_arm_bindings.insert("tree".into());
        assert!(
            !gen.identifier_is_static_call_root("tree"),
            "if let Some(tree) must use . not :: even when tree is an imported module path"
        );
    }

    #[test]
    fn test_ir_cutover_from_env_defaults_formal_flags_on() {
        let config = IrCutoverConfig::from_env();
        assert!(config.ownership);
        assert!(config.clones);
        assert!(config.param_types);
        assert!(config.str_ref);
        assert!(config.call_sites);
        assert!(config.locals);
        assert!(config.all_enabled());
    }

    #[test]
    fn test_ir_cutover_config_all_enabled() {
        let config = IrCutoverConfig {
            ownership: true,
            clones: true,
            param_types: true,
            str_ref: true,
            call_sites: true,
            locals: true,
        };
        assert!(config.all_enabled());
    }

    #[test]
    fn test_ir_cutover_config_partial_not_all() {
        let config = IrCutoverConfig {
            ownership: true,
            clones: false,
            param_types: true,
            str_ref: true,
            call_sites: false,
            locals: false,
        };
        assert!(!config.all_enabled());
    }
}
