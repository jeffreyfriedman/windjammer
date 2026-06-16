#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Ownership and borrow checking analyzer
use crate::auto_clone::AutoCloneAnalysis;
use crate::parser::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod borrow_analysis;
mod cache_locality;
mod forbidden_patterns;
mod function_analysis;
mod generic_analysis;
mod module_analysis;
mod mutation_detection;
mod optimization_detectors;
mod parameter_analysis;
mod passthrough_inference;
mod program_initialization;
mod self_access_and_option_refs;
mod self_analysis;
mod self_binding_mutation;
mod self_dispatch_for_loops;
mod self_field_mutation;
mod self_mutating_calls;
mod self_return_and_consumption;
mod signature_registry;
pub mod simd_loops;
pub mod stdlib_method_traits;
mod string_optimization;
mod trait_analysis;
mod type_checking;
pub mod type_collector;
mod usage_tracking;

pub use signature_registry::{FunctionSignature, SignatureRegistry};

pub use cache_locality::{
    cache_locality_json_report, AccessPatternKind, AoSoACandidate, CacheLocalityAnalysis,
};

// Type alias for complex return type
type ProgramAnalysisResult<'ast> = (
    Vec<AnalyzedFunction<'ast>>,
    SignatureRegistry,
    HashMap<String, HashMap<String, AnalyzedFunction<'ast>>>,
);

#[derive(Debug, Clone)]
pub struct AnalyzedFunction<'ast> {
    pub decl: FunctionDecl<'ast>,
    pub inferred_ownership: HashMap<String, OwnershipMode>,
    // STRING INFERENCE: Track inferred types for string parameters (&str vs String)
    pub inferred_param_types: Vec<Type>,
    // AUTO-MUT: Track which local variables are mutated (for automatic mut inference)
    pub mutated_variables: HashSet<String>,
    // LINTER: Track which parameters are mutated (for owned-but-not-returned lint)
    pub mutated_parameters: HashSet<String>,
    // AUTO-CLONE: Track where clones should be automatically inserted
    pub auto_clone_analysis: AutoCloneAnalysis,
    // PHASE 2 OPTIMIZATION: Track unnecessary clones that can be eliminated
    pub clone_optimizations: Vec<CloneOptimization>,
    // PHASE 3 OPTIMIZATION: Track struct mapping opportunities
    pub struct_mapping_optimizations: Vec<StructMappingOptimization>,
    // PHASE 4 OPTIMIZATION: Track string operations for optimization
    pub string_optimizations: Vec<StringOptimization>,
    // PHASE 5 OPTIMIZATION: Track assignment operations that can use compound operators
    pub assignment_optimizations: Vec<AssignmentOptimization>,
    // PHASE 6 OPTIMIZATION: Track heavy drops that can be deferred to background thread
    pub defer_drop_optimizations: Vec<DeferDropOptimization>,
    // PHASE 7 OPTIMIZATION: Track static values that can be const
    pub const_static_optimizations: Vec<ConstStaticOptimization>,
    // PHASE 8 OPTIMIZATION: Track Vec usage that can use SmallVec
    pub smallvec_optimizations: Vec<SmallVecOptimization>,
    // PHASE 9 OPTIMIZATION: Track string/data that can use Cow
    pub cow_optimizations: Vec<CowOptimization>,
    /// Cache locality: AoSoA / SoA loop candidates (ECS-style Vec<Struct> iteration).
    pub cache_locality: CacheLocalityAnalysis,
    // STRING PARAMETER OPTIMIZATION (Phase 2): Track which string params can use &str
    pub str_ref_optimizable_params: HashSet<String>,
}

/// PHASE 5: Assignment operation that can be optimized to compound operator
#[derive(Debug, Clone)]
pub struct AssignmentOptimization {
    pub variable: String,
    pub location: usize,
    pub operation: CompoundOp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompoundOp {
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
}

/// PHASE 6: Defer drop optimization - automatically defer heavy deallocations to background thread
/// This can make functions return 10,000x faster by dropping large data structures asynchronously.
/// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
#[derive(Debug, Clone)]
pub struct DeferDropOptimization {
    /// The variable name that should be deferred
    pub variable: String,
    /// Estimated size of the type
    pub estimated_size: EstimatedSize,
    /// Reason for deferring
    pub reason: DeferDropReason,
    /// Location where the defer should happen (usually before return)
    pub location: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EstimatedSize {
    Small,     // < 1KB - not worth deferring
    Medium,    // 1KB - 100KB - maybe defer
    Large,     // 100KB - 10MB - definitely defer
    VeryLarge, // > 10MB - always defer
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeferDropReason {
    /// Function owns large parameter, returns small value
    LargeOwnedParameter,
    /// Large local variable goes out of scope
    LargeLocalVariable,
    /// Function builds large collection, extracts small value
    LargeReturnedCollection,
}

/// PHASE 7: Const static optimization - convert runtime static to compile-time const
#[derive(Debug, Clone)]
pub struct ConstStaticOptimization {
    pub variable: String,
    pub can_be_const: bool,
}

/// PHASE 8: SmallVec optimization - use stack allocation for small vectors
#[derive(Debug, Clone)]
pub struct SmallVecOptimization {
    pub variable: String,
    pub estimated_max_size: usize, // Maximum size observed
    pub stack_size: usize,         // Recommended stack size (power of 2)
}

/// PHASE 9: Cow optimization - use clone-on-write for conditionally modified data
#[derive(Debug, Clone)]
pub struct CowOptimization {
    pub variable: String,
    pub reason: CowReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CowReason {
    ConditionalModification, // if/match that may or may not modify
    ReadHeavy,               // Mostly read, rarely written
}

/// Represents a string operation that can be optimized
#[derive(Debug, Clone)]
pub struct StringOptimization {
    /// Type of string optimization
    pub optimization_type: StringOptimizationType,
    /// Estimated capacity needed
    pub estimated_capacity: Option<usize>,
    /// Location in the function
    pub location: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringOptimizationType {
    /// String interpolation that can pre-allocate capacity
    InterpolationWithCapacity,
    /// Multiple string concatenations
    ConcatenationChain,
    /// String building in a loop
    LoopAccumulation,
    /// Repeated format! calls
    RepeatedFormatting,
}

/// Represents a struct-to-struct mapping that can be optimized
#[derive(Debug, Clone)]
pub struct StructMappingOptimization {
    /// Target struct being created
    pub target_struct: String,
    /// Source of data (variable name or "row")
    pub source: String,
    /// Field mappings: (target_field, source_expression)
    pub field_mappings: Vec<(String, String)>,
    /// Optimization strategy to use
    pub strategy: MappingStrategy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MappingStrategy {
    /// Direct field-to-field mapping (zero-cost)
    DirectMapping,
    /// Database row extraction (use FromRow trait)
    FromRow,
    /// Builder pattern optimization
    Builder,
    /// Simple field copy with type conversion
    TypeConversion,
}

/// Represents a `.clone()` call that can be optimized away
#[derive(Debug, Clone)]
pub struct CloneOptimization {
    /// Variable name being cloned
    pub variable: String,
    /// Statement index where clone occurs
    pub location: usize,
    /// Why we can eliminate this clone
    pub reason: CloneEliminationReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CloneEliminationReason {
    /// Value is only read, never mutated
    OnlyRead,
    /// Value is used once and then discarded
    SingleUse,
    /// Value doesn't escape the function
    LocalOnly,
    /// Better to use move semantics
    CanMove,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OwnershipMode {
    Owned,
    Borrowed,
    MutBorrowed,
}

/// During `impl Type` method analysis: resolve `self.field.method()` via `FieldType::method` in the signature registry.
/// `program` is a raw pointer so we do not require `&'ast Program<'ast>` on every analyzer entry point; it is only
/// dereferenced while the surrounding compile pass holds the program alive.
pub(crate) struct ImplSelfFieldContext<'ast> {
    pub impl_type_base: String,
    program: *const Program<'ast>,
}

impl<'ast> ImplSelfFieldContext<'ast> {
    pub(crate) fn new(impl_type_base: String, program: &Program<'ast>) -> Self {
        Self {
            impl_type_base,
            program: std::ptr::from_ref(program),
        }
    }

    pub(crate) fn program(&self) -> &'ast Program<'ast> {
        // SAFETY: `program` points to the AST being analyzed; context is cleared before the borrow ends.
        unsafe { &*self.program }
    }
}

pub struct Analyzer<'ast> {
    // Track variable ownership modes (reserved for future use)
    #[allow(dead_code)]
    variables: HashMap<String, OwnershipMode>,
    // Track enum definitions to determine if they're Copy
    copy_enums: HashSet<String>,
    // Track struct definitions with @derive(Copy) to determine if they're Copy
    /// Arc-wrapped to avoid O(n) cloning when shared across 649+ library files.
    copy_structs: Arc<HashSet<String>>,
    // Track trait definitions for impl block analysis
    trait_definitions: HashMap<String, TraitDecl<'ast>>,
    // Track analyzed trait methods (trait_name -> method_name -> AnalyzedFunction)
    // PUBLIC: The generator needs this for trait signature inference
    pub analyzed_trait_methods: HashMap<String, HashMap<String, AnalyzedFunction<'ast>>>,
    // Track which local variables are mutated (for automatic mut inference)
    mutated_variables: HashSet<String>,
    // Track functions in the current impl block (for cross-method analysis)
    current_impl_functions: Option<HashMap<String, crate::parser::ast::FunctionDecl<'ast>>>,
    /// Set while analyzing methods inside an `impl` block (inherent or trait impl body).
    self_impl_context: Option<ImplSelfFieldContext<'ast>>,
    /// Cross-file struct field type registry for nested field chain resolution.
    /// Maps struct_name → { field_name → Type }.
    /// Arc-wrapped to avoid O(n) cloning when shared across 649+ files.
    global_struct_field_types: Arc<HashMap<String, HashMap<String, Type>>>,
    /// Unqualified struct name → module paths where it is defined (for qualified field lookup).
    /// Arc-wrapped to avoid O(n) cloning when shared across 649+ files.
    struct_defining_module_paths: Arc<HashMap<String, Vec<Vec<String>>>>,
    /// When true, skip expensive optimization detectors (clone, struct mapping,
    /// string, cache locality, etc.) during multipass convergence.
    /// Only ownership inference runs. Final pass sets this to false.
    pub convergence_only: bool,
    /// When true, global ownership already converged (library Step 3) — skip
    /// per-file multi-pass ownership loop in analyze_program_with_global_arc.
    pub ownership_preconverged: bool,
}

impl<'ast> Analyzer<'ast> {
    pub(super) fn new_empty(global_copy_structs: HashSet<String>) -> Self {
        Self::new_empty_shared(Arc::new(global_copy_structs))
    }

    pub(super) fn new_empty_shared(copy_structs: Arc<HashSet<String>>) -> Self {
        Self {
            variables: HashMap::new(),
            copy_enums: HashSet::new(),
            copy_structs,
            trait_definitions: HashMap::new(),
            analyzed_trait_methods: HashMap::new(),
            mutated_variables: HashSet::new(),
            current_impl_functions: None,
            self_impl_context: None,
            global_struct_field_types: Arc::new(HashMap::new()),
            struct_defining_module_paths: Arc::new(HashMap::new()),
            convergence_only: false,
            ownership_preconverged: false,
        }
    }

    pub fn analyze_program(
        &mut self,
        program: &Program<'ast>,
    ) -> Result<ProgramAnalysisResult<'ast>, String> {
        // LANGUAGE DESIGN CHECK: Prohibit Rust-specific patterns before analysis
        self.check_forbidden_rust_patterns(program)?;

        self.analyze_program_with_global_signatures(program, &SignatureRegistry::new())
    }

    /// Check for forbidden Rust-specific patterns that should not appear in Windjammer source.
    /// These are implementation details that the compiler should handle automatically.
    pub fn check_forbidden_rust_patterns(&self, program: &Program<'ast>) -> Result<(), String> {
        forbidden_patterns::check_forbidden_rust_patterns(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_copy_type_primitives() {
        let analyzer = Analyzer::new();

        // Primitive types are Copy
        assert!(analyzer.is_copy_type(&Type::Int));
        assert!(analyzer.is_copy_type(&Type::Int32));
        assert!(analyzer.is_copy_type(&Type::Uint));
        assert!(analyzer.is_copy_type(&Type::Float));
        assert!(analyzer.is_copy_type(&Type::Bool));

        // Compound types are not Copy by default
        assert!(!analyzer.is_copy_type(&Type::String));
        assert!(!analyzer.is_copy_type(&Type::Vec(Box::new(Type::Int))));
    }

    #[test]
    fn test_is_copy_type_references() {
        let analyzer = Analyzer::new();

        // References to Copy types are Copy
        assert!(analyzer.is_copy_type(&Type::Reference(Box::new(Type::Int))));
        assert!(analyzer.is_copy_type(&Type::Reference(Box::new(Type::String))));

        // Mutable references are not Copy
        assert!(!analyzer.is_copy_type(&Type::MutableReference(Box::new(Type::Int))));
    }

    #[test]
    fn test_is_mutating_method() {
        let analyzer = Analyzer::new();

        // Currently recognized mutating methods
        assert!(analyzer.is_mutating_method("push"));
        assert!(analyzer.is_mutating_method("push_str"));
        assert!(analyzer.is_mutating_method("pop"));
        assert!(analyzer.is_mutating_method("insert"));
        assert!(analyzer.is_mutating_method("remove"));
        assert!(analyzer.is_mutating_method("clear"));
        assert!(analyzer.is_mutating_method("append"));
        assert!(analyzer.is_mutating_method("take"));
        assert!(analyzer.is_mutating_method("replace"));
        assert!(analyzer.is_mutating_method("get_or_insert"));

        // Non-mutating methods
        assert!(!analyzer.is_mutating_method("len"));
        assert!(!analyzer.is_mutating_method("is_empty"));
        assert!(!analyzer.is_mutating_method("get"));
        assert!(!analyzer.is_mutating_method("iter"));
        assert!(!analyzer.is_mutating_method("clone"));
    }

    #[test]
    fn test_analyzer_tracks_mutated_variables() {
        let mut analyzer = Analyzer::new();

        // Initially no mutations
        assert!(!analyzer.is_variable_mutated("x"));
        assert!(!analyzer.is_variable_mutated("y"));

        // Track mutation
        analyzer.mutated_variables.insert("x".to_string());
        assert!(analyzer.is_variable_mutated("x"));
        assert!(!analyzer.is_variable_mutated("y"));

        // Track another mutation
        analyzer.mutated_variables.insert("y".to_string());
        assert!(analyzer.is_variable_mutated("y"));
    }

    #[test]
    fn test_ownership_mode_display() {
        assert_eq!(format!("{:?}", OwnershipMode::Owned), "Owned");
        assert_eq!(format!("{:?}", OwnershipMode::Borrowed), "Borrowed");
        assert_eq!(format!("{:?}", OwnershipMode::MutBorrowed), "MutBorrowed");
    }

    #[test]
    fn test_is_generic_type_param() {
        // is_generic_type_param is a static method
        // It checks if a Type looks like a generic (e.g., single uppercase letter)

        // Single uppercase letters are generic
        assert!(Analyzer::is_generic_type_param(&Type::Custom(
            "T".to_string()
        )));
        assert!(Analyzer::is_generic_type_param(&Type::Custom(
            "U".to_string()
        )));
        assert!(Analyzer::is_generic_type_param(&Type::Custom(
            "A".to_string()
        )));

        // Multi-letter types are not generic
        assert!(!Analyzer::is_generic_type_param(&Type::Custom(
            "Point".to_string()
        )));
        assert!(!Analyzer::is_generic_type_param(&Type::Custom(
            "Vec".to_string()
        )));
        assert!(!Analyzer::is_generic_type_param(&Type::Custom(
            "Item".to_string()
        )));

        // Primitive types are not generic
        assert!(!Analyzer::is_generic_type_param(&Type::Int));
        assert!(!Analyzer::is_generic_type_param(&Type::String));
    }

    #[test]
    fn test_analyzer_new() {
        let analyzer = Analyzer::new();

        // New analyzer should have empty state
        assert!(analyzer.mutated_variables.is_empty());
    }

    #[test]
    fn test_is_copy_type_option() {
        let analyzer = Analyzer::new();

        // Option<T> is Copy if T is Copy
        assert!(analyzer.is_copy_type(&Type::Option(Box::new(Type::Int))));
        assert!(analyzer.is_copy_type(&Type::Option(Box::new(Type::Bool))));
        assert!(analyzer.is_copy_type(&Type::Option(Box::new(Type::Float))));

        // Option<T> is not Copy if T is not Copy
        assert!(!analyzer.is_copy_type(&Type::Option(Box::new(Type::String))));
        assert!(!analyzer.is_copy_type(&Type::Option(Box::new(Type::Vec(Box::new(Type::Int))))));
    }

    #[test]
    fn test_is_copy_type_result() {
        let analyzer = Analyzer::new();

        // Result<T, E> is Copy if both T and E are Copy
        assert!(analyzer.is_copy_type(&Type::Result(Box::new(Type::Int), Box::new(Type::Bool))));

        // Result<T, E> is not Copy if T is not Copy
        assert!(!analyzer.is_copy_type(&Type::Result(Box::new(Type::String), Box::new(Type::Int))));

        // Result<T, E> is not Copy if E is not Copy
        assert!(!analyzer.is_copy_type(&Type::Result(Box::new(Type::Int), Box::new(Type::String))));
    }

    #[test]
    fn test_is_copy_type_array() {
        let analyzer = Analyzer::new();

        // Fixed-size arrays [T; N] are Copy if T is Copy
        assert!(analyzer.is_copy_type(&Type::Array(Box::new(Type::Int), 10)));
        assert!(analyzer.is_copy_type(&Type::Array(Box::new(Type::Bool), 5)));

        // Arrays of non-Copy types are not Copy
        assert!(!analyzer.is_copy_type(&Type::Array(Box::new(Type::String), 3)));
    }

    #[test]
    fn test_is_copy_type_vec() {
        let analyzer = Analyzer::new();

        // Vec<T> is never Copy (heap-allocated)
        assert!(!analyzer.is_copy_type(&Type::Vec(Box::new(Type::Int))));
        assert!(!analyzer.is_copy_type(&Type::Vec(Box::new(Type::Bool))));
    }

    #[test]
    fn test_is_copy_type_tuple() {
        let analyzer = Analyzer::new();

        // Tuple of Copy types is Copy
        assert!(analyzer.is_copy_type(&Type::Tuple(vec![Type::Int, Type::Bool])));
        assert!(analyzer.is_copy_type(&Type::Tuple(vec![Type::Float, Type::Uint])));

        // Tuple with non-Copy type is not Copy
        assert!(!analyzer.is_copy_type(&Type::Tuple(vec![Type::Int, Type::String])));
        assert!(!analyzer.is_copy_type(&Type::Tuple(vec![Type::Vec(Box::new(Type::Int))])));
    }

    #[test]
    fn test_ownership_mode_equality() {
        assert_eq!(OwnershipMode::Owned, OwnershipMode::Owned);
        assert_eq!(OwnershipMode::Borrowed, OwnershipMode::Borrowed);
        assert_eq!(OwnershipMode::MutBorrowed, OwnershipMode::MutBorrowed);

        assert_ne!(OwnershipMode::Owned, OwnershipMode::Borrowed);
        assert_ne!(OwnershipMode::Borrowed, OwnershipMode::MutBorrowed);
    }
}
