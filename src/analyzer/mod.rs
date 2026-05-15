#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Ownership and borrow checking analyzer
use crate::auto_clone::AutoCloneAnalysis;
use crate::parser::*;
use std::collections::HashMap;

mod generic_analysis;
mod module_analysis;
mod function_analysis;
mod scope_analysis;
mod type_checking;
mod mutation_detection;
mod optimization_detectors;
mod trait_analysis;
mod passthrough_inference;
mod self_mutating_calls;
mod self_binding_mutation;
mod self_field_mutation;
mod self_return_and_consumption;
mod self_access_and_option_refs;
mod self_dispatch_for_loops;
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
