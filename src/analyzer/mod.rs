#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Ownership and borrow checking analyzer
use crate::auto_clone::AutoCloneAnalysis;
use crate::parser::*;
use std::collections::HashMap;

mod mutation_detection;
mod optimization_detectors;
mod passthrough_inference;
mod self_analysis;
mod string_optimization;
pub mod type_collector;

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

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub param_types: Vec<Type>, // ADDED: Store actual parameter types for smart inference
    pub param_ownership: Vec<OwnershipMode>,
    pub return_type: Option<Type>, // ADDED: Store return type for smart inference
    pub return_ownership: OwnershipMode,
    pub has_self_receiver: bool, // True if first parameter is self/&self/&mut self
    pub is_extern: bool,         // True if this is an extern function (FFI)
}

#[derive(Debug, Clone)]
pub struct SignatureRegistry {
    pub signatures: HashMap<String, FunctionSignature>,
    collision_keys: HashSet<String>,
}

impl Default for SignatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureRegistry {
    pub fn new() -> Self {
        let mut registry = SignatureRegistry {
            signatures: HashMap::new(),
            collision_keys: HashSet::new(),
        };

        // Populate with stdlib signatures by scanning windjammer-runtime source
        if let Err(e) = crate::stdlib_scanner::populate_runtime_signatures(&mut registry) {
            eprintln!("Warning: Failed to scan runtime signatures: {}", e);
            eprintln!("Continuing with empty registry - may generate incorrect borrows");
        }

        registry
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSignature) {
        if let Some(existing) = self.signatures.get(&name) {
            if existing.param_types != sig.param_types
                || existing.param_ownership != sig.param_ownership
            {
                self.collision_keys.insert(name.clone());
            }
        }
        self.signatures.insert(name, sig);
    }

    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(name)
    }

    /// Check if a signature key has been registered with conflicting param types
    /// from different modules (namespace collision).
    pub fn has_collision(&self, name: &str) -> bool {
        self.collision_keys.contains(name)
    }

    /// Fallback lookup: find a signature whose key ends with `::name`.
    /// Used when a method call is recorded as bare "method" but registered as "Type::method".
    pub fn find_signature_ending_with(&self, suffix: &str) -> Option<&FunctionSignature> {
        let pattern = format!("::{}", suffix);
        self.signatures
            .iter()
            .find(|(key, _)| key.ends_with(&pattern))
            .map(|(_, sig)| sig)
    }

    /// Find a signature matching the simple name with a specific argument count.
    /// Searches exact match first, then all qualified names ending with `::name`.
    /// Used when simple-name lookup returns the wrong overload (name collision).
    pub fn find_signature_by_name_and_arg_count(
        &self,
        name: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        // Try exact match first
        if let Some(sig) = self.signatures.get(name) {
            let sig_args = if sig.has_self_receiver {
                sig.param_ownership.len().saturating_sub(1)
            } else {
                sig.param_ownership.len()
            };
            if sig_args == arg_count {
                return Some(sig);
            }
        }
        // Search all signatures ending with ::name
        let pattern = format!("::{}", name);
        for (key, sig) in &self.signatures {
            if key.ends_with(&pattern) || key == name {
                let sig_args = if sig.has_self_receiver {
                    sig.param_ownership.len().saturating_sub(1)
                } else {
                    sig.param_ownership.len()
                };
                if sig_args == arg_count {
                    return Some(sig);
                }
            }
        }
        None
    }

    /// BUG #8 FIX: Merge signatures from another registry.
    /// Detects collisions when different registries provide different
    /// param types for the same key (namespace collision from different modules).
    pub fn merge(&mut self, other: &SignatureRegistry) {
        for (name, sig) in &other.signatures {
            if let Some(existing) = self.signatures.get(name) {
                if existing.param_types != sig.param_types
                    || existing.param_ownership != sig.param_ownership
                {
                    self.collision_keys.insert(name.clone());
                }
            }
            self.signatures.insert(name.clone(), sig.clone());
        }
        self.collision_keys
            .extend(other.collision_keys.iter().cloned());
    }
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
    copy_structs: HashSet<String>,
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
    global_struct_field_types: HashMap<String, HashMap<String, Type>>,
}

use std::collections::HashSet;

impl<'ast> Default for Analyzer<'ast> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Analyzer<'ast> {
    pub fn new() -> Self {
        Self::new_with_copy_structs(HashSet::new())
    }

    /// Create a new Analyzer with a pre-populated set of Copy structs from global registry
    /// This enables proper Copy type detection across multiple files
    pub fn new_with_copy_structs(global_copy_structs: HashSet<String>) -> Self {
        let mut analyzer = Analyzer {
            variables: HashMap::new(),
            copy_enums: HashSet::new(),
            copy_structs: global_copy_structs, // Use global Copy structs from all files
            trait_definitions: HashMap::new(),
            analyzed_trait_methods: HashMap::new(),
            mutated_variables: HashSet::new(),
            current_impl_functions: None,
            self_impl_context: None,
            global_struct_field_types: HashMap::new(),
        };

        // Pre-register standard library traits so the analyzer knows their signatures
        analyzer.register_stdlib_traits();
        analyzer
            .hydrate_prelude_trait_method_signatures()
            .expect("prelude trait method analysis (Drop, etc.)");

        analyzer
    }

    /// Update the analyzer's Copy structs registry (for shared analyzer across files)
    /// This allows newly discovered Copy structs to be available for subsequent file analysis
    pub fn update_copy_structs(&mut self, global_copy_structs: HashSet<String>) {
        self.copy_structs = global_copy_structs;
    }

    /// Register a single struct as Copy (for cross-crate metadata or testing)
    pub fn register_copy_struct(&mut self, name: &str) {
        self.copy_structs.insert(name.to_string());
    }

    /// Set the global struct field types for cross-file nested field chain resolution.
    pub fn set_global_struct_field_types(
        &mut self,
        types: HashMap<String, HashMap<String, Type>>,
    ) {
        self.global_struct_field_types = types;
    }

    /// TDD FIX: Remove a struct from the Copy set (e.g., when local definition differs from metadata)
    pub fn unregister_copy_struct(&mut self, name: &str) {
        self.copy_structs.remove(name);
    }

    /// TDD FIX: Check if a struct is registered as Copy
    pub fn is_copy_struct(&self, name: &str) -> bool {
        self.copy_structs.contains(name)
    }

    /// Get all detected Copy struct names (for metadata emission)
    pub fn get_copy_structs(&self) -> Vec<String> {
        self.copy_structs.iter().cloned().collect()
    }

    /// Pre-register standard library traits (Add, Sub, Mul, Div, etc.)
    /// This allows the analyzer to correctly handle trait implementations
    /// for stdlib traits without needing to parse Rust's stdlib.
    fn register_stdlib_traits(&mut self) {
        use crate::parser::ast::{
            AssociatedType, OwnershipHint, Parameter, TraitDecl, TraitMethod, Type,
        };

        // Helper to create a binary operator trait (Add, Sub, Mul, Div, etc.)
        let create_binary_op_trait = |name: &str, method: &str| -> TraitDecl {
            TraitDecl {
                name: name.to_string(),
                generics: vec!["Rhs".to_string()],
                supertraits: vec![],
                methods: vec![TraitMethod {
                    name: method.to_string(),
                    parameters: vec![
                        Parameter {
                            name: "self".to_string(),
                            pattern: None,
                            type_: Type::Custom("Self".to_string()),
                            ownership: OwnershipHint::Owned,
                            is_mutable: false,
                            decorators: Vec::new(),
                        },
                        Parameter {
                            name: "rhs".to_string(),
                            pattern: None,
                            type_: Type::Custom("Rhs".to_string()),
                            ownership: OwnershipHint::Owned,
                            is_mutable: false,
                            decorators: Vec::new(),
                        },
                    ],
                    return_type: Some(Type::Custom("Output".to_string())),
                    is_async: false,
                    body: None,
                    doc_comment: None,
                }],
                associated_types: vec![AssociatedType {
                    name: "Output".to_string(),
                    concrete_type: None,
                }],
                doc_comment: None,
            }
        };

        // Register common operator traits
        self.trait_definitions
            .insert("Add".to_string(), create_binary_op_trait("Add", "add"));
        self.trait_definitions
            .insert("Sub".to_string(), create_binary_op_trait("Sub", "sub"));
        self.trait_definitions
            .insert("Mul".to_string(), create_binary_op_trait("Mul", "mul"));
        self.trait_definitions
            .insert("Div".to_string(), create_binary_op_trait("Div", "div"));
        self.trait_definitions
            .insert("Rem".to_string(), create_binary_op_trait("Rem", "rem"));

        // Register unary operator traits
        // Neg: -x
        self.trait_definitions.insert(
            "Neg".to_string(),
            TraitDecl {
                name: "Neg".to_string(),
                generics: vec![],
                supertraits: vec![],
                methods: vec![TraitMethod {
                    name: "neg".to_string(),
                    parameters: vec![Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Owned, // THE WINDJAMMER WAY: Neg uses owned self!
                        is_mutable: false,
                        decorators: Vec::new(),
                    }],
                    return_type: Some(Type::Custom("Output".to_string())),
                    is_async: false,
                    body: None,
                    doc_comment: None,
                }],
                associated_types: vec![AssociatedType {
                    name: "Output".to_string(),
                    concrete_type: None,
                }],
                doc_comment: None,
            },
        );

        // Rust std `Drop::drop(&mut self)` — Windjammer users write `fn drop(self)`; generated Rust
        // must match or rustc reports E0186/E0053. Not parsed from .wj, so register like operator traits.
        self.trait_definitions.insert(
            "Drop".to_string(),
            TraitDecl {
                name: "Drop".to_string(),
                generics: vec![],
                supertraits: vec![],
                methods: vec![TraitMethod {
                    name: "drop".to_string(),
                    parameters: vec![Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Mut,
                        is_mutable: false,
                        decorators: Vec::new(),
                    }],
                    return_type: None,
                    is_async: false,
                    body: None,
                    doc_comment: None,
                }],
                associated_types: vec![],
                doc_comment: None,
            },
        );
    }

    /// Analyze prelude traits that exist only in `trait_definitions` (no `trait Drop` in user source).
    fn hydrate_prelude_trait_method_signatures(&mut self) -> Result<(), String> {
        const PRELUDE_TRAIT_KEYS: &[&str] = &["Drop"];
        let empty_registry = SignatureRegistry::new();
        for &trait_key in PRELUDE_TRAIT_KEYS {
            let Some(decl) = self.trait_definitions.get(trait_key).cloned() else {
                continue;
            };
            let mut to_add: Vec<(String, AnalyzedFunction<'ast>)> = Vec::new();
            for method in &decl.methods {
                let already = self
                    .analyzed_trait_methods
                    .get(&decl.name)
                    .map(|m| m.contains_key(&method.name))
                    .unwrap_or(false);
                if already {
                    continue;
                }
                let func = FunctionDecl {
                    name: method.name.clone(),
                    is_pub: true,
                    is_extern: false,
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![],
                    is_async: method.is_async,
                    parameters: method.parameters.clone(),
                    return_type: method.return_type.clone(),
                    return_decorators: Vec::new(),
                    body: vec![],
                    parent_type: None,
                    impl_trait: None,
                    doc_comment: method.doc_comment.clone(),
                };
                let analyzed_func =
                    self.analyze_trait_method(&func, &empty_registry, Some(decl.name.as_str()))?;
                to_add.push((method.name.clone(), analyzed_func));
            }
            let entry = self
                .analyzed_trait_methods
                .entry(decl.name.clone())
                .or_default();
            for (name, analyzed_func) in to_add {
                entry.insert(name, analyzed_func);
            }
        }
        Ok(())
    }

    /// Register trait definitions from an external program (e.g., imported module)
    /// This allows the analyzer to use trait signatures when analyzing impl blocks
    /// in files that import traits from other modules.
    ///
    /// Also analyzes each trait method into `analyzed_trait_methods` when missing, so
    /// impl-only files compiled **before** the trait's source file still see the contract
    /// (receiver + parameter ownership/types). Without this, `trait_method_receiver_ownership`
    /// returns nothing and Rust emits E0053/E0186.
    pub fn register_traits_from_program(&mut self, program: &Program<'ast>) -> Result<(), String> {
        let empty_registry = SignatureRegistry::new();
        for item in &program.items {
            if let Item::Trait { decl, .. } = item {
                self.trait_definitions
                    .insert(decl.name.clone(), decl.clone());

                let mut to_add: Vec<(String, AnalyzedFunction<'ast>)> = Vec::new();
                for method in &decl.methods {
                    let already = self
                        .analyzed_trait_methods
                        .get(&decl.name)
                        .map(|m| m.contains_key(&method.name))
                        .unwrap_or(false);
                    if already {
                        continue;
                    }

                    // Skip body analysis for abstract trait methods (body = None)
                    // Only analyze trait methods with default implementations
                    if method.body.is_none() {
                        // For abstract trait methods, create a minimal FunctionDecl with empty body
                        // This avoids dereferencing invalid &'ast Statement references
                        let func = FunctionDecl {
                            name: method.name.clone(),
                            is_pub: true,
                            is_extern: false,
                            type_params: vec![],
                            where_clause: vec![],
                            decorators: vec![],
                            is_async: method.is_async,
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            return_decorators: Vec::new(),
                            body: vec![], // Empty body - no statements to dereference
                            parent_type: None,
                            impl_trait: None,
                            doc_comment: method.doc_comment.clone(),
                        };

                        // Analyze as trait method - this will infer ownership without walking body
                        let analyzed_func = self.analyze_trait_method(
                            &func,
                            &empty_registry,
                            Some(decl.name.as_str()),
                        )?;
                        to_add.push((method.name.clone(), analyzed_func));
                    } else {
                        // Trait method with default implementation - analyze fully
                        let func = FunctionDecl {
                            name: method.name.clone(),
                            is_pub: true,
                            is_extern: false,
                            type_params: vec![],
                            where_clause: vec![],
                            decorators: vec![],
                            is_async: method.is_async,
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            return_decorators: Vec::new(),
                            body: method.body.clone().unwrap_or_default(),
                            parent_type: None,
                            impl_trait: None,
                            doc_comment: method.doc_comment.clone(),
                        };
                        let analyzed_func = self.analyze_trait_method(
                            &func,
                            &empty_registry,
                            Some(decl.name.as_str()),
                        )?;
                        to_add.push((method.name.clone(), analyzed_func));
                    }
                }
                let entry = self
                    .analyzed_trait_methods
                    .entry(decl.name.clone())
                    .or_default();
                for (name, analyzed_func) in to_add {
                    entry.insert(name, analyzed_func);
                }
            }
        }
        Ok(())
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
        use crate::parser::ast::core::Expression;

        // Recursively check an expression for forbidden patterns
        fn check_expr(expr: &Expression) -> Result<(), String> {
            match expr {
                Expression::MethodCall {
                    method,
                    object,
                    arguments,
                    ..
                } => {
                    // FORBIDDEN: .as_str() - Rust-specific string conversion
                    // The compiler should handle this automatically based on context
                    if method == "as_str" && arguments.is_empty() {
                        return Err("error: `.as_str()` is forbidden in Windjammer source\n\
                             \n\
                             Windjammer automatically handles string conversions based on context.\n\
                             You don't need to call `.as_str()` - the compiler will generate the\n\
                             correct Rust code automatically.\n\
                             \n\
                             Example:\n\
                             ❌ match name.as_str() { ... }  // Don't do this\n\
                             ✅ match name { ... }            // Do this instead\n\
                             \n\
                             This keeps Windjammer code clean and backend-agnostic (Go/JS/etc\n\
                             don't have .as_str()).".to_string());
                    }

                    // Recursively check object and arguments
                    check_expr(object)?;
                    for (_label, arg) in arguments {
                        check_expr(arg)?;
                    }
                }
                Expression::Call {
                    function,
                    arguments,
                    ..
                } => {
                    // ALSO check Call expressions - .as_str() might be parsed as Call(FieldAccess)
                    if let Expression::FieldAccess { field, .. } = &**function {
                        if field == "as_str" && arguments.is_empty() {
                            return Err("error: `.as_str()` is forbidden in Windjammer source\n\
                                 \n\
                                 Windjammer automatically handles string conversions based on context.\n\
                                 You don't need to call `.as_str()` - the compiler will generate the\n\
                                 correct Rust code automatically.\n\
                                 \n\
                                 Example:\n\
                                 ❌ match name.as_str() { ... }  // Don't do this\n\
                                 ✅ match name { ... }            // Do this instead\n\
                                 \n\
                                 This keeps Windjammer code clean and backend-agnostic (Go/JS/etc\n\
                                 don't have .as_str()).".to_string());
                        }
                    }

                    check_expr(function)?;
                    for (_label, arg) in arguments {
                        check_expr(arg)?;
                    }
                }
                Expression::Binary { left, right, .. } => {
                    check_expr(left)?;
                    check_expr(right)?;
                }
                Expression::Unary { operand, .. } => {
                    check_expr(operand)?;
                }
                Expression::FieldAccess { object, .. } => {
                    check_expr(object)?;
                }
                Expression::Index { object, index, .. } => {
                    check_expr(object)?;
                    check_expr(index)?;
                }
                Expression::StructLiteral { fields, .. } => {
                    for (_name, value) in fields {
                        check_expr(value)?;
                    }
                }
                Expression::Array { elements, .. } => {
                    for elem in elements {
                        check_expr(elem)?;
                    }
                }
                Expression::Cast { expr, .. } => {
                    check_expr(expr)?;
                }
                Expression::Closure { body, .. } => {
                    check_expr(body)?;
                }
                Expression::Tuple { elements, .. } => {
                    for elem in elements {
                        check_expr(elem)?;
                    }
                }
                Expression::Range { start, end, .. } => {
                    check_expr(start)?;
                    check_expr(end)?;
                }
                Expression::MapLiteral { pairs, .. } => {
                    for (key, value) in pairs {
                        check_expr(key)?;
                        check_expr(value)?;
                    }
                }
                Expression::TryOp { expr, .. } => {
                    check_expr(expr)?;
                }
                Expression::Await { expr, .. } => {
                    check_expr(expr)?;
                }
                Expression::ChannelSend { channel, value, .. } => {
                    check_expr(channel)?;
                    check_expr(value)?;
                }
                Expression::ChannelRecv { channel, .. } => {
                    check_expr(channel)?;
                }
                Expression::Block { statements, .. } => {
                    for stmt in statements {
                        check_stmt(stmt)?;
                    }
                }
                Expression::MacroInvocation { args, .. } => {
                    for arg in args {
                        check_expr(arg)?;
                    }
                }
                // Base cases - no sub-expressions
                Expression::Literal { .. } | Expression::Identifier { .. } => {}
            }
            Ok(())
        }

        fn check_stmt(stmt: &Statement) -> Result<(), String> {
            match stmt {
                Statement::Let {
                    value, else_block, ..
                } => {
                    check_expr(value)?;
                    if let Some(block) = else_block {
                        for s in block {
                            check_stmt(s)?;
                        }
                    }
                }
                Statement::Const { value, .. } | Statement::Static { value, .. } => {
                    check_expr(value)?;
                }
                Statement::Assignment { value, target, .. } => {
                    check_expr(value)?;
                    check_expr(target)?;
                }
                Statement::Expression { expr, .. } => {
                    check_expr(expr)?;
                }
                Statement::Return { value, .. } => {
                    if let Some(val) = value {
                        check_expr(val)?;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    check_expr(condition)?;
                    for s in then_block {
                        check_stmt(s)?;
                    }
                    if let Some(else_stmts) = else_block {
                        for s in else_stmts {
                            check_stmt(s)?;
                        }
                    }
                }
                Statement::Match { value, arms, .. } => {
                    check_expr(value)?;
                    for arm in arms {
                        check_expr(arm.body)?;
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    check_expr(condition)?;
                    for s in body {
                        check_stmt(s)?;
                    }
                }
                Statement::For { iterable, body, .. } => {
                    check_expr(iterable)?;
                    for s in body {
                        check_stmt(s)?;
                    }
                }
                Statement::Loop { body, .. }
                | Statement::Thread { body, .. }
                | Statement::Async { body, .. } => {
                    for s in body {
                        check_stmt(s)?;
                    }
                }
                Statement::Defer { statement, .. } => {
                    check_stmt(statement)?;
                }
                Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
            }
            Ok(())
        }

        // Check all items in the program
        for item in &program.items {
            match item {
                Item::Function { decl, .. } => {
                    for stmt in &decl.body {
                        check_stmt(stmt)?;
                    }
                }
                Item::Impl { block, .. } => {
                    for func in &block.functions {
                        for stmt in &func.body {
                            check_stmt(stmt)?;
                        }
                    }
                }
                Item::Trait { decl, .. } => {
                    for method in &decl.methods {
                        if let Some(body) = &method.body {
                            for stmt in body {
                                check_stmt(stmt)?;
                            }
                        }
                    }
                }
                Item::Const { value, .. } | Item::Static { value, .. } => {
                    check_expr(value)?;
                }
                Item::Mod { items, .. } => {
                    // Recursively check module items
                    let mod_program = Program {
                        items: items.clone(),
                    };
                    self.check_forbidden_rust_patterns(&mod_program)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Analyze a program with pre-populated signatures from previously compiled files.
    /// This enables cross-file passthrough ownership inference (e.g., Merchant::add_item
    /// can look up Inventory::add_item's ownership when they're in separate files).
    pub fn analyze_program_with_global_signatures(
        &mut self,
        program: &Program<'ast>,
        global_signatures: &SignatureRegistry,
    ) -> Result<ProgramAnalysisResult<'ast>, String> {
        // THE PROPER SOLUTION: Multi-pass ownership analysis
        // Iterate until convergence - no workarounds, no heuristics, just correctness

        // PHASE -1: LANGUAGE DESIGN CHECK - Prohibit Rust-specific `.as_str()`
        // Windjammer compiler should handle string conversions automatically.
        // Users shouldn't need to know about Rust's &str vs String distinction.
        self.check_forbidden_rust_patterns(program)?;

        // PHASE 0: Collect all enum, struct, and trait definitions
        // This must happen before any function analysis
        for item in &program.items {
            match item {
                Item::Enum { decl, .. } => {
                    // Fieldless enums (unit variants only) are Copy by default
                    use crate::parser::ast::EnumVariantData;
                    let is_copy = decl
                        .variants
                        .iter()
                        .all(|v| matches!(v.data, EnumVariantData::Unit));
                    if is_copy {
                        self.copy_enums.insert(decl.name.clone());
                    }
                }
                Item::Trait { decl, .. } => {
                    // Store trait definition for later lookup
                    self.trait_definitions
                        .insert(decl.name.clone(), decl.clone());
                }
                _ => {}
            }
        }

        // PHASE 0b: Struct Copy registry — fixed-point to match codegen and main.rs PASS 0.
        // Single forward pass fails when struct A references Copy struct B but B is declared
        // later in the file; empty structs must be Copy (same as Rust / trait_derivation).
        let mut struct_infos: Vec<(String, Vec<Type>)> = Vec::new();
        for item in &program.items {
            if let Item::Struct { decl, .. } = item {
                let has_copy_derive = decl.decorators.iter().any(|decorator| {
                    decorator.name == "derive"
                        && decorator.arguments.iter().any(|(_, arg)| {
                            if let crate::parser::ast::Expression::Identifier { name, .. } = arg {
                                name == "Copy"
                            } else {
                                false
                            }
                        })
                });
                if has_copy_derive {
                    self.copy_structs.insert(decl.name.clone());
                }
                struct_infos.push((
                    decl.name.clone(),
                    decl.fields.iter().map(|f| f.field_type.clone()).collect(),
                ));
            }
        }
        const MAX_COPY_STRUCT_PASSES: usize = 64;
        for _ in 0..MAX_COPY_STRUCT_PASSES {
            let mut changed = false;
            for (name, field_types) in &struct_infos {
                if self.copy_structs.contains(name) {
                    continue;
                }
                let all_copy =
                    field_types.is_empty() || field_types.iter().all(|ft| self.is_copy_type(ft));
                if all_copy {
                    self.copy_structs.insert(name.clone());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // MULTI-PASS OWNERSHIP INFERENCE
        // Continue analyzing until ownership signatures stabilize (convergence)
        const MAX_PASSES: usize = 10; // Safety limit to prevent infinite loops

        let mut registry = global_signatures.clone();
        let mut pass_number = 1;

        loop {
            let (new_analyzed, new_registry) = self.analyze_program_pass(program, &registry)?;

            // Check for convergence: did any signatures change?
            let converged = self.signatures_converged(&registry, &new_registry);

            if converged {
                return Ok((
                    new_analyzed,
                    new_registry,
                    self.analyzed_trait_methods.clone(),
                ));
            }

            if pass_number >= MAX_PASSES {
                eprintln!(
                    "⚠️  Warning: Ownership analysis did not converge after {} passes",
                    MAX_PASSES
                );
                eprintln!("    Using last known signatures (may be suboptimal)");
                return Ok((
                    new_analyzed,
                    new_registry,
                    self.analyzed_trait_methods.clone(),
                ));
            }

            // Update registry for next pass
            registry = new_registry;
            pass_number += 1;
        }
    }

    /// Helper: Check if two signature registries have converged (no changes)
    fn signatures_converged(&self, old: &SignatureRegistry, new: &SignatureRegistry) -> bool {
        // If sizes differ, not converged
        if old.signatures.len() != new.signatures.len() {
            return false;
        }

        // Compare each signature
        for (name, new_sig) in &new.signatures {
            match old.signatures.get(name) {
                None => return false, // New function appeared
                Some(old_sig) => {
                    // Compare parameter ownership modes
                    if old_sig.param_ownership.len() != new_sig.param_ownership.len() {
                        return false;
                    }

                    for (old_ownership, new_ownership) in
                        old_sig.param_ownership.iter().zip(&new_sig.param_ownership)
                    {
                        if old_ownership != new_ownership {
                            return false;
                        }
                    }

                    // Compare return type ownership
                    if old_sig.return_ownership != new_sig.return_ownership {
                        return false;
                    }
                }
            }
        }

        true // All signatures match
    }

    /// Helper: Single pass of program analysis
    /// Uses the provided registry to infer ownership, returns updated analysis and registry
    fn analyze_program_pass(
        &mut self,
        program: &Program<'ast>,
        existing_registry: &SignatureRegistry,
    ) -> Result<(Vec<AnalyzedFunction<'ast>>, SignatureRegistry), String> {
        let mut analyzed = Vec::new();
        let mut registry = existing_registry.clone();

        // NOTE: Trait signature inference is now done GLOBALLY after all files are compiled
        // See ModuleCompiler::finalize_trait_inference() in main.rs
        // (We no longer call infer_trait_signatures_from_impls here for single files)

        for item in &program.items {
            match item {
                Item::Function { decl: func, .. } => {
                    let mut analyzed_func = self.analyze_function(func, &registry)?;

                    // PHASE 7: Detect const/static optimizations
                    analyzed_func.const_static_optimizations =
                        self.detect_const_static_opportunities(&analyzed_func);

                    // PHASE 8: Detect SmallVec optimizations
                    analyzed_func.smallvec_optimizations = self.detect_smallvec_opportunities(func);

                    // PHASE 9: Detect Cow optimizations
                    analyzed_func.cow_optimizations = self.detect_cow_opportunities(func);

                    let signature = self.build_signature(&analyzed_func);
                    registry.add_function(func.name.clone(), signature);
                    analyzed.push(analyzed_func);
                }
                Item::Impl {
                    block: impl_block, ..
                } => {
                    // TDD FIX: Multi-pass fixed-point iteration for transitive mutability inference
                    //
                    // Problem: Single-pass analysis fails for multi-level call chains:
                    //   update() calls poll_input() which calls keyboard.update_key(&mut self)
                    //   Single pass: update(&self) ❌ (wrong!)
                    //   Multi-pass: update(&mut self) ✅ (correct!)
                    //
                    // Solution: Iterate until no signatures change (fixed-point)
                    let mut analyzed_funcs: std::collections::HashMap<
                        String,
                        AnalyzedFunction<'ast>,
                    > = std::collections::HashMap::new();
                    let mut local_registry = registry.clone();

                    // Pass 1: Initial analysis (direct mutations only)
                    for func in &impl_block.functions {
                        let analyzed_func = if let Some(trait_name) = &impl_block.trait_name {
                            self.analyze_trait_impl_function(
                                func,
                                trait_name,
                                impl_block,
                                program,
                                &local_registry,
                            )?
                        } else {
                            self.analyze_function_in_impl(
                                func,
                                impl_block,
                                program,
                                &local_registry,
                            )?
                        };
                        analyzed_funcs.insert(func.name.clone(), analyzed_func);
                    }

                    // Pass 2-N: Fixed-point iteration (propagate transitive mutations)
                    let mut changed = true;
                    let mut iteration = 0;
                    const MAX_ITERATIONS: usize = 10; // Safety limit

                    while changed && iteration < MAX_ITERATIONS {
                        changed = false;
                        iteration += 1;

                        // Update local registry with current analyzed signatures
                        for (name, analyzed_func) in &analyzed_funcs {
                            let signature = self.build_signature(analyzed_func);
                            let qualified_name = format!("{}::{}", impl_block.type_name, name);
                            local_registry.add_function(qualified_name, signature.clone());
                            local_registry.add_function(name.clone(), signature);
                        }

                        // Re-analyze all methods with updated registry
                        for func in &impl_block.functions {
                            let new_analyzed = if let Some(trait_name) = &impl_block.trait_name {
                                self.analyze_trait_impl_function(
                                    func,
                                    trait_name,
                                    impl_block,
                                    program,
                                    &local_registry,
                                )?
                            } else {
                                self.analyze_function_in_impl(
                                    func,
                                    impl_block,
                                    program,
                                    &local_registry,
                                )?
                            };

                            // Check if self ownership changed
                            let old_analyzed = &analyzed_funcs[&func.name];
                            let old_self_ownership = old_analyzed
                                .inferred_ownership
                                .get("self")
                                .copied()
                                .unwrap_or(OwnershipMode::Owned);
                            let new_self_ownership = new_analyzed
                                .inferred_ownership
                                .get("self")
                                .copied()
                                .unwrap_or(OwnershipMode::Owned);

                            if old_self_ownership != new_self_ownership {
                                analyzed_funcs.insert(func.name.clone(), new_analyzed);
                                changed = true;
                            }
                        }
                    }

                    // Process all analyzed functions (after fixed-point convergence)
                    let is_trait_impl = impl_block.trait_name.is_some();
                    for func in &impl_block.functions {
                        let analyzed_func_opt = analyzed_funcs.remove(&func.name);
                        if analyzed_func_opt.is_none() {
                            // Duplicate function name in impl block -- skip the second
                            // occurrence. The first definition wins (already processed).
                            continue;
                        }
                        let mut analyzed_func = analyzed_func_opt.unwrap();

                        // PHASE 7: Detect const/static optimizations
                        analyzed_func.const_static_optimizations =
                            self.detect_const_static_opportunities(&analyzed_func);

                        // PHASE 8: Detect SmallVec optimizations
                        analyzed_func.smallvec_optimizations =
                            self.detect_smallvec_opportunities(func);

                        // PHASE 9: Detect Cow optimizations
                        analyzed_func.cow_optimizations = self.detect_cow_opportunities(func);

                        let signature = self.build_signature(&analyzed_func);

                        let qualified_name = format!("{}::{}", impl_block.type_name, func.name);
                        if is_trait_impl {
                            // Trait impl methods: don't overwrite a direct impl's entry.
                            // Callers like `obj.method()` resolve to the direct impl in Rust,
                            // so the registry's Type::method entry must reflect the direct impl's
                            // signature (parameter types and ownership).
                            if registry.get_signature(&qualified_name).is_none() {
                                registry.add_function(qualified_name.clone(), signature.clone());
                            }
                            // Also register under Trait::method for trait-based lookups.
                            if let Some(trait_name) = &impl_block.trait_name {
                                let trait_qualified = format!("{}::{}", trait_name, func.name);
                                registry.add_function(trait_qualified, signature.clone());
                            }
                        } else {
                            // Direct impl methods always take priority in the registry.
                            registry.add_function(qualified_name.clone(), signature.clone());
                        }
                        // Generic type base name registration
                        if let Some(base_name) = impl_block.type_name.split('<').next() {
                            if base_name != impl_block.type_name {
                                let base_qualified = format!("{}::{}", base_name, func.name);
                                if !is_trait_impl
                                    || registry.get_signature(&base_qualified).is_none()
                                {
                                    registry.add_function(base_qualified, signature.clone());
                                }
                            }
                        }
                        if !is_trait_impl || registry.get_signature(&func.name).is_none() {
                            registry.add_function(func.name.clone(), signature);
                        }

                        analyzed.push(analyzed_func);
                    }
                }
                Item::Trait { decl, .. } => {
                    // THE WINDJAMMER WAY: Analyze ALL trait methods, not just default impls.
                    // Abstract methods need ownership inference too - the compiler must set
                    // the correct self convention (&self, &mut self) even without a body.
                    // This is refined later by infer_trait_signatures_from_impls.
                    for method in &decl.methods {
                        // Convert TraitMethod to FunctionDecl for analysis
                        let func = FunctionDecl {
                            name: method.name.clone(),
                            is_pub: true, // Trait methods are public
                            is_extern: false,
                            type_params: vec![],
                            where_clause: vec![],
                            decorators: vec![],
                            is_async: method.is_async,
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            return_decorators: Vec::new(),
                            body: method.body.clone().unwrap_or_default(),
                            parent_type: None,
                            impl_trait: None,
                            doc_comment: method.doc_comment.clone(),
                        };

                        // Trait methods (both abstract and default) should use &self or &mut self
                        // to work with unsized types. The Windjammer way: make it work!
                        let mut analyzed_func =
                            self.analyze_trait_method(&func, &registry, Some(decl.name.as_str()))?;

                        // PHASE 7: Detect const/static optimizations
                        analyzed_func.const_static_optimizations =
                            self.detect_const_static_opportunities(&analyzed_func);

                        // PHASE 8: Detect SmallVec optimizations
                        analyzed_func.smallvec_optimizations =
                            self.detect_smallvec_opportunities(&func);

                        // PHASE 9: Detect Cow optimizations
                        analyzed_func.cow_optimizations = self.detect_cow_opportunities(&func);

                        // THE WINDJAMMER WAY: Store analyzed trait method for trait impl matching
                        // BUT: Don't overwrite if cross-file inference has already set it!
                        // (finalize_trait_inference runs globally and sets the most permissive signature)
                        let trait_methods = self
                            .analyzed_trait_methods
                            .entry(decl.name.clone())
                            .or_default();

                        // Merge: if the impl body infers a stronger ownership
                        // than the abstract trait stub, upgrade the trait entry.
                        if let Some(existing) = trait_methods.get(&func.name) {
                            let existing_self = existing.inferred_ownership.get("self").copied();
                            let new_self = analyzed_func.inferred_ownership.get("self").copied();
                            let should_upgrade = matches!(
                                (existing_self, new_self),
                                (None, Some(_))
                                    | (
                                        Some(OwnershipMode::Borrowed),
                                        Some(OwnershipMode::MutBorrowed | OwnershipMode::Owned)
                                    )
                                    | (
                                        Some(OwnershipMode::MutBorrowed),
                                        Some(OwnershipMode::Owned)
                                    )
                            );
                            if should_upgrade {
                                trait_methods.insert(func.name.clone(), analyzed_func.clone());
                            }
                        } else {
                            trait_methods.insert(func.name.clone(), analyzed_func.clone());
                        }

                        // Add trait methods to analyzed list so codegen can access ownership info
                        // They won't be generated as standalone functions (codegen skips trait methods)
                        let signature = self.build_signature(&analyzed_func);
                        registry.add_function(func.name.clone(), signature);
                        analyzed.push(analyzed_func);
                    }
                }
                Item::Static { mutable, value, .. } => {
                    // Analyze static declarations for const promotion
                    if !mutable && self.is_const_evaluable(value) {
                        // This static can be promoted to const
                        // Store in a global optimization list (TODO: add to Program-level analysis)
                    }
                }
                Item::Mod { items, .. } => {
                    // Recursively analyze items inside inline modules
                    // NOTE: We analyze them for signature registry, but don't add them
                    // to the top-level analyzed list since they'll be generated inside their modules
                    for item in items {
                        match item {
                            Item::Function { decl: func, .. } => {
                                let mut analyzed_func = self.analyze_function(func, &registry)?;
                                analyzed_func.const_static_optimizations =
                                    self.detect_const_static_opportunities(&analyzed_func);
                                analyzed_func.smallvec_optimizations =
                                    self.detect_smallvec_opportunities(func);
                                analyzed_func.cow_optimizations =
                                    self.detect_cow_opportunities(func);
                                let signature = self.build_signature(&analyzed_func);
                                registry.add_function(func.name.clone(), signature);
                                // Add to analyzed list for codegen to access (but marked as in-module)
                                analyzed.push(analyzed_func);
                            }
                            Item::Impl {
                                block: impl_block, ..
                            } => {
                                // TDD FIX: Multi-pass fixed-point iteration (same as top-level impl blocks)
                                let mut analyzed_funcs: std::collections::HashMap<
                                    String,
                                    AnalyzedFunction<'ast>,
                                > = std::collections::HashMap::new();
                                let mut local_registry = registry.clone();

                                // Pass 1: Initial analysis
                                for func in &impl_block.functions {
                                    let analyzed_func =
                                        if let Some(trait_name) = &impl_block.trait_name {
                                            self.analyze_trait_impl_function(
                                                func,
                                                trait_name,
                                                impl_block,
                                                program,
                                                &local_registry,
                                            )?
                                        } else {
                                            self.analyze_function_in_impl(
                                                func,
                                                impl_block,
                                                program,
                                                &local_registry,
                                            )?
                                        };
                                    analyzed_funcs.insert(func.name.clone(), analyzed_func);
                                }

                                // Pass 2-N: Fixed-point iteration
                                let mut changed = true;
                                let mut iteration = 0;
                                const MAX_ITERATIONS: usize = 10;

                                while changed && iteration < MAX_ITERATIONS {
                                    changed = false;
                                    iteration += 1;

                                    // Update registry
                                    for (name, analyzed_func) in &analyzed_funcs {
                                        let signature = self.build_signature(analyzed_func);
                                        local_registry.add_function(name.clone(), signature);
                                    }

                                    // Re-analyze
                                    for func in &impl_block.functions {
                                        let new_analyzed =
                                            if let Some(trait_name) = &impl_block.trait_name {
                                                self.analyze_trait_impl_function(
                                                    func,
                                                    trait_name,
                                                    impl_block,
                                                    program,
                                                    &local_registry,
                                                )?
                                            } else {
                                                self.analyze_function_in_impl(
                                                    func,
                                                    impl_block,
                                                    program,
                                                    &local_registry,
                                                )?
                                            };

                                        // Check if ownership changed
                                        let old_analyzed = &analyzed_funcs[&func.name];
                                        let old_self = old_analyzed
                                            .inferred_ownership
                                            .get("self")
                                            .copied()
                                            .unwrap_or(OwnershipMode::Owned);
                                        let new_self = new_analyzed
                                            .inferred_ownership
                                            .get("self")
                                            .copied()
                                            .unwrap_or(OwnershipMode::Owned);

                                        if old_self != new_self {
                                            analyzed_funcs.insert(func.name.clone(), new_analyzed);
                                            changed = true;
                                        }
                                    }
                                }

                                // Process converged results
                                let is_trait_impl = impl_block.trait_name.is_some();
                                for func in &impl_block.functions {
                                    let mut analyzed_func = analyzed_funcs
                                        .remove(&func.name)
                                        .expect("Function should exist");

                                    analyzed_func.const_static_optimizations =
                                        self.detect_const_static_opportunities(&analyzed_func);
                                    analyzed_func.smallvec_optimizations =
                                        self.detect_smallvec_opportunities(func);
                                    analyzed_func.cow_optimizations =
                                        self.detect_cow_opportunities(func);

                                    let signature = self.build_signature(&analyzed_func);
                                    let qualified_name =
                                        format!("{}::{}", impl_block.type_name, func.name);
                                    if is_trait_impl {
                                        if registry.get_signature(&qualified_name).is_none() {
                                            registry
                                                .add_function(qualified_name, signature.clone());
                                        }
                                        if let Some(trait_name) = &impl_block.trait_name {
                                            let trait_qualified =
                                                format!("{}::{}", trait_name, func.name);
                                            registry
                                                .add_function(trait_qualified, signature.clone());
                                        }
                                    } else {
                                        registry.add_function(qualified_name, signature.clone());
                                    }
                                    if !is_trait_impl
                                        || registry.get_signature(&func.name).is_none()
                                    {
                                        registry.add_function(func.name.clone(), signature);
                                    }
                                    analyzed.push(analyzed_func);
                                }
                            }
                            // Could recursively handle nested modules here
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((analyzed, registry))
    }

    /// Analyze a trait method with default implementation
    /// Trait methods must use &self or &mut self (not owned self)
    /// to work with unsized types
    /// Helper: Convert OwnershipHint to OwnershipMode
    fn convert_ownership_hint_to_mode(
        &self,
        hint: &OwnershipHint,
        param_name: &str,
    ) -> OwnershipMode {
        match hint {
            OwnershipHint::Owned => OwnershipMode::Owned,
            OwnershipHint::Ref => OwnershipMode::Borrowed,
            OwnershipHint::Mut => OwnershipMode::MutBorrowed,
            OwnershipHint::Inferred => {
                // For inferred parameters, default to borrowed for self, owned otherwise
                if param_name == "self" {
                    OwnershipMode::Borrowed
                } else {
                    OwnershipMode::Owned
                }
            }
        }
    }

    /// THE WINDJAMMER WAY: Infer trait method signatures from ALL implementations
    /// If any impl needs &mut self, the trait gets &mut self
    /// The compiler does the work, not the user!
    pub fn infer_trait_signatures_from_impls(
        &mut self,
        program: &Program<'ast>,
    ) -> Result<(), String> {
        use std::collections::HashMap;

        // Step 1: Collect all trait implementations WITH their impl blocks
        // THE WINDJAMMER WAY: We need the impl block for proper ownership analysis
        // Map: trait_name -> Vec<(ImplBlock, functions)>
        let mut trait_impls: HashMap<String, Vec<crate::parser::ast::ImplBlock>> = HashMap::new();

        for item in &program.items {
            if let Item::Impl {
                block: impl_block, ..
            } = item
            {
                if let Some(trait_name) = &impl_block.trait_name {
                    trait_impls
                        .entry(trait_name.clone())
                        .or_default()
                        .push(impl_block.clone());
                }
            }
        }

        // Step 2: For each trait, analyze ALL implementations and determine most permissive signature
        for (trait_name, impl_blocks) in trait_impls {
            let trait_methods_opt = self
                .analyzed_trait_methods
                .get(&trait_name)
                .cloned()
                .or_else(|| {
                    trait_name
                        .rfind("::")
                        .map(|i| trait_name[i + 2..].to_string())
                        .and_then(|short| self.analyzed_trait_methods.get(&short).cloned())
                });

            if let Some(trait_methods) = trait_methods_opt {
                let mut updated_methods = HashMap::new();

                for (method_name, mut trait_method_analysis) in trait_methods {
                    // Trait receiver is the contract. When any impl exists in this program, derive
                    // `self` **only** from those impls (merge with max-permissive: &mut beats &).
                    // Trait-only crates keep `analyze_trait_method` defaults (abstract → &mut self).
                    //
                    // Associated functions (`fn create() -> Self`) have no `self` entry — skip.

                    if !trait_method_analysis
                        .inferred_ownership
                        .contains_key("self")
                    {
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    if matches!(
                        trait_method_analysis.inferred_ownership.get("self"),
                        Some(OwnershipMode::Owned)
                    ) {
                        // Consuming `self` on the trait is authoritative; do not refine from impls.
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    let trait_lookup_key = trait_name
                        .rfind("::")
                        .map(|i| &trait_name[i + 2..])
                        .unwrap_or(trait_name.as_str());
                    let explicit_mut_self_contract = self
                        .trait_definitions
                        .get(trait_lookup_key)
                        .and_then(|decl| decl.methods.iter().find(|m| m.name == method_name))
                        .and_then(|m| m.parameters.iter().find(|p| p.name == "self"))
                        .is_some_and(|p| matches!(p.ownership, OwnershipHint::Mut));
                    if explicit_mut_self_contract {
                        // Rust std / user wrote `&mut self` on the trait — never refine from impls.
                        // Otherwise `infer_trait_signatures_from_impls` can replace `&mut self` with `&self`
                        // (e.g. `Drop::drop` vs Windjammer `fn drop(self)`), causing E0186/E0053.
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    let mut merged_from_impls: Option<OwnershipMode> = None;

                    for impl_block in &impl_blocks {
                        for func in &impl_block.functions {
                            if func.name == method_name {
                                let empty_registry = SignatureRegistry::new();
                                let impl_analysis = self.analyze_function_in_impl(
                                    func,
                                    impl_block,
                                    program,
                                    &empty_registry,
                                )?;

                                if let Some(&impl_self_ownership) =
                                    impl_analysis.inferred_ownership.get("self")
                                {
                                    merged_from_impls = Some(match merged_from_impls {
                                        None => impl_self_ownership,
                                        Some(acc) => Self::merge_borrow_trait_receivers(
                                            acc,
                                            impl_self_ownership,
                                        ),
                                    });
                                }
                            }
                        }
                    }

                    if let Some(merged_self) = merged_from_impls {
                        trait_method_analysis
                            .inferred_ownership
                            .insert("self".to_string(), merged_self);
                    }

                    updated_methods.insert(method_name, trait_method_analysis);
                }

                // Store under the impl's trait name and, when qualified, also under the final segment
                // so `analyzed_trait_methods.get("GameLoop")` and `.get("crate::GameLoop")` stay in sync.
                self.analyzed_trait_methods
                    .insert(trait_name.clone(), updated_methods.clone());
                if let Some(pos) = trait_name.rfind("::") {
                    let short = trait_name[pos + 2..].to_string();
                    if short != trait_name {
                        self.analyzed_trait_methods.insert(short, updated_methods);
                    }
                }
            }
        }

        Ok(())
    }

    fn analyze_trait_method(
        &mut self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
        trait_name: Option<&str>,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        // Analyze the function normally first
        let mut analyzed = self.analyze_function(func, registry)?;

        // WINDJAMMER PHILOSOPHY: Ownership is a mechanical detail the compiler handles
        // - User writes trait methods without thinking about ownership
        // - Compiler infers optimal ownership from usage
        // - For explicit `&self` or `&mut self`, preserve them (user explicitly requested)
        // - For `self` (inferred), analyze body/implementations and optimize

        for param in &func.parameters {
            if param.name == "self" {
                match &param.ownership {
                    OwnershipHint::Ref => {
                        // User explicitly wrote &self - preserve it
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), OwnershipMode::Borrowed);
                    }
                    OwnershipHint::Mut => {
                        // User explicitly wrote &mut self - preserve it
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), OwnershipMode::MutBorrowed);
                    }
                    OwnershipHint::Owned => {
                        // Explicit consuming `self` in the trait (e.g. fn consume(self) -> T)
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), OwnershipMode::Owned);
                    }
                    OwnershipHint::Inferred => {
                        // Omitted receiver: infer from trait body. Abstract: void → &mut self; with return → &self.
                        let modifies_self =
                            self.function_modifies_self_fields_with_registry(func, Some(registry));
                        let self_ownership = if modifies_self {
                            OwnershipMode::MutBorrowed
                        } else if func.body.is_empty() {
                            if func.return_type.is_some() {
                                OwnershipMode::Borrowed
                            } else {
                                OwnershipMode::MutBorrowed
                            }
                        } else {
                            OwnershipMode::Borrowed
                        };
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), self_ownership);
                    }
                }
            } else {
                // Non-self parameters: preserve explicit, infer otherwise
                let ownership = match &param.ownership {
                    OwnershipHint::Ref => OwnershipMode::Borrowed,
                    OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                    OwnershipHint::Owned | OwnershipHint::Inferred => OwnershipMode::Owned,
                };
                analyzed
                    .inferred_ownership
                    .insert(param.name.clone(), ownership);
            }
        }

        // E0053 FIX: Trait methods without explicit self (e.g. fn initialize()) need self for impl matching.
        // Windjammer trait methods often omit self - add default so infer_trait_signatures_from_impls can upgrade.
        //
        // Associated functions (constructors / `fn make() -> MyTrait`): no receiver in Rust.
        // Detect by: common factory names, `-> Self`, or `-> TraitName` for the trait being defined.
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");
        let is_named_constructor = crate::type_classification::is_constructor_name(&func.name);
        let returns_associated_type = matches!(
            &func.return_type,
            Some(Type::Custom(name))
                if name == "Self" || trait_name.is_some_and(|t| t == name.as_str())
        );
        let is_associated_fn =
            !has_explicit_self && (is_named_constructor || returns_associated_type);
        if !analyzed.inferred_ownership.contains_key("self") && !is_associated_fn {
            let default_receiver = if func.return_type.is_some() {
                OwnershipMode::Borrowed
            } else {
                OwnershipMode::MutBorrowed
            };
            analyzed
                .inferred_ownership
                .insert("self".to_string(), default_receiver);
        }

        Ok(analyzed)
    }

    /// Merge receiver ownership from impls: strongest wins.
    /// `Owned` (consuming) > `MutBorrowed` (&mut self) > `Borrowed` (&self).
    fn merge_borrow_trait_receivers(a: OwnershipMode, b: OwnershipMode) -> OwnershipMode {
        use OwnershipMode::*;
        match (a, b) {
            (Owned, _) | (_, Owned) => Owned,
            (MutBorrowed, _) | (_, MutBorrowed) => MutBorrowed,
            _ => Borrowed,
        }
    }

    /// All methods for `type_name` across every `impl` block in the program (including inherent + trait impls).
    /// Used so `self.helper()` in `impl Trait for T` resolves `helper` from `impl T` on the same type.
    fn merged_impl_methods_for_type(
        program: &Program<'ast>,
        type_name: &str,
    ) -> HashMap<String, FunctionDecl<'ast>> {
        let type_base = type_name.split('<').next().unwrap_or(type_name);
        let mut merged = HashMap::new();
        Self::collect_impl_methods_recursive(&program.items, type_base, &mut merged);
        merged
    }

    fn collect_impl_methods_recursive(
        items: &[Item<'ast>],
        type_base: &str,
        merged: &mut HashMap<String, FunctionDecl<'ast>>,
    ) {
        for item in items {
            match item {
                Item::Impl { block, .. } => {
                    let block_base = block
                        .type_name
                        .split('<')
                        .next()
                        .unwrap_or(&block.type_name);
                    if block_base == type_base {
                        for f in &block.functions {
                            merged.insert(f.name.clone(), f.clone());
                        }
                    }
                }
                Item::Mod { items: inner, .. } => {
                    Self::collect_impl_methods_recursive(inner, type_base, merged);
                }
                _ => {}
            }
        }
    }

    /// Analyze a function within an impl block (has access to other methods for cross-method analysis)
    fn analyze_function_in_impl(
        &mut self,
        func: &FunctionDecl<'ast>,
        impl_block: &crate::parser::ast::ImplBlock<'ast>,
        program: &Program<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        // Same-type impl merge: trait impl methods can call inherent helpers on the same type.
        self.current_impl_functions = Some(Self::merged_impl_methods_for_type(
            program,
            &impl_block.type_name,
        ));
        let impl_base = impl_block
            .type_name
            .split('<')
            .next()
            .unwrap_or(impl_block.type_name.as_str())
            .to_string();
        self.self_impl_context = Some(ImplSelfFieldContext::new(impl_base, program));
        let mut analyzed = self.analyze_function(func, registry)?;

        // Inherent impls: `for x in self.field` + `x.foo()` where `foo` is `&mut self` on a trait object
        // requires `&mut self` on the outer method (codegen emits `&mut self.field`).
        if impl_block.trait_name.is_none() {
            self.maybe_upgrade_self_for_dispatch_for_loops(
                &mut analyzed,
                func,
                impl_block.type_name.as_str(),
                program,
                registry,
            );
        }

        // Clear impl block after analysis
        self.self_impl_context = None;
        self.current_impl_functions = None;

        Ok(analyzed)
    }

    fn analyze_function(
        &mut self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        let mut inferred_ownership = HashMap::new();

        // Check if this is a game decorator function
        let is_game_decorator = func.decorators.iter().any(|d| {
            matches!(
                d.name.as_str(),
                "init" | "update" | "render" | "render3d" | "input" | "cleanup"
            )
        });
        let is_render3d = func.decorators.iter().any(|d| d.name == "render3d");

        // THE WINDJAMMER WAY: Auto-Self Inference
        // If a method uses `self` in its body but doesn't declare it as a parameter,
        // automatically infer and add it.
        let declares_self = func.parameters.iter().any(|p| p.name == "self");
        let uses_self = self.function_uses_identifier("self", &func.body);

        if uses_self && !declares_self {
            // Auto-infer self ownership based on usage
            let modifies_fields =
                self.function_modifies_self_fields_with_registry(func, Some(registry));
            let returns_self = self.function_returns_self(func);
            let body_moves_fields = self.function_body_moves_non_copy_self_fields(func);

            let self_ownership = if returns_self {
                // Builder pattern: mut self (owned)
                OwnershipMode::Owned
            } else if body_moves_fields {
                // Body moves non-Copy self fields (e.g., Foo { f: self.bindings }) → must own
                OwnershipMode::Owned
            } else if self.function_matches_on_self(func) {
                // TDD FIX for E0606: match self { ... } consumes self
                OwnershipMode::Owned
            } else if self.function_consumes_self_field_elements(func, Some(registry)) {
                // TDD FIX for E0507: for item in self.items { item.consume() }
                // Iterating over self.field and calling consuming methods requires owned self
                OwnershipMode::Owned
            } else if modifies_fields {
                // Mutating method: &mut self
                OwnershipMode::MutBorrowed
            } else if self.is_used_in_binary_op("self", &func.body) {
                // Copy types in operators: self (owned)
                OwnershipMode::Owned
            } else {
                // Read-only: &self
                OwnershipMode::Borrowed
            };

            // Store inferred self ownership
            inferred_ownership.insert("self".to_string(), self_ownership);
        }

        // Analyze each parameter to infer ownership mode
        for (i, param) in func.parameters.iter().enumerate() {
            let mode = match param.ownership {
                OwnershipHint::Owned => {
                    // DOGFOODING FIX #1: Respect explicit ownership annotations!
                    // If user writes `self` (not `&self` or `&mut self`), they want OWNED.
                    // Bug was: analyzer checked modifies_fields and downgraded to &mut self
                    // Fix: When Owned is explicit, use it - don't analyze or modify!
                    // Analysis should ONLY happen for OwnershipHint::Inferred.
                    OwnershipMode::Owned
                }
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Ref => {
                    // SMART FIX: If user wrote &self but function modifies fields, upgrade to &mut self
                    // This prevents a common user error
                    if param.name == "self"
                        && self.function_modifies_self_fields_with_registry(func, Some(registry))
                    {
                        OwnershipMode::MutBorrowed
                    } else {
                        OwnershipMode::Borrowed
                    }
                }
                OwnershipHint::Inferred => {
                    // Special case: Game decorator functions always take &mut for first parameter (game state)
                    if is_game_decorator && i == 0 {
                        OwnershipMode::MutBorrowed
                    } else if is_render3d && i == 2 {
                        // Special case: @render3d functions take &mut for camera parameter (3rd param)
                        OwnershipMode::MutBorrowed
                    } else if param.name == "self" {
                        // `extern impl Type { fn f(self) {} }` / empty extern bodies: the signature is an
                        // FFI stub; bare `self` in Windjammer means a receiver is passed — for inherent
                        // impl methods, treat as `&mut self` so `self.field.method()` is dispatchable
                        // without moving the struct (see ownership_self_field_mutation test).
                        if func.is_extern && func.body.is_empty() && func.parent_type.is_some() {
                            OwnershipMode::MutBorrowed
                        } else {
                            let modifies_fields = self
                                .function_modifies_self_fields_with_registry(func, Some(registry));
                            let returns_self = self.function_returns_self(func);
                            // WINDJAMMER FIX: Do NOT make self Owned for returning non-Copy fields.
                            // Getters like `fn id(self) -> String { self.id }` should be &self
                            // with the codegen auto-cloning the returned field. This prevents
                            // cascading E0382 at callsites.
                            let body_moves_fields =
                                self.function_body_moves_non_copy_self_fields(func);

                            if returns_self || body_moves_fields {
                                OwnershipMode::Owned
                            } else if self.function_moves_self_into_return(func) {
                                // self is moved into a struct literal or returned directly
                                OwnershipMode::Owned
                            } else if self.function_matches_on_self(func) {
                                // TDD FIX for E0606: match self { ... } consumes self
                                // Match expressions move the scrutinee value, requiring owned self
                                OwnershipMode::Owned
                            } else if self
                                .function_consumes_self_field_elements(func, Some(registry))
                            {
                                // TDD FIX for E0507: for item in self.items { item.consume() }
                                OwnershipMode::Owned
                            } else if modifies_fields {
                                // Mutating method that doesn't return self: use `&mut self`
                                OwnershipMode::MutBorrowed
                            } else {
                                // Check if self is used in binary operations (for Copy types like Vec2, Vec3)
                                // If self is used in operators (self.x * other.y, etc.), keep it owned
                                // This is especially important for math operations on Copy types
                                if self.is_used_in_binary_op("self", &func.body) {
                                    OwnershipMode::Owned
                                } else {
                                    // Default to borrowed for read-only methods
                                    // This is correct whether or not the method accesses self.fields
                                    // - If it accesses fields: &self works because we only read
                                    // - If it doesn't access self at all: &self is fine, no need to consume
                                    OwnershipMode::Borrowed
                                }
                            }
                        }
                    } else {
                        // For Copy types, check if they're mutated first
                        // Mutated Copy types should be &mut, not Owned
                        let is_copy = self.is_copy_type(&param.type_);

                        if is_copy {
                            if self.is_mutated(&param.name, &func.body, registry)
                                || matches!(
                                    self.infer_passthrough_ownership(
                                        &param.name,
                                        &param.type_,
                                        &func.body,
                                        registry,
                                        &func.name,
                                    ),
                                    Some(OwnershipMode::MutBorrowed)
                                )
                            {
                                OwnershipMode::MutBorrowed
                            } else {
                                OwnershipMode::Owned
                            }
                        } else {
                            // Perform inference based on usage in function body
                            let inferred_mode = self.infer_parameter_ownership(
                                &param.name,
                                &param.type_,
                                &func.body,
                                &func.return_type,
                                registry,
                                &func.name,
                            )?;

                            // DEBUG: Log ownership inference for non-Copy parameters
                            if std::env::var("WJ_DEBUG_OWNERSHIP").is_ok() {
                                eprintln!(
                                    "  [OWNERSHIP] {} in {}: {:?} (type: {:?})",
                                    param.name, func.name, inferred_mode, param.type_
                                );
                            }

                            inferred_mode
                        }
                    }
                }
            };

            inferred_ownership.insert(param.name.clone(), mode);
        }

        // PHASE 2 OPTIMIZATION: Detect unnecessary clones
        let clone_optimizations = self.detect_unnecessary_clones(func);

        // PHASE 3 OPTIMIZATION: Detect struct mapping opportunities
        let struct_mapping_optimizations = self.detect_struct_mappings(func);

        // PHASE 4 OPTIMIZATION: Detect string operation opportunities
        let string_optimizations = self.detect_string_optimizations(func);

        // PHASE 5: Detect assignment operations that can use compound operators
        let assignment_optimizations = self.detect_assignment_optimizations(func);
        let defer_drop_optimizations = self.detect_defer_drop_opportunities(func, registry);

        // AUTO-CLONE: Analyze where clones should be automatically inserted
        let auto_clone_analysis = AutoCloneAnalysis::analyze_function(func);

        // AUTO-MUT: Track which local variables are mutated (for automatic mut inference)
        self.track_mutations(&func.body, registry);
        let mutated_variables = self.mutated_variables.clone();

        // LINTER: Track which parameters are mutated (for owned-but-not-returned lint)
        let mut mutated_parameters = HashSet::new();
        for param in &func.parameters {
            if self.is_mutated(&param.name, &func.body, registry) {
                mutated_parameters.insert(param.name.clone());
            }
        }

        // PHASE 7-9: Additional optimizations (future implementation)
        let const_static_optimizations = Vec::new(); // TODO: Implement detection
        let smallvec_optimizations = Vec::new(); // TODO: Implement detection
        let cow_optimizations = Vec::new(); // TODO: Implement detection

        // PHASE 2: Analyze which string parameters can use &str optimization
        let str_ref_optimizable_params = self.analyze_str_ref_optimizable_params(func, registry);

        // Build inferred parameter types based on Phase 2 analysis
        let inferred_param_types: Vec<Type> = func
            .parameters
            .iter()
            .map(|param| {
                // Check if this parameter can be optimized to &str (instead of &String)
                let can_use_str_ref = str_ref_optimizable_params.contains(&param.name);

                if can_use_str_ref {
                    // Optimize to &str (not &String)
                    Type::Reference(Box::new(Type::Custom("str".to_string())))
                } else {
                    // Keep original type (will become &String for string params)
                    param.type_.clone()
                }
            })
            .collect();

        Ok(AnalyzedFunction {
            decl: func.clone(),
            inferred_ownership,
            inferred_param_types,
            mutated_variables,
            mutated_parameters,
            auto_clone_analysis,
            clone_optimizations,
            struct_mapping_optimizations,
            string_optimizations,
            assignment_optimizations,
            defer_drop_optimizations,
            const_static_optimizations,
            smallvec_optimizations,
            cow_optimizations,
            str_ref_optimizable_params,
        })
    }

    /// Analyze a function that implements a trait method
    /// Use the trait's method signature instead of inferring
    fn analyze_trait_impl_function(
        &mut self,
        func: &FunctionDecl<'ast>,
        trait_name: &str,
        impl_block: &crate::parser::ast::ImplBlock<'ast>,
        program: &Program<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        // Trait impl bodies may call `self.inherent_helper()` from `impl Type` — merge those decls.
        self.current_impl_functions = Some(Self::merged_impl_methods_for_type(
            program,
            &impl_block.type_name,
        ));
        let impl_base = impl_block
            .type_name
            .split('<')
            .next()
            .unwrap_or(impl_block.type_name.as_str())
            .to_string();
        self.self_impl_context = Some(ImplSelfFieldContext::new(impl_base, program));
        let analyzed_base = self.analyze_function(func, registry);
        self.self_impl_context = None;
        self.current_impl_functions = None;
        let mut analyzed = analyzed_base?;

        // Look up the trait definition
        // Try both the full trait name and just the last segment (e.g., "std::ops::Add" -> "Add")
        let trait_key = if let Some(pos) = trait_name.rfind("::") {
            &trait_name[pos + 2..]
        } else {
            trait_name
        };

        let is_std_operator_trait =
            crate::type_classification::is_consuming_operator_trait(trait_key);

        // For standard operator traits, use `self` (owned) instead of `&self`
        if is_std_operator_trait {
            // Standard operator trait (Add, Sub, Mul, etc.) - not defined in Windjammer stdlib
            // These traits use `self` (owned) for the first parameter (self), not `&self`
            // Example: `fn add(self, rhs: Rhs) -> Output`

            // For the first parameter (self), use Owned for Copy types
            if let Some(first_param) = func.parameters.first() {
                if first_param.name == "self" {
                    // Use Owned (self) for operator traits on Copy types
                    analyzed
                        .inferred_ownership
                        .insert("self".to_string(), OwnershipMode::Owned);
                }
            }
        } else if let Some(trait_decl) = self.trait_definitions.get(trait_key) {
            // Find the matching trait method
            if let Some(trait_method) = trait_decl.methods.iter().find(|m| m.name == func.name) {
                // Override ALL parameters to match trait signature
                // Trait implementations must match the trait's exact signature
                // Match by POSITION, not by name (trait uses "rhs", impl might use "other")
                for (i, trait_param) in trait_method.parameters.iter().enumerate() {
                    // Get the corresponding parameter from the implementation by position
                    if let Some(impl_param) = func.parameters.get(i) {
                        // WINDJAMMER PHILOSOPHY: Use ANALYZED trait method ownership, not AST ownership!
                        // The AST might have `self` (Owned) but analysis infers `&self` (Borrowed).
                        // Check if this trait method was analyzed (has default implementation)
                        let trait_methods_opt = self
                            .analyzed_trait_methods
                            .get(trait_key)
                            .or_else(|| self.analyzed_trait_methods.get(trait_name));
                        let trait_mode = if let Some(trait_methods) = trait_methods_opt {
                            if let Some(analyzed_trait_method) = trait_methods.get(&func.name) {
                                analyzed_trait_method
                                    .inferred_ownership
                                    .get(&trait_param.name)
                                    .copied()
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // When the trait has concrete ownership data (from default
                        // impl or from a previous impl upgrade), the impl must match.
                        // When the trait has NO data (abstract method, never analyzed),
                        // the impl's own body analysis drives ownership, and the trait
                        // entry will be upgraded in the post-analysis merge step.
                        let final_mode = if let Some(mode) = trait_mode {
                            mode
                        } else {
                            analyzed
                                .inferred_ownership
                                .get(&impl_param.name)
                                .copied()
                                .unwrap_or_else(|| {
                                    self.convert_ownership_hint_to_mode(
                                        &trait_param.ownership,
                                        &trait_param.name,
                                    )
                                })
                        };

                        // INSERT or UPDATE with the final ownership mode
                        analyzed
                            .inferred_ownership
                            .insert(impl_param.name.clone(), final_mode);
                    }
                }
            }
        }

        // E0186 / E0053: Impl receiver must match the trait — never infer only from the impl body.
        // Replacing analyzed_trait_methods with the impl used to drop implicit `self` when the impl
        // body was empty or omitted `self`, producing trait/impl mismatches in Rust.
        if let Some(self_mode) =
            self.trait_method_receiver_ownership(trait_name, trait_key, &func.name)
        {
            analyzed
                .inferred_ownership
                .insert("self".to_string(), self_mode);
        }

        // E0053: Parameter types in generated Rust must match the trait declaration (impls may
        // rename parameters or use incompatible aliases). Ownership already matches the trait above.
        if !is_std_operator_trait {
            if let Some(analyzed_trait_fn) = self
                .analyzed_trait_methods
                .get(trait_key)
                .and_then(|m| m.get(&func.name))
                .or_else(|| {
                    self.analyzed_trait_methods
                        .get(trait_name)
                        .and_then(|m| m.get(&func.name))
                })
            {
                for (i, _) in func.parameters.iter().enumerate() {
                    if let Some(trait_ty) = analyzed_trait_fn.inferred_param_types.get(i) {
                        if i < analyzed.inferred_param_types.len() {
                            analyzed.inferred_param_types[i] = trait_ty.clone();
                        }
                    }
                }
            }
            // If multipass never stored analyzed trait fn types, still copy AST parameter types so
            // generated Rust matches the trait item (E0053).
            if let Some(trait_decl) = self.trait_definitions.get(trait_key) {
                if let Some(trait_method) = trait_decl.methods.iter().find(|m| m.name == func.name)
                {
                    let tf = self
                        .analyzed_trait_methods
                        .get(trait_key)
                        .and_then(|m| m.get(&func.name))
                        .or_else(|| {
                            self.analyzed_trait_methods
                                .get(trait_name)
                                .and_then(|m| m.get(&func.name))
                        });
                    for (i, trait_param) in trait_method.parameters.iter().enumerate() {
                        if i >= analyzed.inferred_param_types.len() {
                            break;
                        }
                        let use_ast = tf.and_then(|t| t.inferred_param_types.get(i)).is_none();
                        if use_ast {
                            analyzed.inferred_param_types[i] = trait_param.type_.clone();
                        }
                    }
                }
            }
        }

        Ok(analyzed)
    }

    /// Look up the analyzed receiver (`self`) ownership for a trait method (trait is the contract).
    fn trait_method_receiver_ownership(
        &self,
        trait_name: &str,
        trait_key: &str,
        method_name: &str,
    ) -> Option<OwnershipMode> {
        for key in [trait_key, trait_name] {
            if let Some(methods) = self.analyzed_trait_methods.get(key) {
                if let Some(trait_fn) = methods.get(method_name) {
                    return trait_fn.inferred_ownership.get("self").copied();
                }
            }
        }
        None
    }

    fn infer_parameter_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
        return_type: &Option<Type>,
        registry: &SignatureRegistry,
        current_func_name: &str,
    ) -> Result<OwnershipMode, String> {
        // 0a. Generic type parameters and impl Trait always stay Owned.
        // Adding & would change trait bounds: `impl Foo` -> `&impl Foo` breaks dispatch.
        // Generic types like T, G, S should always be passed by value.
        if Self::is_generic_type_param(param_type) {
            return Ok(OwnershipMode::Owned);
        }

        // 0b. Explicit Rust type `String` (not Windjammer `string`) stays Owned.
        // When user writes `path: String`, they're explicitly requesting Rust's String type,
        // which should be respected as owned. Do NOT infer it as borrowed.
        // This is different from `path: string` (lowercase), which can infer to &str.
        if matches!(param_type, Type::Custom(name) if name == "String") {
            return Ok(OwnershipMode::Owned);
        }

        // 0c. Return-type-aware ownership: When return type contains param type, we need Owned.
        // Bug: save_migration.wj - migrate(data) -> Result<GameSaveData, string> was inferring
        // &GameSaveData because we only read data fields. But we assign to current_data and
        // return that - we need to own the input to produce the output.
        // Handles: fn(T) -> T, fn(T) -> Result<T,E>, fn(T) -> Option<T>
        //
        // TDD FIX: Skip when param is ONLY used as &param (e.g., a + &b + &c).
        // For concatenate(a, b, c) -> a + &b + &c, b and c are borrowed, not consumed.
        // param_type_matches_return would incorrectly infer Owned for all string params.
        if !self.is_only_used_as_borrow(param_name, body) {
            if let Some(return_type) = return_type {
                if self.param_type_matches_return(param_type, return_type) {
                    // Windjammer `string` / `str` parameters: return type also being string-like
                    // does NOT mean the parameter is consumed into the return value.
                    // Example: find_translation(lang, key: string) -> string only compares `key`;
                    // inferring Owned for `key` breaks callers that pass the same String twice (E0382).
                    // Non-string types keep the broader rule (transform/migrate still get Owned).
                    let string_like = matches!(param_type, Type::String);
                    if self.is_returned(param_name, body)
                        || self.is_stored(param_name, body)
                    {
                        return Ok(OwnershipMode::Owned);
                    } else if !string_like
                        && self.param_is_consumed_into_return(param_name, body)
                    {
                        return Ok(OwnershipMode::Owned);
                    }
                }
            }
        }

        // WINDJAMMER DESIGN: String parameters infer to &str (not &String!)
        //
        // When a string parameter is read-only, we generate `&str` (idiomatic Rust):
        // - Accepts both `String` and `&str` via deref coercion
        // - No `&String` anti-pattern (Clippy-approved)
        // - Zero-cost for read-only access
        //
        // Strings are treated like any other type in ownership inference.
        // The codegen layer will emit `&str` for Borrowed String parameters.

        // Multi-pass registry-aware inference

        // 1. Check if parameter is mutated (uses registry for method call detection)
        if self.is_mutated(param_name, body, registry) {
            return Ok(OwnershipMode::MutBorrowed);
        }

        // 2. Check if parameter is returned (escapes function)
        if self.is_returned(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 2.1. Returning a non-Copy self field (e.g. `fn id(self) -> String { self.id }`)
        // WINDJAMMER FIX: Do NOT make self Owned just because a non-Copy field is returned.
        // Instead, keep self as Borrowed (&self) and the codegen will auto-clone the
        // returned field. This prevents cascading E0382 errors at callsites where the
        // caller uses the object again after calling a getter.
        // The old behavior (Owned self for getters) was correct Rust semantics but bad
        // Windjammer ergonomics -- every caller had to .clone() the object before calling
        // a simple getter.

        // 2.3. WINDJAMMER FIX: Check if parameter is used in if/else expression
        // When a parameter appears in an if/else that's assigned or returned,
        // it needs to be owned to match the other branch's ownership
        if self.is_used_in_if_else_expression(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // THE WINDJAMMER WAY: Removed aggressive string optimization
        // When a user writes `text: string`, they mean `String` (owned), period.
        // Do NOT auto-convert to `&str` just because it's "only passed to read-only functions".
        // That breaks API contracts and causes confusing type errors.
        //
        // Smart inference is OFF for explicit type annotations.
        // (Future: Could add #[optimize] annotation for user-requested optimization)

        // THE WINDJAMMER WAY: Removed aggressive string optimization
        // When a user writes `text: string`, they mean `String` (owned), period.
        // Do NOT auto-convert to `&str` just because it's "only passed to read-only functions".
        // That breaks API contracts and causes confusing type errors.
        //
        // Smart inference is OFF for explicit type annotations.
        // (Future: Could add #[optimize] annotation for user-requested optimization)

        // 3. Check if parameter is stored in a struct or collection
        if self.is_stored(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 4. Check if parameter is used in arithmetic binary operations (for Copy types)
        // TDD FIX (Bug #5): Comparison operators (==, !=, <, >, <=, >=) work with borrowed
        // values, so we should only force Owned for arithmetic operations (Add, Sub, Mul, Div).
        if self.is_used_in_arithmetic_op(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 5. Check if parameter is pattern matched with field extraction
        if self.is_pattern_matched_with_fields(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 6. TDD: Check if parameter is iterated over in a for loop
        if self.is_iterated_over(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 7. MULTI-PASS OWNERSHIP INFERENCE (The Proper Solution!)
        // Check if parameter is passed as an argument to another function/method.
        // Look up the callee's signature in the registry and match the ownership mode.
        //
        // THE WINDJAMMER WAY: The compiler does the work, not the user.
        // - Pass 1: No registry yet → infer from local usage (comparisons → Borrowed)
        // - Pass 2+: Look up callee signatures → match their ownership
        // - Convergence: Ownership propagates until stable
        //
        // Example: fn wrapper(id: string) { has_item(id) }
        // - Pass 1: has_item doesn't exist, id only used in pass-through → Borrowed
        // - Pass 2: has_item exists with id: &String → wrapper matches Borrowed
        // - Pass 3: No changes → CONVERGED ✅
        //
        // IMPORTANT: Only use registry if callees expect stricter ownership than local usage
        if let Some(pass_through_mode) = self.infer_passthrough_ownership(
            param_name,
            param_type,
            body,
            registry,
            current_func_name,
        ) {
            match pass_through_mode {
                OwnershipMode::Borrowed => return Ok(OwnershipMode::Borrowed),
                OwnershipMode::MutBorrowed => return Ok(OwnershipMode::MutBorrowed),
                OwnershipMode::Owned => {
                    return Ok(OwnershipMode::Owned);
                }
            }
        }

        // 8. Default ownership: Borrowed (THE WINDJAMMER WAY!)
        //
        // **PHILOSOPHY**: The compiler does the work, not the user.
        // - Default to **Borrowed** for read-only parameters
        // - The checks above handle all consuming cases (mutated, returned,
        //   stored, iterated, used in binary ops, pattern matched)
        // - If none of those apply, the parameter is truly read-only
        // - Read-only non-Copy parameters should be &T in generated Rust
        // - Copy types are overridden to Owned in build_signature
        //
        // This matches the Windjammer philosophy: users write `data: Vec<f32>`
        // and the compiler infers `&Vec<f32>` when data is only read.
        // Call sites naturally pass `&self.data` which matches `&Vec<f32>`.
        //
        // Dogfooding evidence: 6+ E0308 errors in windjammer-game-editor
        // from read-only params generating owned types while call sites pass &T.
        Ok(OwnershipMode::Borrowed)
    }

    /// TDD: Check if parameter is ONLY used as &param or &mut param (never consumed directly).
    /// Example: a + &b + &c - b and c are only used as &b, &c → true for b and c.
    /// Used to avoid param_type_matches_return incorrectly inferring Owned for string params
    /// in concatenation: fn(a, b, c) -> a + &b + &c.
    fn is_only_used_as_borrow(&self, param_name: &str, body: &[&'ast Statement<'ast>]) -> bool {
        for stmt in body {
            if !self.stmt_param_only_borrowed(param_name, stmt, false) {
                return false;
            }
        }
        true
    }

    fn stmt_param_only_borrowed(
        &self,
        param_name: &str,
        stmt: &Statement,
        _inside_ref: bool,
    ) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_param_only_borrowed(param_name, value, false),
            Statement::Return { value, .. } => value
                .as_ref()
                .is_none_or(|e| self.expr_param_only_borrowed(param_name, e, false)),
            Statement::Expression { expr, .. } => {
                self.expr_param_only_borrowed(param_name, expr, false)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_param_only_borrowed(param_name, condition, false)
                    && then_block
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
                    && else_block.as_ref().is_none_or(|b| {
                        b.iter()
                            .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_param_only_borrowed(param_name, condition, false)
                    && body
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
            }
            Statement::For { iterable, body, .. } => {
                self.expr_param_only_borrowed(param_name, iterable, false)
                    && body
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
            }
            _ => true,
        }
    }

    fn expr_param_only_borrowed(
        &self,
        param_name: &str,
        expr: &Expression,
        inside_ref: bool,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => {
                // Found param: must be inside & or &mut to be "only borrowed"
                inside_ref
            }
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => self.expr_param_only_borrowed(param_name, operand, true),
            Expression::Binary { left, right, .. } => {
                self.expr_param_only_borrowed(param_name, left, false)
                    && self.expr_param_only_borrowed(param_name, right, false)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_param_only_borrowed(param_name, function, false)
                    && arguments
                        .iter()
                        .all(|(_, a)| self.expr_param_only_borrowed(param_name, a, false))
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_param_only_borrowed(param_name, object, false)
                    && arguments
                        .iter()
                        .all(|(_, a)| self.expr_param_only_borrowed(param_name, a, false))
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_param_only_borrowed(param_name, object, false)
            }
            Expression::Index { object, index, .. } => {
                self.expr_param_only_borrowed(param_name, object, false)
                    && self.expr_param_only_borrowed(param_name, index, false)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .all(|s| self.stmt_param_only_borrowed(param_name, s, false)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .all(|e| self.expr_param_only_borrowed(param_name, e, false)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .all(|(_, v)| self.expr_param_only_borrowed(param_name, v, false)),
            Expression::TryOp { expr, .. } => {
                self.expr_param_only_borrowed(param_name, expr, false)
            }
            _ => true,
        }
    }

    fn is_used_in_if_else_expression(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        // Check if parameter is used in an if/else expression
        // Example:
        //   let x = if cond { Thing::new(...) } else { param }
        //
        // When param is in an if/else that gets assigned or returned,
        // it needs to be owned to match the other branch's type

        for stmt in statements {
            if self.stmt_has_if_else_with_param(name, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_has_if_else_with_param(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let { value, .. } => {
                // let x = if ... { ... } else { param }
                self.expr_is_if_else_with_param(name, value)
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                // return if ... { ... } else { param }
                self.expr_is_if_else_with_param(name, expr)
            }
            Statement::Expression { expr, .. } => {
                // Implicit return or assignment
                self.expr_is_if_else_with_param(name, expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                // Check nested if statements
                self.stmts_have_if_else_with_param(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| self.stmts_have_if_else_with_param(name, block))
            }
            _ => false,
        }
    }

    fn stmts_have_if_else_with_param(&self, name: &str, stmts: &[&'ast Statement<'ast>]) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_has_if_else_with_param(name, stmt))
    }

    fn expr_is_if_else_with_param(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                // Check if block contains an if statement with the parameter
                for stmt in statements {
                    if let Statement::If {
                        then_block,
                        else_block,
                        ..
                    } = stmt
                    {
                        let in_then = self.stmts_mention_identifier(name, then_block);
                        let in_else = else_block
                            .as_ref()
                            .is_some_and(|block| self.stmts_mention_identifier(name, block));
                        if in_then || in_else {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn stmts_mention_identifier(&self, name: &str, stmts: &[&'ast Statement<'ast>]) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_mentions_identifier(name, stmt))
    }

    fn stmt_mentions_identifier(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expr_mentions_identifier(name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_mentions_identifier(name, expr),
            Statement::Let { value, .. } => self.expr_mentions_identifier(name, value),
            _ => false,
        }
    }

    fn expr_mentions_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expr_mentions_identifier(name, object),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_mentions_identifier(name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_mentions_identifier(name, arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_identifier(name, left)
                    || self.expr_mentions_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_mentions_identifier(name, operand),
            Expression::TryOp { expr, .. } => self.expr_mentions_identifier(name, expr),
            _ => false,
        }
    }
    fn is_returned(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        let len = statements.len();
        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == len - 1;
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Check if parameter is returned directly or wrapped in Some/Ok/Err/tuple
                    if self.expression_uses_identifier_for_return(name, expr) {
                        return true;
                    }
                }
                // CRITICAL: Handle implicit returns (last expression without semicolon)
                // In Windjammer/Rust, the last expression in a block is the return value
                Statement::Expression { expr, .. } if is_last => {
                    // Skip ONLY void-returning function calls (like println)
                    // Wrapper calls (Some, Ok, Err) DO return their arguments!
                    let is_void_call = if let Expression::Call { function, .. } = expr {
                        if let Expression::Identifier { name: fn_name, .. } = &**function {
                            matches!(
                                fn_name.as_str(),
                                "println" | "print" | "eprintln" | "eprint" | "assert" | "panic"
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !is_void_call && self.expression_uses_identifier_for_return(name, expr) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_returned(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_returned(name, else_b) {
                            return true;
                        }
                    }
                }
                // CRITICAL: Handle match expressions where parameter is returned in arms
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if self.expression_uses_identifier_for_return(name, arm.body) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if an expression uses a parameter in a way that requires ownership for return.
    /// This includes direct use, wrapping in Some/Ok/Err, tuples, etc.
    fn expression_uses_identifier_for_return(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            // Direct identifier use
            Expression::Identifier { name: id, .. } if id == name => true,

            // Wrapped in constructors: Some(param), Ok(param), Err(param), Enum::Variant(param)
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    let is_known_wrapper = matches!(fn_name.as_str(), "Some" | "Ok" | "Err");
                    let is_enum_constructor = Self::looks_like_enum_variant_constructor(fn_name);

                    if is_known_wrapper || is_enum_constructor {
                        for (_label, arg) in arguments {
                            if self.expression_uses_identifier(name, arg) {
                                return true;
                            }
                        }
                    }
                }
                false
            }

            // Tuple expression: (a, b, c)
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    if self.expression_uses_identifier(name, elem) {
                        return true;
                    }
                }
                false
            }

            // CRITICAL FIX: Binary expressions (comparisons, arithmetic) return the RESULT, not the parameter
            // Example: `id == "test"` returns bool, NOT id
            // Example: `id + 1` returns the sum, NOT id
            // The parameter is only being READ, not returned
            Expression::Binary { .. } => false,

            // Unary expressions also return the result, not the operand
            Expression::Unary { .. } => false,

            // Default: reject (conservative - only allow explicit cases above)
            _ => false,
        }
    }

    /// Check if an expression stores a parameter by value.
    /// Matches direct identifier use, wrapping in Some/Ok/Err, enum variant constructors,
    /// tuples, and struct literals containing the parameter.
    fn expression_stores_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    let is_constructor =
                        matches!(fn_name.as_str(), "Some" | "Ok" | "Err") || fn_name.contains("::");
                    if is_constructor {
                        return arguments
                            .iter()
                            .any(|(_label, arg)| self.expression_stores_identifier(name, arg));
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier(name, el)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_stores_identifier(name, v)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier(name, el)),
            _ => false,
        }
    }

    fn param_is_consumed_into_return(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in body {
            match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(var_name),
                    value,
                    ..
                } => {
                    if self.expression_uses_identifier(param_name, value) {
                        if self.is_returned(var_name, body) {
                            return true;
                        }
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expression_uses_identifier(param_name, value) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.param_is_consumed_into_return(param_name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.param_is_consumed_into_return(param_name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            if self.param_is_consumed_into_return(param_name, statements) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn is_stored(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        // Check if the parameter is stored in a struct field or collection
        for stmt in statements {
            match stmt {
                Statement::Let {
                    value: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    for (_field_name, field_expr) in fields {
                        if self.expression_uses_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Return {
                    value: Some(Expression::StructLiteral { fields, .. }),
                    ..
                } => {
                    // Check if parameter is used in a returned struct literal
                    for (_, field_expr) in fields {
                        if self.expression_uses_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Expression {
                    expr: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    // Check if parameter is used in a struct literal expression (implicit return)
                    for (_, field_expr) in fields {
                        if self.expression_uses_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Assignment {
                    target: Expression::FieldAccess { object, .. },
                    value,
                    ..
                } => {
                    // Check if the parameter is assigned to a struct field, either directly
                    // or wrapped in Some/Enum constructors/tuples.
                    //
                    // Direct: obj.field = param
                    // Wrapped: obj.field = Some(param)
                    // Enum: obj.field = Enum::Variant(param)
                    if matches!(&**object, Expression::Identifier { .. }) {
                        if self.expression_stores_identifier(name, value) {
                            return true;
                        }
                    }
                }
                // Check if parameter is stored via index assignment
                // e.g., self.slots[i] = item
                // e.g., self.slots[i] = Some(ItemStack::new(item, qty))
                Statement::Assignment {
                    target: Expression::Index { .. },
                    value,
                    ..
                } => {
                    if self.expression_stores_identifier(name, value) {
                        return true;
                    }
                }
                Statement::Expression {
                    expr:
                        Expression::MethodCall {
                            object,
                            method,
                            arguments,
                            ..
                        },
                    ..
                } => {
                    let is_storage_method = crate::method_registry::is_storage_method(method);

                    if is_storage_method {
                        // Check for storage method calls on ANY object:
                        // - self.field.push(param)
                        // - self.field.push((param, other))  ← tuple wrapping
                        // - self.field.push(Enum::Variant(param))  ← enum wrapping
                        // - local_var.push(param)
                        let is_on_field_or_var =
                            matches!(&**object, Expression::FieldAccess { .. })
                                || matches!(&**object, Expression::Identifier { .. });

                        if is_on_field_or_var {
                            for (_label, arg) in arguments {
                                if self.expression_stores_identifier(name, arg) {
                                    return true;
                                }
                            }
                        }

                        // TDD FIX: Also check for method calls on LOCAL struct fields: local_var.field.push(param)
                        // e.g., choice.conditions.push(condition) where choice is a local variable
                        if let Expression::FieldAccess {
                            object: field_obj, ..
                        } = &**object
                        {
                            // Check if it's a local variable (not self)
                            if matches!(&**field_obj, Expression::Identifier { name: id, .. } if id != "self")
                            {
                                for (_label, arg) in arguments {
                                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }

                    // Also check for method calls on local variables: props.push(Property { name, ... })
                    // The parameter might be used in a struct literal passed as an argument
                    for (_label, arg) in arguments {
                        if let Expression::StructLiteral { fields, .. } = arg {
                            for (_field_name, field_expr) in fields {
                                if self.expression_uses_identifier(name, field_expr) {
                                    return true;
                                }
                            }
                        }
                    }

                    // Check for push/insert with a constructor call: vec.push(Node::new(param, ...))
                    // The parameter is being stored if passed to a constructor that stores it
                    if is_storage_method {
                        for (_label, arg) in arguments {
                            if let Expression::Call {
                                arguments: call_args,
                                ..
                            } = arg
                            {
                                for (_call_label, call_arg) in call_args {
                                    if matches!(call_arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                // Recursively check if/else bodies for storage operations
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_stored(name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_stored(name, else_stmts) {
                            return true;
                        }
                    }
                }
                // Recursively check loop bodies
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    if self.is_stored(name, body) {
                        return true;
                    }
                }
                // General case: check any statement for enum variant constructors
                // that consume the parameter. Covers patterns like:
                //   let x = Func(EnumType::Variant(param, ...))
                //   let x = Func(format!(..., param), &EnumType::Variant(param, ...))
                _ => {
                    if self.stmt_has_enum_variant_consuming(name, stmt) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a statement contains an enum variant constructor that consumes a parameter.
    /// Recursively scans all expressions within the statement.
    fn stmt_has_enum_variant_consuming(&self, name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_has_enum_variant_consuming(name, value),
            Statement::Expression { expr, .. } => self.expr_has_enum_variant_consuming(name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_has_enum_variant_consuming(name, expr),
            Statement::Assignment { value, .. } => {
                self.expr_has_enum_variant_consuming(name, value)
            }
            _ => false,
        }
    }

    /// Recursively check if an expression contains an enum variant constructor
    /// (function call where name contains "::") that has the parameter as a direct argument.
    fn expr_has_enum_variant_consuming(&self, name: &str, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let is_enum_variant = if let Expression::Identifier { name: fn_name, .. } = function
                {
                    Self::looks_like_enum_variant_constructor(fn_name)
                } else if let Expression::FieldAccess { field, .. } = function {
                    Self::looks_like_enum_variant_constructor(field)
                } else {
                    false
                };

                if is_enum_variant {
                    for (_label, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
                    }
                }

                // Recurse into all arguments
                for (_label, arg) in arguments {
                    if self.expr_has_enum_variant_consuming(name, arg) {
                        return true;
                    }
                }
                // Recurse into function expression
                self.expr_has_enum_variant_consuming(name, function)
            }
            Expression::Unary { operand, .. } => {
                self.expr_has_enum_variant_consuming(name, operand)
            }
            Expression::Block { statements, .. } => {
                for s in statements {
                    if self.stmt_has_enum_variant_consuming(name, s) {
                        return true;
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => {
                for el in elements {
                    if self.expr_has_enum_variant_consuming(name, el) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if a qualified name like "Type::Variant" looks like an enum variant constructor
    /// rather than a static method call. Enum variants use PascalCase after "::"
    /// (e.g., Option::Some, Color::Custom), while methods use snake_case
    /// (e.g., FpsCamera::collides_aabb, Vec3::new).
    fn looks_like_enum_variant_constructor(qualified_name: &str) -> bool {
        if let Some(pos) = qualified_name.rfind("::") {
            let after_colons = &qualified_name[pos + 2..];
            after_colons
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        } else {
            false
        }
    }

    /// TDD: Check if a parameter is iterated over in a for loop (consumed by iteration)
    /// e.g., `for item in items` (not `for item in &items`)
    /// When you iterate over a Vec without `&`, the Vec is consumed and elements are moved.
    fn is_iterated_over(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::For { iterable, body, .. } => {
                    // Check if the iterable is exactly the parameter (direct iteration)
                    if let Expression::Identifier { name: id, .. } = iterable {
                        if id == name {
                            return true;
                        }
                    }

                    // Recursively check nested for loops
                    if self.is_iterated_over(name, body) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_iterated_over(name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_iterated_over(name, else_stmts) {
                            return true;
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if self.is_iterated_over(name, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a parameter is passed as a direct (non-&) argument to a function or method call.
    /// When a parameter is passed directly (not via &) to another function, it could be consumed
    /// (the callee may take ownership). Without knowing the callee's signature, we conservatively
    /// assume consumption and keep the parameter Owned.
    ///
    /// Examples that trigger Owned:
    /// - `Quest::new(id, title, description)` — id is a direct argument
    /// - `process(data)` — data is a direct argument
    ///
    /// Examples that do NOT trigger Owned:
    /// - `data.len()` — data is the receiver, not an argument
    /// - `process(&data)` — & wraps the argument, so it's borrowed
    /// - `format!("{}", data)` — macro call, not a function call in the AST
    #[allow(dead_code)]
    fn is_passed_as_argument(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            if self.stmt_passes_as_argument(name, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_passes_as_argument(&self, name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_passes_as_argument(name, value),
            Statement::Expression { expr, .. } => self.expr_passes_as_argument(name, expr),
            Statement::Return { value: Some(v), .. } => self.expr_passes_as_argument(name, v),
            Statement::Assignment { value, .. } => self.expr_passes_as_argument(name, value),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_passes_as_argument(name, condition)
                    || self.is_passed_as_argument(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.is_passed_as_argument(name, b))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_passes_as_argument(name, condition)
                    || self.is_passed_as_argument(name, body)
            }
            Statement::Loop { body, .. } => self.is_passed_as_argument(name, body),
            Statement::For { body, .. } => self.is_passed_as_argument(name, body),
            Statement::Match { value, arms, .. } => {
                self.expr_passes_as_argument(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expr_passes_as_argument(name, arm.body))
            }
            _ => false,
        }
    }

    fn expr_passes_as_argument(&self, name: &str, expr: &Expression<'ast>) -> bool {
        match expr {
            // Function call: check if parameter is a bare argument (not wrapped in &)
            Expression::Call { arguments, .. } => {
                // TDD FIX: Don't force Owned for simple pass-through!
                // If a parameter is ONLY passed to another function with no other operations,
                // it might be a pass-through and can stay Borrowed.
                //
                // CONSERVATIVE APPROACH: Still return true (force Owned) because without
                // the callee's signature (which doesn't exist during analysis), we can't
                // know if the callee consumes the value or just borrows it.
                //
                // FUTURE: Multi-pass analysis could solve this:
                // - Pass 1: Conservative inference
                // - Pass 2: Re-infer using SignatureRegistry from Pass 1
                // - Iterate until stable
                for (_label, arg) in arguments {
                    // Direct identifier: `f(param)` → potentially consuming
                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                    // Recursively check sub-expressions for nested calls
                    if self.expr_passes_as_argument(name, arg) {
                        return true;
                    }
                }
                false
            }
            // Method call: check arguments (NOT the receiver)
            Expression::MethodCall {
                object, arguments, ..
            } => {
                for (_label, arg) in arguments {
                    // Direct identifier: `obj.method(param)` → consuming
                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                    // Recursively check sub-expressions
                    if self.expr_passes_as_argument(name, arg) {
                        return true;
                    }
                }
                // Also check the receiver for nested calls (but NOT as a direct argument)
                self.expr_passes_as_argument(name, object)
            }
            // Block expression: check all statements
            Expression::Block { statements, .. } => self.is_passed_as_argument(name, statements),
            // Binary, unary, index, etc.: recurse into sub-expressions
            Expression::Binary { left, right, .. } => {
                self.expr_passes_as_argument(name, left)
                    || self.expr_passes_as_argument(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_passes_as_argument(name, operand),
            Expression::Index { object, index, .. } => {
                self.expr_passes_as_argument(name, object)
                    || self.expr_passes_as_argument(name, index)
            }
            Expression::FieldAccess { object, .. } => self.expr_passes_as_argument(name, object),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| self.expr_passes_as_argument(name, e)),
            Expression::Closure { body, .. } => self.expr_passes_as_argument(name, body),
            // TDD FIX: TryOp wraps expressions with `?` (error propagation).
            // e.g., `process(data)?` produces TryOp { expr: Call { args: [data] } }
            // We must recurse into the inner expression to detect argument passing.
            Expression::TryOp { expr, .. } => self.expr_passes_as_argument(name, expr),
            // Note: We do NOT check Expression::Identifier here because bare identifiers
            // outside of Call/MethodCall arguments are not consuming (e.g., `data.len()`)
            _ => false,
        }
    }

    // TDD FIX (Bug #5): New function to check ONLY arithmetic operations, not comparisons
    fn is_used_in_arithmetic_op(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, value) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_arithmetic_op(name, then_block) {
                        return true;
                    }
                    if let Some(else_block) = else_block {
                        if self.is_used_in_arithmetic_op(name, else_block) {
                            return true;
                        }
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_arithmetic_op(name, body) {
                        return true;
                    }
                }
                Statement::For { body, .. } => {
                    if self.is_used_in_arithmetic_op(name, body) {
                        return true;
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expr_uses_in_arithmetic_op(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary {
                op, left, right, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                // Only check for arithmetic operators, not comparisons
                let is_arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                );

                if is_arithmetic {
                    if self.expr_is_identifier(left, name) || self.expr_is_identifier(right, name) {
                        return true;
                    }
                }
                // Recursively check nested expressions
                self.expr_uses_in_arithmetic_op(name, left)
                    || self.expr_uses_in_arithmetic_op(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_uses_in_arithmetic_op(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_uses_in_arithmetic_op(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_uses_in_arithmetic_op(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_uses_in_arithmetic_op(name, arg))
            }
            Expression::FieldAccess { .. } => false,
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_arithmetic_op(name, object)
                    || self.expr_uses_in_arithmetic_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_arithmetic_op(name, statements),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_arithmetic_op(name, elem)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_arithmetic_op(name, elem)),
            Expression::TryOp { expr, .. } => self.expr_uses_in_arithmetic_op(name, expr),
            _ => false,
        }
    }

    fn is_used_in_binary_op(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expr_uses_in_binary_op(name, value) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expr_uses_in_binary_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_uses_in_binary_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_uses_in_binary_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_binary_op(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_used_in_binary_op(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. }
                | Statement::While { body, .. }
                | Statement::For { body, .. } => {
                    if self.is_used_in_binary_op(name, body) {
                        return true;
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expr_uses_in_binary_op(name, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expr_uses_in_binary_op(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary { left, right, .. } => {
                // Check if the parameter is directly used in a binary operation
                // This is for Copy types like Vec2, Vec3 where `a + b` requires owned values
                if self.expr_is_identifier(left, name) || self.expr_is_identifier(right, name) {
                    return true;
                }
                // Recursively check nested expressions
                self.expr_uses_in_binary_op(name, left) || self.expr_uses_in_binary_op(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_uses_in_binary_op(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_uses_in_binary_op(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_uses_in_binary_op(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_uses_in_binary_op(name, arg))
            }
            // CRITICAL FIX: Don't recurse into FieldAccess for binary op detection
            // `self.field + value` doesn't mean `self` is used in a binary op
            // We only care about the DIRECT use of the parameter, like `param + value`
            Expression::FieldAccess { .. } => false,
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_binary_op(name, object)
                    || self.expr_uses_in_binary_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_binary_op(name, statements),
            // Recurse into tuple elements
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_binary_op(name, elem)),
            // Recurse into array elements
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_binary_op(name, elem)),
            Expression::TryOp { expr, .. } => self.expr_uses_in_binary_op(name, expr),
            _ => false,
        }
    }

    /// `let v = param` (and chains) so that `if let` / `match` on `v` must still require an owned
    /// parameter when the arm moves out of `Option`/`Result`, etc.
    fn simple_let_alias_ids_for_param(
        &self,
        param_name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert(param_name.to_string());
        let mut changed = true;
        while changed {
            changed = false;
            self.simple_let_alias_expand_pass(&mut set, statements, &mut changed);
        }
        set
    }

    fn simple_let_alias_expand_pass(
        &self,
        set: &mut HashSet<String>,
        statements: &[&'ast Statement<'ast>],
        changed: &mut bool,
    ) {
        for stmt in statements {
            match stmt {
                Statement::Let { pattern, value, .. } => {
                    if let Pattern::Identifier(local) = pattern {
                        if let Expression::Identifier { name: src, .. } = &**value {
                            if set.contains(src) && set.insert(local.clone()) {
                                *changed = true;
                            }
                        }
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.simple_let_alias_expand_pass(set, then_block, changed);
                    if let Some(else_b) = else_block {
                        self.simple_let_alias_expand_pass(set, else_b, changed);
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    self.simple_let_alias_expand_pass(set, body, changed);
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.simple_let_alias_expand_pass(set, statements, changed);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check if a parameter is pattern matched with field extraction
    /// e.g., `match param { Enum::Variant { field: f } => ... }`
    /// If we borrow the parameter, `f` becomes a reference, breaking calls expecting owned values
    fn is_pattern_matched_with_fields(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        let aliases = self.simple_let_alias_ids_for_param(name, statements);
        self.match_arm_destructures_enum_subpatterns_in_stmts(&aliases, statements)
    }

    fn match_arm_destructures_enum_subpatterns_in_stmts(
        &self,
        aliases: &HashSet<String>,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Match { value, arms, .. } => {
                    if let Expression::Identifier { name: id, .. } = value {
                        if aliases.contains(id) {
                            for arm in arms {
                                if self.pattern_has_field_bindings(&arm.pattern) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, else_b) {
                            return true;
                        }
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a pattern has field bindings (not just wildcards or simple identifiers)
    fn pattern_has_field_bindings(&self, pattern: &Pattern) -> bool {
        use crate::parser::EnumPatternBinding;

        match pattern {
            Pattern::EnumVariant(_, binding) => {
                // Check if the binding extracts fields
                matches!(
                    binding,
                    EnumPatternBinding::Single(_)
                        | EnumPatternBinding::Tuple(_)
                        | EnumPatternBinding::Struct(_, _)
                )
            }
            Pattern::Tuple(patterns) => patterns.iter().any(|p| self.pattern_has_field_bindings(p)),
            Pattern::Or(patterns) => patterns.iter().any(|p| self.pattern_has_field_bindings(p)),
            _ => false,
        }
    }

    fn build_signature(&self, func: &AnalyzedFunction) -> FunctionSignature {
        let param_ownership: Vec<OwnershipMode> = func
            .decl
            .parameters
            .iter()
            .map(|param| {
                // CRITICAL FIX: Check the actual type annotation FIRST
                // If parameter is explicitly declared as &T or &mut T, respect that
                use crate::parser::Type;
                match &param.type_ {
                    Type::Reference(_) => {
                        // Parameter is explicitly &T - must borrow
                        return OwnershipMode::Borrowed;
                    }
                    Type::MutableReference(_) => {
                        // Parameter is explicitly &mut T - must mut borrow
                        return OwnershipMode::MutBorrowed;
                    }
                    _ => {
                        // Not an explicit reference, use inference
                    }
                }

                let inferred = func
                    .inferred_ownership
                    .get(&param.name)
                    .cloned()
                    .unwrap_or(OwnershipMode::Owned);

                // CRITICAL: Generic type parameters (like G in fn foo<G: Trait>(g: G))
                // should ALWAYS be Owned. The trait bound is on G, not on &G.
                // Adding & at call sites would break trait bounds.
                if Self::is_generic_type_param(&param.type_) {
                    return OwnershipMode::Owned;
                }

                // Copy types are always passed by value (Owned) unless mutated
                // This must match the logic in codegen.rs
                if self.is_copy_type(&param.type_) {
                    // Copy types: pass by value unless they need to be mutated
                    if inferred == OwnershipMode::MutBorrowed {
                        OwnershipMode::MutBorrowed
                    } else {
                        OwnershipMode::Owned
                    }
                } else {
                    // THE WINDJAMMER WAY: The compiler infers ownership, not the user.
                    // Non-Copy types follow the analyzer's inference:
                    // - Borrowed: parameter is only read (default for read-only params)
                    // - MutBorrowed: parameter is mutated
                    // - Owned: parameter is consumed (returned, stored, iterated, etc.)
                    //
                    // Users write `data: Vec<f32>` and the compiler figures out whether
                    // it should be `&Vec<f32>`, `&mut Vec<f32>`, or `Vec<f32>` in Rust.
                    // This matches call sites where `&self.data` is naturally passed.
                    inferred
                }
            })
            .collect();

        // PHASE 2 STRING OPTIMIZATION: Use inferred parameter types when available
        // The analyzer determines which string parameters can be &str vs &String
        // based on how they're used in the function body.
        let mut param_types: Vec<Type> = func
            .decl
            .parameters
            .iter()
            .enumerate()
            .map(|(idx, param)| {
                // Use inferred type if available (Phase 2 optimization)
                // Otherwise fall back to explicit type annotation
                func.inferred_param_types
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| param.type_.clone())
            })
            .collect();

        let explicit_self = func
            .decl
            .parameters
            .first()
            .is_some_and(|p| p.name == "self" || p.name == "mut self");

        // Omitted `self` in source (`fn touch() { self... }`): analyzer stores ownership under
        // "self" but decl.parameters has no receiver. SignatureRegistry must still expose
        // `has_self_receiver` + `param_ownership[0]` so cross-type calls (e.g. `.touch()`) resolve.
        let synthetic_self_receiver =
            func.inferred_ownership.contains_key("self") && !explicit_self;

        let mut param_ownership = param_ownership;
        if synthetic_self_receiver {
            let self_mode = func
                .inferred_ownership
                .get("self")
                .copied()
                .unwrap_or(OwnershipMode::Borrowed);
            param_ownership.insert(0, self_mode);
            let self_ty = func
                .decl
                .parent_type
                .as_ref()
                .map(|n| Type::Custom(n.clone()))
                .unwrap_or(Type::Custom("Self".to_string()));
            param_types.insert(0, self_ty);
        }

        let has_self_receiver = explicit_self || synthetic_self_receiver;

        // Extract return type for smart string inference
        let return_type = func.decl.return_type.clone();

        FunctionSignature {
            name: func.decl.name.clone(),
            param_types,
            param_ownership,
            return_type,
            return_ownership: OwnershipMode::Owned, // For now, always owned
            has_self_receiver,
            is_extern: func.decl.is_extern,
        }
    }

    /// Check if a type is a generic type parameter (like T, G, S, T1, T2, etc.)
    /// or an impl Trait parameter (like `impl Describable`).
    /// Generic type parameters are typically single uppercase letters or uppercase with numbers.
    /// impl Trait parameters use static dispatch and should always be Owned
    /// (adding & would change the trait bound from `T: Trait` to `&T: Trait`).
    /// This matches the logic in codegen/rust/generator.rs is_generic_type().
    fn is_generic_type_param(ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => {
                // Generic type parameters are single uppercase letters, optionally followed by a digit.
                // Examples: T, U, K, V, S, G, T1, T2
                // NOT: BVH, GPU, API, SVO, AABB, AABB3 (these are concrete type names)
                let len = name.len();
                if len == 1 {
                    name.chars().next().is_some_and(|c| c.is_uppercase())
                } else if len == 2 {
                    let mut chars = name.chars();
                    let first = chars.next().unwrap();
                    let second = chars.next().unwrap();
                    first.is_uppercase() && second.is_ascii_digit()
                } else {
                    false
                }
            }
            // impl Trait parameters (e.g., `item: impl Describable`) should always be Owned.
            // Borrowing would change from `impl Trait` to `&impl Trait`, breaking trait dispatch.
            Type::ImplTrait(_) => true,
            _ => false,
        }
    }

    /// Check if param type appears in return type (direct, Result<T,E>, or Option<T>).
    /// When fn(T) -> Result<T,E>, we need owned param to produce the return value.
    fn param_type_matches_return(&self, param_type: &Type, return_type: &Type) -> bool {
        match return_type {
            // Direct match: fn(T) -> T
            t if self.types_equal(param_type, t) => true,
            // Result<T, E>: fn(T) -> Result<T, E>
            Type::Result(ok_type, _err_type) => self.types_equal(param_type, ok_type),
            // Option<T>: fn(T) -> Option<T>
            Type::Option(inner) => self.types_equal(param_type, inner),
            _ => false,
        }
    }

    /// Compare two types for equality (custom types, primitives).
    /// Callee parameter type from `SignatureRegistry` must match the caller's declared type
    /// before passthrough ownership is applied. Prevents short names like `contains` / `len`
    /// from unrelated impls forcing Owned on `str` / `string` parameters (E0382).
    pub(super) fn passthrough_types_compatible(&self, sig_ty: &Type, decl_ty: &Type) -> bool {
        if self.types_equal(sig_ty, decl_ty) {
            return true;
        }
        let decl_str = matches!(decl_ty, Type::String);
        let sig_str = matches!(sig_ty, Type::String);
        decl_str && sig_str
    }

    fn types_equal(&self, a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Custom(name_a), Type::Custom(name_b)) => name_a == name_b,
            (Type::String, Type::String) => true,
            (Type::Int, Type::Int) => true,
            (Type::Int32, Type::Int32) => true,
            (Type::Uint, Type::Uint) => true,
            (Type::Float, Type::Float) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Vec(inner_a), Type::Vec(inner_b)) => self.types_equal(inner_a, inner_b),
            (Type::Option(inner_a), Type::Option(inner_b)) => self.types_equal(inner_a, inner_b),
            (Type::Array(inner_a, _), Type::Array(inner_b, _)) => {
                self.types_equal(inner_a, inner_b)
            }
            (Type::Result(ok_a, err_a), Type::Result(ok_b, err_b)) => {
                self.types_equal(ok_a, ok_b) && self.types_equal(err_a, err_b)
            }
            (Type::Tuple(elems_a), Type::Tuple(elems_b)) => {
                elems_a.len() == elems_b.len()
                    && elems_a
                        .iter()
                        .zip(elems_b.iter())
                        .all(|(a, b)| self.types_equal(a, b))
            }
            (Type::Reference(inner_a), Type::Reference(inner_b)) => {
                self.types_equal(inner_a, inner_b)
            }
            (Type::MutableReference(inner_a), Type::MutableReference(inner_b)) => {
                self.types_equal(inner_a, inner_b)
            }
            (Type::Parameterized(name_a, args_a), Type::Parameterized(name_b, args_b)) => {
                name_a == name_b
                    && args_a.len() == args_b.len()
                    && args_a
                        .iter()
                        .zip(args_b.iter())
                        .all(|(a, b)| self.types_equal(a, b))
            }
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_copy_type(&self, ty: &Type) -> bool {
        use crate::parser::Type;
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,
            Type::MutableReference(_) => false,
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            // Option<T> is Copy if T is Copy
            Type::Option(inner) => self.is_copy_type(inner),
            // Result<T, E> is Copy if both T and E are Copy
            Type::Result(ok_type, err_type) => {
                self.is_copy_type(ok_type) && self.is_copy_type(err_type)
            }
            // Fixed-size arrays [T; N] are Copy if T is Copy
            Type::Array(inner, _size) => self.is_copy_type(inner),
            // Vec<T> is never Copy (heap-allocated)
            Type::Vec(_) => false,
            // TDD FIX: Function pointers are ALWAYS Copy (they're just pointers!)
            // Example: fn(string, i32) -> bool is Copy
            // This ensures function pointer parameters are inferred as Owned, not Borrowed
            Type::FunctionPointer { .. } => true,
            Type::RawPointer { .. } => true,
            Type::Custom(name) => {
                // Check if it's a known Copy enum
                if self.copy_enums.contains(name) {
                    return true;
                }
                // Check if it's a known Copy struct (detected via @derive(Copy))
                // This now works properly via the global copy_structs registry which is
                // populated from all files in PASS 0 before any individual file compilation
                if self.copy_structs.contains(name) {
                    return true;
                }
                crate::type_classification::is_copy_primitive(name)
            }
            _ => false,
        }
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
