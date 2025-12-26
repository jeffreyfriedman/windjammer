// Ownership and borrow checking analyzer
use crate::auto_clone::AutoCloneAnalysis;
use crate::parser::*;
use std::collections::HashMap;

// Type alias for complex return type
type ProgramAnalysisResult = (
    Vec<AnalyzedFunction>,
    SignatureRegistry,
    HashMap<String, HashMap<String, AnalyzedFunction>>,
);

#[derive(Debug, Clone)]
pub struct AnalyzedFunction {
    pub decl: FunctionDecl,
    pub inferred_ownership: HashMap<String, OwnershipMode>,
    // STRING INFERENCE: Track inferred types for string parameters (&str vs String)
    pub inferred_param_types: Vec<Type>,
    // AUTO-MUT: Track which local variables are mutated (for automatic mut inference)
    pub mutated_variables: HashSet<String>,
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
    signatures: HashMap<String, FunctionSignature>,
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
        };

        // Populate with stdlib signatures by scanning windjammer-runtime source
        if let Err(e) = crate::stdlib_scanner::populate_runtime_signatures(&mut registry) {
            eprintln!("Warning: Failed to scan runtime signatures: {}", e);
            eprintln!("Continuing with empty registry - may generate incorrect borrows");
        }

        registry
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSignature) {
        self.signatures.insert(name, sig);
    }

    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(name)
    }
}

pub struct Analyzer {
    // Track variable ownership modes (reserved for future use)
    #[allow(dead_code)]
    variables: HashMap<String, OwnershipMode>,
    // Track enum definitions to determine if they're Copy
    copy_enums: HashSet<String>,
    // Track struct definitions with @derive(Copy) to determine if they're Copy
    copy_structs: HashSet<String>,
    // Track trait definitions for impl block analysis
    trait_definitions: HashMap<String, TraitDecl>,
    // Track analyzed trait methods (trait_name -> method_name -> AnalyzedFunction)
    // PUBLIC: The generator needs this for trait signature inference
    pub analyzed_trait_methods: HashMap<String, HashMap<String, AnalyzedFunction>>,
    // Track which local variables are mutated (for automatic mut inference)
    mutated_variables: HashSet<String>,
    // Track functions in the current impl block (for cross-method analysis)
    current_impl_functions: Option<HashMap<String, crate::parser::ast::FunctionDecl>>,
}

use std::collections::HashSet;

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
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
        };

        // Pre-register standard library traits so the analyzer knows their signatures
        analyzer.register_stdlib_traits();

        analyzer
    }

    /// Update the analyzer's Copy structs registry (for shared analyzer across files)
    /// This allows newly discovered Copy structs to be available for subsequent file analysis
    pub fn update_copy_structs(&mut self, global_copy_structs: HashSet<String>) {
        self.copy_structs = global_copy_structs;
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
                        },
                        Parameter {
                            name: "rhs".to_string(),
                            pattern: None,
                            type_: Type::Custom("Rhs".to_string()),
                            ownership: OwnershipHint::Owned,
                            is_mutable: false,
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
    }

    /// Register trait definitions from an external program (e.g., imported module)
    /// This allows the analyzer to use trait signatures when analyzing impl blocks
    /// in files that import traits from other modules.
    pub fn register_traits_from_program(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Trait { decl, .. } = item {
                self.trait_definitions
                    .insert(decl.name.clone(), decl.clone());
            }
        }
    }

    pub fn analyze_program(&mut self, program: &Program) -> Result<ProgramAnalysisResult, String> {
        let mut analyzed = Vec::new();
        let mut registry = SignatureRegistry::new();

        // First pass: Collect all enum and struct definitions to determine Copy types
        // and collect all trait definitions for impl block analysis
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
                Item::Struct { decl, .. } => {
                    // Check if struct has @derive(Copy) decorator
                    // This will work when all files are compiled together in one pass
                    let has_copy_derive = decl.decorators.iter().any(|decorator| {
                        decorator.name == "derive"
                            && decorator.arguments.iter().any(|(_, arg)| {
                                if let crate::parser::ast::Expression::Identifier { name, .. } = arg
                                {
                                    name == "Copy"
                                } else {
                                    false
                                }
                            })
                    });

                    // Also check for @auto decorator - if all fields are Copy, struct gets Copy
                    let has_auto_derive = decl.decorators.iter().any(|d| d.name == "auto");
                    let all_fields_copy =
                        decl.fields.iter().all(|f| self.is_copy_type(&f.field_type));

                    if has_copy_derive || (has_auto_derive && all_fields_copy) {
                        self.copy_structs.insert(decl.name.clone());
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

        // NOTE: Trait signature inference is now done GLOBALLY after all files are compiled
        // See ModuleCompiler::finalize_trait_inference() in main.rs
        // (We no longer call infer_trait_signatures_from_impls here for single files)

        for item in &program.items {
            match item {
                Item::Function { decl: func, .. } => {
                    let mut analyzed_func = self.analyze_function(func)?;

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
                    // Analyze methods in impl blocks
                    for func in &impl_block.functions {
                        // Check if this is a trait implementation
                        let mut analyzed_func = if let Some(trait_name) = &impl_block.trait_name {
                            // This is a trait impl - use trait method signatures
                            self.analyze_trait_impl_function(func, trait_name)?
                        } else {
                            // Regular impl - infer as usual (pass impl_block for cross-method analysis)
                            self.analyze_function_in_impl(func, impl_block)?
                        };

                        // PHASE 7: Detect const/static optimizations
                        analyzed_func.const_static_optimizations =
                            self.detect_const_static_opportunities(&analyzed_func);

                        // PHASE 8: Detect SmallVec optimizations
                        analyzed_func.smallvec_optimizations =
                            self.detect_smallvec_opportunities(func);

                        // PHASE 9: Detect Cow optimizations
                        analyzed_func.cow_optimizations = self.detect_cow_opportunities(func);

                        let signature = self.build_signature(&analyzed_func);
                        registry.add_function(func.name.clone(), signature);
                        analyzed.push(analyzed_func);
                    }
                }
                Item::Trait { decl, .. } => {
                    // Analyze trait methods with default implementations
                    for method in &decl.methods {
                        // Only analyze methods with bodies (default implementations)
                        if method.body.is_none() {
                            // Abstract method - no analysis needed (no body)
                            continue;
                        }

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
                            body: method.body.clone().unwrap_or_default(),
                            parent_type: None,
                            doc_comment: method.doc_comment.clone(),
                        };

                        // Trait methods with default implementations should use &self
                        // to work with unsized types. The Windjammer way: make it work!
                        let mut analyzed_func = self.analyze_trait_method(&func)?;

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

                        // Only insert if not already present (from global inference)
                        if !trait_methods.contains_key(&func.name) {
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
                                let mut analyzed_func = self.analyze_function(func)?;
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
                                // Analyze methods in impl blocks inside modules
                                for func in &impl_block.functions {
                                    let mut analyzed_func =
                                        if let Some(trait_name) = &impl_block.trait_name {
                                            self.analyze_trait_impl_function(func, trait_name)?
                                        } else {
                                            self.analyze_function_in_impl(func, impl_block)?
                                        };
                                    analyzed_func.const_static_optimizations =
                                        self.detect_const_static_opportunities(&analyzed_func);
                                    analyzed_func.smallvec_optimizations =
                                        self.detect_smallvec_opportunities(func);
                                    analyzed_func.cow_optimizations =
                                        self.detect_cow_opportunities(func);
                                    let signature = self.build_signature(&analyzed_func);
                                    registry.add_function(func.name.clone(), signature);
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

        Ok((analyzed, registry, self.analyzed_trait_methods.clone()))
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
    pub fn infer_trait_signatures_from_impls(&mut self, program: &Program) -> Result<(), String> {
        use std::collections::HashMap;

        eprintln!(
            "DEBUG: infer_trait_signatures_from_impls called with {} items",
            program.items.len()
        );

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

        eprintln!("DEBUG: Found {} trait implementations", trait_impls.len());
        for trait_name in trait_impls.keys() {
            eprintln!("DEBUG:   - {}", trait_name);
        }

        // Step 2: For each trait, analyze ALL implementations and determine most permissive signature
        for (trait_name, impl_blocks) in trait_impls {
            eprintln!(
                "DEBUG INFERENCE: Processing trait: {} with {} impl blocks",
                trait_name,
                impl_blocks.len()
            );
            if let Some(trait_methods) = self.analyzed_trait_methods.get(&trait_name).cloned() {
                eprintln!(
                    "DEBUG INFERENCE:   Found {} methods in trait",
                    trait_methods.len()
                );
                for (method_name, method_analysis) in &trait_methods {
                    eprintln!(
                        "DEBUG INFERENCE:     Method {} has self ownership: {:?}",
                        method_name,
                        method_analysis.inferred_ownership.get("self")
                    );
                }
                let mut updated_methods = HashMap::new();

                for (method_name, mut trait_method_analysis) in trait_methods {
                    // WINDJAMMER PHILOSOPHY: Infer optimal trait signature from ALL implementations
                    // - Start with trait's default implementation inference (if any)
                    // - Upgrade based on what implementations actually need
                    // - If any impl needs `&mut self`, upgrade trait to `&mut self`
                    // - This ensures implementations don't violate borrow checker

                    let initial_self_ownership = trait_method_analysis
                        .inferred_ownership
                        .get("self")
                        .copied()
                        .unwrap_or(OwnershipMode::Borrowed);

                    let mut most_permissive_self = initial_self_ownership;

                    // Examine ALL implementations to find what they actually need for `self`
                    // IMPORTANT: We only upgrade `self`, not other parameters
                    // Parameters stay as the trait defines them (user's explicit choice)
                    for impl_block in &impl_blocks {
                        for func in &impl_block.functions {
                            if func.name == method_name {
                                let impl_analysis =
                                    self.analyze_function_in_impl(func, impl_block)?;

                                // Upgrade self ownership if implementation needs more permission
                                if let Some(&impl_self_ownership) =
                                    impl_analysis.inferred_ownership.get("self")
                                {
                                    // Upgrade: Owned > MutBorrowed > Borrowed
                                    match (most_permissive_self, impl_self_ownership) {
                                        (OwnershipMode::Borrowed, OwnershipMode::MutBorrowed) => {
                                            most_permissive_self = OwnershipMode::MutBorrowed;
                                        }
                                        (OwnershipMode::Borrowed, OwnershipMode::Owned) => {
                                            most_permissive_self = OwnershipMode::Owned;
                                        }
                                        (OwnershipMode::MutBorrowed, OwnershipMode::Owned) => {
                                            most_permissive_self = OwnershipMode::Owned;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }

                    // Update trait method with upgraded self ownership
                    // Parameters stay as originally analyzed (from trait definition)
                    trait_method_analysis
                        .inferred_ownership
                        .insert("self".to_string(), most_permissive_self);

                    eprintln!(
                        "DEBUG:     Method {} upgraded to self: {:?}",
                        method_name, most_permissive_self
                    );

                    updated_methods.insert(method_name, trait_method_analysis);
                }

                // Replace the trait's analyzed methods with updated versions
                self.analyzed_trait_methods
                    .insert(trait_name, updated_methods);
            }
        }

        Ok(())
    }

    fn analyze_trait_method(&mut self, func: &FunctionDecl) -> Result<AnalyzedFunction, String> {
        // Analyze the function normally first
        let mut analyzed = self.analyze_function(func)?;

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
                    OwnershipHint::Owned | OwnershipHint::Inferred => {
                        // User wrote `self` (inferred) - optimize based on usage
                        // If body exists, infer from body; otherwise will be refined by infer_trait_signatures_from_impls
                        let modifies_self = self.function_modifies_self_fields(func);
                        let self_ownership = if modifies_self {
                            OwnershipMode::MutBorrowed
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

        Ok(analyzed)
    }

    /// Analyze a function within an impl block (has access to other methods for cross-method analysis)
    fn analyze_function_in_impl(
        &mut self,
        func: &FunctionDecl,
        impl_block: &crate::parser::ast::ImplBlock,
    ) -> Result<AnalyzedFunction, String> {
        // Store current impl block for cross-method lookups
        self.current_impl_functions = Some(
            impl_block
                .functions
                .iter()
                .map(|f| (f.name.clone(), f.clone()))
                .collect(),
        );

        let result = self.analyze_function(func);

        // Clear impl block after analysis
        self.current_impl_functions = None;

        result
    }

    fn analyze_function(&mut self, func: &FunctionDecl) -> Result<AnalyzedFunction, String> {
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
            let modifies_fields = self.function_modifies_self_fields(func);
            let returns_self = self.function_returns_self(func);

            let self_ownership = if returns_self {
                // Builder pattern: mut self (owned)
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
                    // Special case: 'self' parameter in impl methods
                    if param.name == "self" {
                        // Check if this method modifies any fields
                        let modifies_fields = self.function_modifies_self_fields(func);
                        let returns_self = self.function_returns_self(func);

                        // CRITICAL FIX: Check returns_self FIRST!
                        // If a function returns Self, it's a builder pattern and should always consume self (Owned)
                        // This is true even if it doesn't directly modify fields (it creates a new struct instead)
                        if returns_self {
                            // Builder pattern: consumes self, returns Self (either `self` or a new struct literal)
                            // Use `mut self` (Owned), not `&self` (Borrowed)
                            OwnershipMode::Owned
                        } else if modifies_fields {
                            // Mutating method that doesn't return self: use `&mut self`
                            OwnershipMode::MutBorrowed
                        } else {
                            // Check if self is used in binary operations (for Copy types like Vec2, Vec3)
                            if self.is_used_in_binary_op("self", &func.body) {
                                OwnershipMode::Owned
                            } else {
                                // Default to borrowed for read-only methods
                                // Whether or not the method accesses self.fields, &self is appropriate
                                OwnershipMode::Borrowed
                            }
                        }
                    } else {
                        OwnershipMode::Owned
                    }
                }
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Ref => {
                    // SMART FIX: If user wrote &self but function modifies fields, upgrade to &mut self
                    // This prevents a common user error
                    if param.name == "self" && self.function_modifies_self_fields(func) {
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
                        // Infer ownership for self based on field access and return type
                        let modifies_fields = self.function_modifies_self_fields(func);
                        let returns_self = self.function_returns_self(func);

                        // CRITICAL FIX: Check returns_self FIRST!
                        // If a function returns Self, it's a builder pattern and should always consume self (Owned)
                        // This is true even if it doesn't directly modify fields (it creates a new struct literal)
                        if returns_self {
                            // Builder pattern: consumes self, returns Self (either `self` or a new struct literal)
                            // Use `mut self` (Owned), not `&self` (Borrowed)
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
                    } else {
                        // For Copy types, check if they're mutated first
                        // Mutated Copy types should be &mut, not Owned
                        if self.is_copy_type(&param.type_) {
                            // Still check for mutation - mutated Copy types need &mut
                            if self.is_mutated(&param.name, &func.body) {
                                OwnershipMode::MutBorrowed
                            } else {
                                // Non-mutated Copy types default to Owned (pass by value)
                                OwnershipMode::Owned
                            }
                        } else {
                            // Perform inference based on usage in function body
                            self.infer_parameter_ownership(
                                &param.name,
                                &param.type_,
                                &func.body,
                                &func.return_type,
                            )?
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
        let defer_drop_optimizations = self.detect_defer_drop_opportunities(func);

        // AUTO-CLONE: Analyze where clones should be automatically inserted
        let auto_clone_analysis = AutoCloneAnalysis::analyze_function(func);

        // AUTO-MUT: Track which local variables are mutated (for automatic mut inference)
        self.track_mutations(&func.body);
        let mutated_variables = self.mutated_variables.clone();

        // PHASE 7-9: Additional optimizations (future implementation)
        let const_static_optimizations = Vec::new(); // TODO: Implement detection
        let smallvec_optimizations = Vec::new(); // TODO: Implement detection
        let cow_optimizations = Vec::new(); // TODO: Implement detection

        // THE WINDJAMMER WAY: Keep parameter types as explicitly declared
        // No smart string inference for parameters - respect the API contract
        let inferred_param_types: Vec<Type> = func
            .parameters
            .iter()
            .map(|param| param.type_.clone())
            .collect();

        Ok(AnalyzedFunction {
            decl: func.clone(),
            inferred_ownership,
            inferred_param_types,
            mutated_variables,
            auto_clone_analysis,
            clone_optimizations,
            struct_mapping_optimizations,
            string_optimizations,
            assignment_optimizations,
            defer_drop_optimizations,
            const_static_optimizations,
            smallvec_optimizations,
            cow_optimizations,
        })
    }

    /// Analyze a function that implements a trait method
    /// Use the trait's method signature instead of inferring
    fn analyze_trait_impl_function(
        &mut self,
        func: &FunctionDecl,
        trait_name: &str,
    ) -> Result<AnalyzedFunction, String> {
        // Start with regular analysis
        let mut analyzed = self.analyze_function(func)?;

        // Look up the trait definition
        // Try both the full trait name and just the last segment (e.g., "std::ops::Add" -> "Add")
        let trait_key = if let Some(pos) = trait_name.rfind("::") {
            &trait_name[pos + 2..]
        } else {
            trait_name
        };

        // Check if this is a standard operator trait (std::ops::Add, Sub, Mul, etc.)
        // These traits use `self` (owned) for Copy types, not `&self`
        let is_std_operator_trait = matches!(
            trait_key,
            "Add"
                | "Sub"
                | "Mul"
                | "Div"
                | "Rem"
                | "Neg"
                | "Not"
                | "BitAnd"
                | "BitOr"
                | "BitXor"
                | "Shl"
                | "Shr"
        );

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
                        let trait_mode = if let Some(trait_methods) =
                            self.analyzed_trait_methods.get(trait_key)
                        {
                            if let Some(analyzed_trait_method) = trait_methods.get(&func.name) {
                                // Use analyzed ownership (takes priority!)
                                if let Some(analyzed_ownership) = analyzed_trait_method
                                    .inferred_ownership
                                    .get(&trait_param.name)
                                {
                                    *analyzed_ownership
                                } else {
                                    // Fall back to converting AST ownership hint
                                    self.convert_ownership_hint_to_mode(
                                        &trait_param.ownership,
                                        &trait_param.name,
                                    )
                                }
                            } else {
                                // Trait method not analyzed, convert from AST
                                self.convert_ownership_hint_to_mode(
                                    &trait_param.ownership,
                                    &trait_param.name,
                                )
                            }
                        } else {
                            // Trait not analyzed, convert from AST
                            self.convert_ownership_hint_to_mode(
                                &trait_param.ownership,
                                &trait_param.name,
                            )
                        };

                        // WINDJAMMER PHILOSOPHY: Trait signatures are contracts
                        // Implementations MUST match the trait signature EXACTLY
                        // This is true whether or not the trait has a default implementation
                        // The trait defines the interface - impls conform to it, not vice versa
                        let final_mode = trait_mode;

                        // INSERT or UPDATE with the final ownership mode
                        analyzed
                            .inferred_ownership
                            .insert(impl_param.name.clone(), final_mode);
                    }
                }

                // WINDJAMMER PHILOSOPHY: For traits WITHOUT default implementations,
                // update the trait method's analyzed ownership from the impl.
                // This ensures the trait signature matches the impl's inferred signature.
                if trait_method.body.is_none() {
                    // Trait has no default implementation - use impl's analyzed ownership
                    // Create or update the analyzed trait method entry
                    let analyzed_trait_methods_for_trait = self
                        .analyzed_trait_methods
                        .entry(trait_key.to_string())
                        .or_default();

                    // Store the impl's analyzed ownership for this trait method
                    analyzed_trait_methods_for_trait.insert(func.name.clone(), analyzed.clone());
                }
            }
        }

        Ok(analyzed)
    }

    fn infer_parameter_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[Statement],
        _return_type: &Option<Type>,
    ) -> Result<OwnershipMode, String> {
        // Simple heuristic-based inference

        // 1. Check if parameter is mutated
        if self.is_mutated(param_name, body) {
            return Ok(OwnershipMode::MutBorrowed);
        }

        // 2. Check if parameter is returned (escapes function)
        if self.is_returned(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

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

        // 3. Check if parameter is stored in a struct or collection
        if self.is_stored(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 4. Check if parameter is used in binary operations (for Copy types)
        // Copy types used in operators (a - b, a + b, etc.) should remain owned
        // because operator traits are typically implemented for owned values, not references
        if self.is_used_in_binary_op(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 5. Check if parameter is pattern matched with field extraction
        // Borrowing an enum and pattern matching extracts references to fields
        // which breaks calls expecting owned values. Keep such parameters owned.
        if self.is_pattern_matched_with_fields(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 6. Default ownership based on type
        // THE WINDJAMMER WAY: Respect explicit type annotations
        //
        // - Copy types → Owned (passed by value)
        // - Generic types → Owned (trait bounds are on T, not &T)
        // - String type → Owned (user said String, not &str)
        // - Other types → Borrowed (optimize by default)

        if self.is_copy_type(param_type) {
            Ok(OwnershipMode::Owned)
        } else if Self::is_generic_type_param(param_type) {
            // Keep generic types owned - trait bounds are on T, not &T
            Ok(OwnershipMode::Owned)
        } else if matches!(param_type, Type::String) {
            // THE WINDJAMMER WAY: Explicit String type annotation means owned String
            // User wrote `text: string` → they want `text: String`, not `text: &str`
            Ok(OwnershipMode::Owned)
        } else {
            // Other non-Copy types can be borrowed by default
            Ok(OwnershipMode::Borrowed)
        }
    }

    fn is_used_in_if_else_expression(&self, name: &str, statements: &[Statement]) -> bool {
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

    fn stmts_have_if_else_with_param(&self, name: &str, stmts: &[Statement]) -> bool {
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

    fn stmts_mention_identifier(&self, name: &str, stmts: &[Statement]) -> bool {
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
            _ => false,
        }
    }

    fn is_mutated(&self, name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment { target, .. } => {
                    // Check if the assignment target is the parameter itself
                    if let Expression::Identifier { name: id, .. } = target {
                        if id == name {
                            return true;
                        }
                    }

                    // THE WINDJAMMER WAY: Check if the assignment target is a field of the parameter
                    // e.g., p.x = ... or p.position.x = ...
                    // But NOT if the parameter is just used in an index expression!
                    // e.g., arr[entity.index] = x  <- entity is READ, not mutated
                    if self.is_direct_mutation_target(name, target) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    // Check for method calls that might mutate
                    if self.has_mutable_method_call(name, expr) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_mutated(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_mutated(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. }
                | Statement::While { body, .. }
                | Statement::For { body, .. } => {
                    if self.is_mutated(name, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a parameter is the DIRECT target of mutation
    /// Returns true for: p = x, p.field = x, p.field.nested = x
    /// Returns false for: arr[p.index] = x, obj[p] = x  (p is only READ here)
    fn is_direct_mutation_target(&self, name: &str, target: &Expression) -> bool {
        match target {
            // Direct assignment: p = ...
            Expression::Identifier { name: id, .. } => id == name,

            // Field access: p.x = ... or p.field.nested = ...
            Expression::FieldAccess { object, .. } => {
                // Recursively check if parameter is the object being accessed
                self.is_direct_mutation_target(name, object)
            }

            // Index access: arr[i] = ...
            // The parameter is NOT being mutated if it appears in the index
            // Example: arr[entity.index] = x  <- entity is READ, not mutated
            Expression::Index { object, .. } => {
                // Only check the object, not the index!
                // If `name` appears in the object, then it's being mutated
                // e.g., p[0] = x  <- p is mutated
                // But NOT: arr[p.index] = x  <- p is only READ
                self.is_direct_mutation_target(name, object)
            }

            _ => false,
        }
    }

    fn has_mutable_method_call(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name: id, .. } = &**object {
                    if id == name {
                        // Heuristic: methods like push, insert, etc. are mutating
                        return method.starts_with("push")
                            || method.starts_with("insert")
                            || method.starts_with("remove")
                            || method.starts_with("clear")
                            || method.ends_with("_mut");
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn is_returned(&self, name: &str, statements: &[Statement]) -> bool {
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
                        if self.expression_uses_identifier_for_return(name, &arm.body) {
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

            // Wrapped in Some, Ok, Err, etc.
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Check if this is a wrapper call like Some(param) or Ok(param)
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    if matches!(fn_name.as_str(), "Some" | "Ok" | "Err") {
                        for (_label, arg) in arguments {
                            if self.expression_uses_identifier(name, arg) {
                                return true;
                            }
                        }
                    }
                }
                // Also check general function calls
                self.expression_uses_identifier(name, expr)
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

            // Default: use standard identifier check
            _ => self.expression_uses_identifier(name, expr),
        }
    }

    fn is_stored(&self, name: &str, statements: &[Statement]) -> bool {
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
                    // CRITICAL FIX: Only consider it "stored" if the parameter is DIRECTLY assigned
                    // to a field (self.field = param), not if it's just used in a calculation
                    // (self.field = self.field * param).
                    //
                    // Direct assignment: self.field = param
                    // Calculation: self.field = self.field * param (or any other expression)
                    //
                    // We check if the value is JUST the identifier, not part of a larger expression.
                    if matches!(&**object, Expression::Identifier { name: id, .. } if id == "self")
                    {
                        // Only return true if the value is EXACTLY the parameter identifier
                        if matches!(value, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
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
                    // Check for method calls on fields: self.field.push(param), self.field.insert(param), etc.
                    // Only consider storage methods (push, insert, extend) - not lookup methods (contains, get)
                    let is_storage_method = matches!(
                        method.as_str(),
                        "push"
                            | "insert"
                            | "extend"
                            | "append"
                            | "add"
                            | "push_back"
                            | "push_front"
                    );

                    if is_storage_method {
                        // Check for method calls on fields: self.field.push(param)
                        if let Expression::FieldAccess {
                            object: field_obj, ..
                        } = &**object
                        {
                            if matches!(&**field_obj, Expression::Identifier { name: id, .. } if id == "self")
                            {
                                // Check if any argument uses the parameter DIRECTLY
                                for (_label, arg) in arguments {
                                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }

                        // Also check for method calls on local variables: vec.push(param)
                        // This catches cases like store_path(paths, path) where paths.push(path)
                        if let Expression::Identifier { .. } = &**object {
                            for (_label, arg) in arguments {
                                if matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                                {
                                    return true;
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
                _ => {}
            }
        }
        false
    }

    fn is_used_in_binary_op(&self, name: &str, statements: &[Statement]) -> bool {
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
                if self.expr_is_identifier(name, left) || self.expr_is_identifier(name, right) {
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
            _ => false,
        }
    }

    /// Check if a parameter is pattern matched with field extraction
    /// e.g., `match param { Enum::Variant { field: f } => ... }`
    /// If we borrow the parameter, `f` becomes a reference, breaking calls expecting owned values
    fn is_pattern_matched_with_fields(&self, name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                #[allow(clippy::collapsible_match)]
                Statement::Match { value, arms, .. } => {
                    // Check if the match value is the parameter
                    if let Expression::Identifier { name: id, .. } = value {
                        if id == name {
                            // Check if any arm has a pattern with field bindings
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
                    if self.is_pattern_matched_with_fields(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_pattern_matched_with_fields(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    if self.is_pattern_matched_with_fields(name, body) {
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

    fn expr_is_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } if id == name => true,
            // CRITICAL FIX: FieldAccess is NOT the same as the identifier!
            // `self.field` is a field access, NOT the identifier `self`
            // We should NOT treat `self.field` as "using self in binary op"
            // This was causing methods that just read self.field in arithmetic
            // to be incorrectly inferred as needing owned `self`
            Expression::FieldAccess { .. } => false,
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
                    // THE WINDJAMMER WAY: Respect explicit ownership for non-Copy types
                    // If a parameter is explicitly declared as `String` (not `&string`),
                    // it should remain Owned, not be auto-borrowed just because it's read-only.
                    // This respects the user's explicit API contract.
                    //
                    // Only apply inference for implicitly-typed parameters.
                    match &param.type_ {
                        Type::String
                        | Type::Vec(_)
                        | Type::Array(_, _)
                        | Type::Custom(_)
                        | Type::Parameterized(_, _) => {
                            // Explicitly owned types should stay owned unless mutated
                            if inferred == OwnershipMode::MutBorrowed {
                                OwnershipMode::MutBorrowed
                            } else {
                                OwnershipMode::Owned
                            }
                        }
                        _ => inferred,
                    }
                }
            })
            .collect();

        // Check if first parameter is self
        let has_self_receiver = func
            .decl
            .parameters
            .first()
            .map(|p| p.name == "self" || p.name == "mut self")
            .unwrap_or(false);

        // THE WINDJAMMER WAY: Respect explicit type annotations
        // When a user writes `text: string`, they mean `String` (owned).
        // Do NOT auto-convert to `&str` - that's too aggressive and breaks API contracts.
        //
        // Smart inference should only apply to:
        // - Local variables (not parameters)
        // - Return types (when optimizing)
        //
        // For parameters, the user's explicit type annotation is the contract.
        let param_types: Vec<Type> = func
            .decl
            .parameters
            .iter()
            .map(|param| {
                // Respect explicit type annotations - NO smart inference for parameters
                param.type_.clone()
            })
            .collect();

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
    /// Generic type parameters are typically single uppercase letters or uppercase with numbers.
    /// This matches the logic in codegen/rust/generator.rs is_generic_type().
    fn is_generic_type_param(ty: &Type) -> bool {
        if let Type::Custom(name) = ty {
            // Generic type parameters are uppercase letters, possibly followed by numbers
            // Examples: T, G, S, T1, T2, KEY, VALUE
            name.chars().next().is_some_and(|c| c.is_uppercase())
                && (name.len() == 1 || name.chars().all(|c| c.is_uppercase() || c.is_numeric()))
        } else {
            false
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
                // Recognize Rust primitive types by name
                matches!(
                    name.as_str(),
                    // Primitives (the only truly hardcoded types)
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
                        // Common game math types that are always Copy
                        | "Vec2"
                        | "Vec3"
                        | "Vec4"
                        | "Color"
                        | "Rect"
                        | "Point"
                        | "Size"
                        | "Transform2D"
                        | "Matrix4"
                        | "Quaternion"
                )
            }
            _ => false,
        }
    }

    /// PHASE 2 OPTIMIZATION: Detect unnecessary .clone() calls
    /// Returns a list of clones that can be optimized away
    fn detect_unnecessary_clones(&self, func: &FunctionDecl) -> Vec<CloneOptimization> {
        let mut optimizations = Vec::new();

        // Track variable usage: (variable_name, (read_count, write_count, escapes, in_loop))
        let mut usage: HashMap<String, (usize, usize, bool, bool)> = HashMap::new();

        // First pass: analyze usage patterns
        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_clones(stmt, &mut usage, idx);
        }

        // Second pass: identify unnecessary clones
        for (var_name, (reads, writes, escapes, in_loop)) in usage {
            // NEVER optimize away clones for variables used in loops
            // Each loop iteration needs its own copy
            if in_loop {
                continue;
            }

            // Clone is unnecessary if:
            // 1. Variable is only read (never written) AND not in loop -> can use borrow
            if writes == 0 && !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name.clone(),
                    location: 0, // TODO: track actual location
                    reason: CloneEliminationReason::OnlyRead,
                });
            }
            // 2. Variable is used once and doesn't escape AND not in loop -> can move
            else if reads == 1 && writes == 0 && !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name.clone(),
                    location: 0,
                    reason: CloneEliminationReason::SingleUse,
                });
            }
        }

        optimizations
    }

    /// PHASE 3 OPTIMIZATION: Detect struct mapping opportunities
    /// Identifies patterns where struct literals can be optimized
    fn detect_struct_mappings(&self, func: &FunctionDecl) -> Vec<StructMappingOptimization> {
        let mut optimizations = Vec::new();

        // Scan function body for struct literal expressions
        for stmt in &func.body {
            self.analyze_statement_for_struct_mappings(stmt, &mut optimizations);
        }

        optimizations
    }

    /// Helper to analyze statements for struct mapping patterns
    fn analyze_statement_for_struct_mappings(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<StructMappingOptimization>,
    ) {
        match stmt {
            Statement::Let { value, .. }
            | Statement::Return {
                value: Some(value), ..
            } => {
                self.analyze_expression_for_struct_mappings(value, optimizations);
            }
            Statement::Expression { expr, .. } => {
                self.analyze_expression_for_struct_mappings(expr, optimizations);
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for s in then_block {
                    self.analyze_statement_for_struct_mappings(s, optimizations);
                }
                if let Some(else_b) = else_block {
                    for s in else_b {
                        self.analyze_statement_for_struct_mappings(s, optimizations);
                    }
                }
            }
            _ => {}
        }
    }

    /// Analyze an expression for struct mapping opportunities
    fn analyze_expression_for_struct_mappings(
        &self,
        expr: &Expression,
        optimizations: &mut Vec<StructMappingOptimization>,
    ) {
        match expr {
            Expression::StructLiteral { name, fields, .. } => {
                // Detect patterns:
                // 1. All fields come from a single source (direct mapping)
                // 2. Fields extracted from database row (FromRow pattern)
                // 3. Builder pattern (chained method calls)

                let mut field_mappings = Vec::new();
                let mut source_candidates = HashMap::new();

                for (field_name, field_expr) in fields {
                    let field_source = self.extract_field_source(field_expr);
                    field_mappings
                        .push((field_name.clone(), self.expression_to_string(field_expr)));

                    // Track which variables are used as field sources
                    if let Some(src) = &field_source {
                        *source_candidates.entry(src.clone()).or_insert(0) += 1;
                    }
                }

                // Determine optimization strategy
                let strategy = if let Some((dominant_source, count)) =
                    source_candidates.iter().max_by_key(|(_, c)| *c)
                {
                    if *count == fields.len() {
                        // All fields from same source -> DirectMapping
                        MappingStrategy::DirectMapping
                    } else if dominant_source == "row" || dominant_source.starts_with("row.") {
                        // Database row extraction
                        MappingStrategy::FromRow
                    } else {
                        // Mixed sources, use type conversion
                        MappingStrategy::TypeConversion
                    }
                } else {
                    // No clear source pattern
                    MappingStrategy::TypeConversion
                };

                // Only optimize if we have a clear source
                if !source_candidates.is_empty() {
                    let source = source_candidates
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());

                    optimizations.push(StructMappingOptimization {
                        target_struct: name.clone(),
                        source,
                        field_mappings,
                        strategy,
                    });
                }
            }
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                // Check arguments for struct literals
                for (_, arg) in arguments {
                    self.analyze_expression_for_struct_mappings(arg, optimizations);
                }
            }
            _ => {}
        }
    }

    /// Extract the source variable/expression from a field expression
    fn extract_field_source(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => {
                // Extract base object
                if let Expression::Identifier { name, .. } = &**object {
                    Some(name.clone())
                } else {
                    None
                }
            }
            Expression::MethodCall { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    Some(name.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert expression to string for field mapping tracking
    #[allow(clippy::only_used_in_recursion)]
    fn expression_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier { name, .. } => name.clone(),
            Expression::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.expression_to_string(object), field)
            }
            Expression::MethodCall { object, method, .. } => {
                format!("{}.{}()", self.expression_to_string(object), method)
            }
            Expression::Literal { value: lit, .. } => format!("{:?}", lit),
            _ => "expr".to_string(),
        }
    }

    /// PHASE 5 OPTIMIZATION: Detect assignment operations (x = x + 1 → x += 1)
    fn detect_assignment_optimizations(&self, func: &FunctionDecl) -> Vec<AssignmentOptimization> {
        let mut optimizations = Vec::new();

        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_assignments(stmt, &mut optimizations, idx);
        }

        optimizations
    }

    #[allow(clippy::only_used_in_recursion)]
    fn analyze_statement_for_assignments(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<AssignmentOptimization>,
        idx: usize,
    ) {
        match stmt {
            Statement::Assignment {
                target: Expression::Identifier { name: var_name, .. },
                value:
                    Expression::Binary {
                        left, right: _, op, ..
                    },
                ..
            } => {
                // Check if it's pattern: x = x op y
                if let Expression::Identifier { name: left_var, .. } = &**left {
                    if left_var == var_name {
                        // Pattern matched: x = x op y
                        let compound_op = match op {
                            BinaryOp::Add => Some(CompoundOp::AddAssign),
                            BinaryOp::Sub => Some(CompoundOp::SubAssign),
                            BinaryOp::Mul => Some(CompoundOp::MulAssign),
                            BinaryOp::Div => Some(CompoundOp::DivAssign),
                            _ => None,
                        };

                        if let Some(operation) = compound_op {
                            optimizations.push(AssignmentOptimization {
                                variable: var_name.clone(),
                                location: idx,
                                operation,
                            });
                        }
                    }
                }
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for stmt in then_block {
                    self.analyze_statement_for_assignments(stmt, optimizations, idx);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        self.analyze_statement_for_assignments(stmt, optimizations, idx);
                    }
                }
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => {
                for stmt in body {
                    self.analyze_statement_for_assignments(stmt, optimizations, idx);
                }
            }
            Statement::For { body, .. } => {
                for stmt in body {
                    self.analyze_statement_for_assignments(stmt, optimizations, idx);
                }
            }
            _ => {}
        }
    }

    /// PHASE 4 OPTIMIZATION: Detect string operation opportunities
    /// Identifies patterns where string operations can be optimized
    fn detect_string_optimizations(&self, func: &FunctionDecl) -> Vec<StringOptimization> {
        let mut optimizations = Vec::new();

        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_string_ops(stmt, &mut optimizations, idx);
        }

        optimizations
    }

    /// Analyze a statement for string optimization opportunities
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_statement_for_string_ops(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<StringOptimization>,
        idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. }
            | Statement::Return {
                value: Some(value), ..
            } => {
                // Check for format! macro calls (string interpolation is converted to format!)
                if let Expression::MacroInvocation { name, .. } = value {
                    if name == "format" {
                        // String interpolation detected - could pre-allocate capacity
                        optimizations.push(StringOptimization {
                            optimization_type: StringOptimizationType::InterpolationWithCapacity,
                            estimated_capacity: Some(64), // Default estimate
                            location: idx,
                        });
                    }
                }
            }
            // Recursively analyze nested blocks
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for nested_stmt in then_block {
                    self.analyze_statement_for_string_ops(nested_stmt, optimizations, idx);
                }
                if let Some(else_b) = else_block {
                    for nested_stmt in else_b {
                        self.analyze_statement_for_string_ops(nested_stmt, optimizations, idx);
                    }
                }
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body, .. } => {
                for nested_stmt in body {
                    self.analyze_statement_for_string_ops(nested_stmt, optimizations, idx);
                }
            }
            _ => {}
        }
    }

    /// Detect concatenation chains (a + b + c + ...)
    #[allow(dead_code)] // TODO: Implement concatenation optimization in future version
    fn detect_concatenation_chain(
        &self,
        expr: &Expression,
        optimizations: &mut Vec<StringOptimization>,
        idx: usize,
    ) {
        let mut concat_count = 0;
        self.count_concatenations(expr, &mut concat_count);

        if concat_count >= 3 {
            // Multiple concatenations, could benefit from pre-allocation
            optimizations.push(StringOptimization {
                optimization_type: StringOptimizationType::ConcatenationChain,
                estimated_capacity: Some(concat_count * 32), // Rough estimate
                location: idx,
            });
        }
    }

    /// Count the number of concatenation operations
    #[allow(dead_code)] // TODO: Implement concatenation optimization in future version
    #[allow(clippy::only_used_in_recursion)]
    fn count_concatenations(&self, expr: &Expression, count: &mut usize) {
        if let Expression::Binary {
            op, left, right, ..
        } = expr
        {
            if matches!(op, BinaryOp::Add) {
                *count += 1;
                self.count_concatenations(left, count);
                self.count_concatenations(right, count);
            }
        }
    }

    /// Check if a statement is accumulating strings (s += ...)
    #[allow(dead_code)] // TODO: Implement loop accumulation optimization in future version
    fn is_string_accumulation(&self, stmt: &Statement) -> bool {
        matches!(
            stmt,
            Statement::Assignment {
                target: Expression::Identifier { .. },
                ..
            }
        )
    }

    /// Helper to analyze a statement for clone patterns
    fn analyze_statement_for_clones(
        &self,
        stmt: &Statement,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
        _idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. } => {
                self.analyze_expression_for_clones(value, usage);
            }
            Statement::Assignment { target, value, .. } => {
                // Track writes
                if let Expression::Identifier { name, .. } = target {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.1 += 1; // increment write count
                }
                self.analyze_expression_for_clones(value, usage);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                // Returned values escape the function
                if let Expression::Identifier { name, .. } = expr {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.2 = true; // mark as escapes
                }
                self.analyze_expression_for_clones(expr, usage);
            }
            Statement::Expression { expr, .. } => {
                self.analyze_expression_for_clones(expr, usage);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression_for_clones(condition, usage);
                for stmt in then_block {
                    self.analyze_statement_for_clones(stmt, usage, _idx);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        self.analyze_statement_for_clones(stmt, usage, _idx);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.analyze_expression_for_clones(condition, usage);
                // Mark all variables used in loop body as in_loop
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.analyze_expression_for_clones(iterable, usage);
                // Mark all variables used in loop body as in_loop
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::Loop { body, .. } => {
                // Mark all variables used in loop body as in_loop
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            _ => {}
        }
    }

    /// Helper to analyze a statement in loop context (marks variables as in_loop)
    fn analyze_statement_for_clones_in_loop(
        &self,
        stmt: &Statement,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
        _idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. } => {
                self.analyze_expression_for_clones_in_loop(value, usage);
            }
            Statement::Assignment { target, value, .. } => {
                if let Expression::Identifier { name, .. } = target {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.1 += 1; // increment write count
                    entry.3 = true; // mark as in_loop
                }
                self.analyze_expression_for_clones_in_loop(value, usage);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                if let Expression::Identifier { name, .. } = expr {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.2 = true; // mark as escapes
                    entry.3 = true; // mark as in_loop
                }
                self.analyze_expression_for_clones_in_loop(expr, usage);
            }
            Statement::Expression { expr, .. } => {
                self.analyze_expression_for_clones_in_loop(expr, usage);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression_for_clones_in_loop(condition, usage);
                for stmt in then_block {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.analyze_expression_for_clones_in_loop(condition, usage);
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.analyze_expression_for_clones_in_loop(iterable, usage);
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            _ => {}
        }
    }

    /// Helper to analyze an expression for variable usage in loop context
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_expression_for_clones_in_loop(
        &self,
        expr: &Expression,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                entry.0 += 1; // increment read count
                entry.3 = true; // mark as in_loop
            }
            Expression::Binary { left, right, .. } => {
                self.analyze_expression_for_clones_in_loop(left, usage);
                self.analyze_expression_for_clones_in_loop(right, usage);
            }
            Expression::Unary { operand, .. } => {
                self.analyze_expression_for_clones_in_loop(operand, usage);
            }
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                for (_, arg) in arguments {
                    self.analyze_expression_for_clones_in_loop(arg, usage);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.analyze_expression_for_clones_in_loop(object, usage);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.analyze_expression_for_clones_in_loop(value, usage);
                }
            }
            Expression::Cast { expr, .. } => {
                self.analyze_expression_for_clones_in_loop(expr, usage);
            }
            _ => {}
        }
    }

    /// Helper to analyze an expression for variable usage
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_expression_for_clones(
        &self,
        expr: &Expression,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                // Track reads
                let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                entry.0 += 1; // increment read count
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.analyze_expression_for_clones(object, usage);
                for (_, arg) in arguments {
                    self.analyze_expression_for_clones(arg, usage);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.analyze_expression_for_clones(function, usage);
                for (_, arg) in arguments {
                    self.analyze_expression_for_clones(arg, usage);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.analyze_expression_for_clones(left, usage);
                self.analyze_expression_for_clones(right, usage);
            }
            Expression::Unary { operand, .. } => {
                self.analyze_expression_for_clones(operand, usage);
            }
            Expression::FieldAccess { object, .. } => {
                self.analyze_expression_for_clones(object, usage);
            }
            Expression::Index { object, index, .. } => {
                self.analyze_expression_for_clones(object, usage);
                self.analyze_expression_for_clones(index, usage);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    self.analyze_expression_for_clones(field_expr, usage);
                }
            }
            Expression::Cast { expr, .. } => {
                self.analyze_expression_for_clones(expr, usage);
            }
            _ => {}
        }
    }
    /// PHASE 6: Detect defer drop optimization opportunities
    /// This detects when a function owns large data structures and returns small values,
    /// allowing us to defer the drop to a background thread for 10,000x speedup.
    /// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
    fn detect_defer_drop_opportunities(&self, func: &FunctionDecl) -> Vec<DeferDropOptimization> {
        let mut optimizations = Vec::new();

        // Pattern 1: Large owned parameter → small return value
        for param in &func.parameters {
            // Check if parameter is owned
            let ownership = match param.ownership {
                OwnershipHint::Ref => OwnershipMode::Borrowed,
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Owned => OwnershipMode::Owned,
                OwnershipHint::Inferred => {
                    // Infer ownership if not specified
                    self.infer_parameter_ownership(
                        &param.name,
                        &param.type_,
                        &func.body,
                        &func.return_type,
                    )
                    .unwrap_or(OwnershipMode::Owned)
                }
            };

            if ownership == OwnershipMode::Owned {
                let param_size = self.estimate_type_size(&param.type_);

                // Only consider large types
                if matches!(param_size, EstimatedSize::Large | EstimatedSize::VeryLarge) {
                    // Check if return type is small
                    if let Some(ref ret_type) = func.return_type {
                        if self.is_small_type(ret_type) {
                            // Check if it's safe to defer
                            if self.is_safe_to_defer(&param.type_) {
                                optimizations.push(DeferDropOptimization {
                                    variable: param.name.clone(),
                                    estimated_size: param_size,
                                    reason: DeferDropReason::LargeOwnedParameter,
                                    location: func.body.len().saturating_sub(1),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Pattern 2: Large local variable that goes out of scope
        // TODO: Track local variable lifetimes and sizes
        // This would require more sophisticated analysis of let statements and their usage

        optimizations
    }

    /// Estimate the size of a type for defer drop optimization
    fn estimate_type_size(&self, ty: &Type) -> EstimatedSize {
        match ty {
            // Collections are potentially large
            Type::Custom(name) if name.contains("HashMap") => EstimatedSize::Large,
            Type::Custom(name) if name.contains("BTreeMap") => EstimatedSize::Large,
            Type::Custom(name) if name.contains("HashSet") => EstimatedSize::Large,
            Type::Custom(name) if name.contains("BTreeSet") => EstimatedSize::Large,
            Type::Parameterized(name, _) if name.contains("HashMap") => EstimatedSize::Large,
            Type::Parameterized(name, _) if name.contains("BTreeMap") => EstimatedSize::Large,
            Type::Parameterized(name, _) if name.contains("HashSet") => EstimatedSize::Large,
            Type::Parameterized(name, _) if name.contains("BTreeSet") => EstimatedSize::Large,
            Type::Parameterized(name, _) if name.contains("Vec") => EstimatedSize::Medium,
            Type::Parameterized(name, _) if name.contains("VecDeque") => EstimatedSize::Medium,
            Type::Vec(_) => EstimatedSize::Medium,
            Type::String => EstimatedSize::Medium,

            // User-defined structs - conservative estimate (Medium)
            Type::Custom(_) => EstimatedSize::Medium,

            // Small primitive types
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => EstimatedSize::Small,
            Type::Reference(_) => EstimatedSize::Small, // References are just pointers
            Type::MutableReference(_) => EstimatedSize::Small,

            _ => EstimatedSize::Small,
        }
    }

    /// Check if a type is small (return value size check)
    fn is_small_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
        ) || matches!(ty, Type::Custom(name) if name == "usize" || name == "isize")
            || matches!(ty, Type::Reference(_) | Type::MutableReference(_))
    }

    /// Check if it's safe to defer dropping this type
    /// Must be Send (can move to another thread) and have no important Drop side effects
    fn is_safe_to_defer(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) | Type::Parameterized(name, _) => {
                // Types with important Drop implementations - DO NOT defer
                if name.contains("Mutex")
                    || name.contains("RwLock")
                    || name.contains("File")
                    || name.contains("TcpStream")
                    || name.contains("UdpSocket")
                    || name.contains("Channel")
                    || name.contains("Receiver")
                    || name.contains("Sender")
                    || name.contains("JoinHandle")
                {
                    return false;
                }

                // Standard collections are safe to defer
                if name.contains("HashMap")
                    || name.contains("BTreeMap")
                    || name.contains("HashSet")
                    || name.contains("BTreeSet")
                    || name.contains("Vec")
                    || name.contains("VecDeque")
                    || name.contains("String")
                {
                    return true;
                }

                // User-defined types - conservatively assume safe for now
                // TODO: Add more sophisticated analysis or user annotations
                true
            }
            Type::Vec(_) | Type::String => true, // Built-in collections are safe
            _ => false, // Primitives and references don't benefit from defer drop
        }
    }

    /// PHASE 7: Detect const/static optimization opportunities
    /// Returns variables/constants within a function that can be promoted to const
    fn detect_const_static_opportunities(
        &self,
        _func: &AnalyzedFunction,
    ) -> Vec<ConstStaticOptimization> {
        // For now, we focus on global static analysis (done in analyze_program)
        // Function-level const detection would look for:
        // 1. Local variables initialized with const-evaluable expressions
        // 2. Static local variables that never change
        // 3. Repeated literal values that could be extracted to const

        // TODO: Implement function-level const detection
        // This requires analyzing the function body's statements and expressions

        Vec::new()
    }

    /// Check if an expression can be evaluated at compile time (const-evaluable)
    #[allow(clippy::only_used_in_recursion)]
    fn is_const_evaluable(&self, expr: &Expression) -> bool {
        match expr {
            // Literals are always const
            Expression::Literal { .. } => true,

            // Binary operations on const values are const
            Expression::Binary { left, right, .. } => {
                self.is_const_evaluable(left) && self.is_const_evaluable(right)
            }

            // Unary operations on const values are const
            Expression::Unary { operand, .. } => self.is_const_evaluable(operand),

            // Struct literals with const fields might be const (depends on struct)
            Expression::StructLiteral { fields, .. } => {
                fields.iter().all(|(_, expr)| self.is_const_evaluable(expr))
            }

            // References to other const values would be const (requires symbol table)
            // For now, we're conservative and don't allow this
            Expression::Identifier { .. } => false,

            // Function calls are generally not const (unless const fn, which we don't track yet)
            Expression::Call { .. } => false,

            // Field access could be const if the base is const, but we're conservative
            Expression::FieldAccess { .. } => false,

            // Method calls are not const
            Expression::MethodCall { .. } => false,

            // Everything else is not const
            _ => false,
        }
    }

    /// PHASE 8: Detect SmallVec optimization opportunities
    /// Returns Vec variables that can use stack allocation via SmallVec
    fn detect_smallvec_opportunities(&self, func: &FunctionDecl) -> Vec<SmallVecOptimization> {
        let mut optimizations = Vec::new();

        // TODO: Implement full SmallVec detection
        // This requires analyzing:
        // 1. Vec literal sizes: vec![1, 2, 3] → size 3
        // 2. Loop bounds: (0..n).collect() where n is const → size n
        // 3. Multiple push() calls → count them
        // 4. Usage patterns to ensure size stays small

        // For now, detect obvious cases: vec![...] literals with ≤ 8 elements
        for stmt in &func.body {
            self.detect_smallvec_in_statement(stmt, &mut optimizations);
        }

        optimizations
    }

    fn detect_smallvec_in_statement(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<SmallVecOptimization>,
    ) {
        if let Statement::Let {
            pattern: Pattern::Identifier(name),
            value,
            ..
        } = stmt
        {
            if let Some(size) = self.estimate_vec_literal_size(value) {
                if size <= 8 {
                    // Recommend SmallVec with power-of-2 stack size
                    let stack_size = size.next_power_of_two().max(4);
                    optimizations.push(SmallVecOptimization {
                        variable: name.clone(),
                        estimated_max_size: size,
                        stack_size,
                    });
                }
            }
        }
    }

    /// Estimate the size of a Vec literal or similar construction
    fn estimate_vec_literal_size(&self, expr: &Expression) -> Option<usize> {
        match expr {
            // vec![1, 2, 3] macro invocation
            Expression::MacroInvocation {
                name,
                args,
                delimiter,
                ..
            } if name == "vec" && *delimiter == MacroDelimiter::Brackets => Some(args.len()),

            // Vec::new() - starts empty
            // IMPORTANT: Only match if the object is actually "Vec", not any arbitrary type
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } if method == "new" && arguments.is_empty() => {
                // Check if the object is an identifier named "Vec"
                if let Expression::Identifier { name, .. } = object.as_ref() {
                    if name == "Vec" {
                        return Some(0);
                    }
                }
                None
            }

            // Static method Vec::<T>::with_capacity(n) where n is a literal
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Check if it's Vec::with_capacity or similar
                if let Expression::FieldAccess { object, field, .. } = function.as_ref() {
                    // Ensure the object is "Vec"
                    if let Expression::Identifier { name, .. } = object.as_ref() {
                        if name == "Vec" && field == "with_capacity" {
                            // Try to extract capacity from first argument
                            if let Some((_, arg)) = arguments.first() {
                                return self.extract_literal_int(arg);
                            }
                        }
                    }
                }
                None
            }

            // (0..n).collect::<Vec<_>>() patterns
            Expression::MethodCall { object, method, .. } if method == "collect" => {
                // Check if object is a Range
                if let Expression::Range { start, end, .. } = object.as_ref() {
                    // Try to compute range size
                    let start_val = self.extract_literal_int(start).unwrap_or(0);
                    let end_val = self.extract_literal_int(end)?;
                    return Some(end_val - start_val);
                }
                None
            }

            _ => None,
        }
    }

    /// Extract an integer literal value from an expression
    fn extract_literal_int(&self, expr: &Expression) -> Option<usize> {
        match expr {
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } if *n >= 0 => Some(*n as usize),
            _ => None,
        }
    }

    /// PHASE 9: Detect Cow (Clone-on-Write) optimization opportunities
    /// Returns parameters/variables that can use Cow to avoid unnecessary clones
    fn detect_cow_opportunities(&self, func: &FunctionDecl) -> Vec<CowOptimization> {
        let mut optimizations = Vec::new();

        // Analyze function parameters that might be conditionally modified
        for param in &func.parameters {
            // Check if parameter is String or str (common Cow candidates)
            let is_string_like = matches!(param.type_, Type::String)
                || matches!(param.type_, Type::Reference(ref inner) if matches!(**inner, Type::String));

            if !is_string_like {
                continue;
            }

            // Analyze if the parameter is conditionally modified
            if let Some(reason) = self.analyze_conditional_modification(&param.name, &func.body) {
                optimizations.push(CowOptimization {
                    variable: param.name.clone(),
                    reason,
                });
            }
        }

        optimizations
    }

    /// Analyze if a variable is conditionally modified (some branches modify, others don't)
    fn analyze_conditional_modification(
        &self,
        var_name: &str,
        body: &[Statement],
    ) -> Option<CowReason> {
        let mut has_read_only_path = false;
        let mut has_modifying_path = false;

        for stmt in body {
            match stmt {
                // Check if statements
                Statement::If {
                    condition: _,
                    then_block,
                    else_block,
                    ..
                } => {
                    // Check if variable is modified in then block
                    let modified_in_then = self.is_variable_modified(var_name, then_block);
                    let modified_in_else = else_block
                        .as_ref()
                        .map(|block| self.is_variable_modified(var_name, block))
                        .unwrap_or(false);

                    // XOR: exactly one branch modifies
                    if modified_in_then != modified_in_else {
                        has_read_only_path = true;
                        has_modifying_path = true;
                    } else if !modified_in_then {
                        // Neither modifies - read only
                        has_read_only_path = true;
                    } else {
                        // Both modify
                        has_modifying_path = true;
                    }
                }

                // Check match statements
                Statement::Match { value: _, arms, .. } => {
                    // For match expressions, check if the variable is referenced in any arm
                    // Full analysis would require checking if arms modify vs just read
                    // For now, consider it a potential read-only use
                    for arm in arms {
                        if self.expression_references_variable(var_name, &arm.body) {
                            has_read_only_path = true;
                        }
                    }
                }

                // Check if variable is used in a read-only way
                Statement::Expression { expr, .. }
                | Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expression_references_variable(var_name, expr) {
                        // Simple use - consider it read-only unless it's being modified
                        has_read_only_path = true;
                    }
                }

                _ => {}
            }
        }

        // If we have both read-only and modifying paths, Cow is beneficial
        if has_read_only_path && has_modifying_path {
            Some(CowReason::ConditionalModification)
        } else {
            None
        }
    }

    /// Check if a variable is modified in a block of statements
    fn is_variable_modified(&self, var_name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                // Assignment to the variable
                Statement::Assignment {
                    target: Expression::Identifier { name, .. },
                    ..
                } if name == var_name => {
                    return true;
                }

                // Method calls that might modify (e.g., push_str, clear)
                Statement::Expression {
                    expr: Expression::MethodCall { object, method, .. },
                    ..
                } => {
                    if let Expression::Identifier { name, .. } = object.as_ref() {
                        if name == var_name && self.is_mutating_method(method) {
                            return true;
                        }
                    }
                }

                _ => {}
            }
        }
        false
    }

    /// Check if a method mutates the object
    fn is_mutating_method(&self, method: &str) -> bool {
        matches!(
            method,
            "push"
                | "push_str"
                | "clear"
                | "pop"
                | "remove"
                | "insert"
                | "append"
                | "get_mut"
                | "allocate"
                | "free"
                | "update"
                | "play"
                | "reset"
        )
    }

    /// Check if a function modifies self fields (for impl methods)
    fn function_modifies_self_fields(&self, func: &FunctionDecl) -> bool {
        // THE WINDJAMMER WAY: Check ALL cases that require &mut self

        // Case 1: Return type is &mut T (requires &mut self)
        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        // Case 2: Function calls other methods on self that need &mut self
        if self.function_calls_mutating_self_methods(func) {
            return true;
        }

        // Case 3: Function modifies self fields directly
        for stmt in &func.body {
            if self.statement_modifies_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a function modifies self fields WITHOUT checking for method calls
    /// (prevents infinite recursion when analyzing cross-method dependencies)
    fn function_modifies_self_fields_recursive(&self, func: &FunctionDecl) -> bool {
        // Case 1: Return type is &mut T (requires &mut self)
        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        // Case 2: Function modifies self fields directly
        for stmt in &func.body {
            if self.statement_modifies_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a type contains a mutable reference (&mut T)
    /// This includes Option<&mut T>, Result<&mut T, E>, Vec<&mut T>, etc.
    fn type_is_mut_ref(&self, ty: &Type) -> bool {
        match ty {
            Type::MutableReference(_) => true,
            Type::Option(inner) | Type::Vec(inner) | Type::Reference(inner) => {
                self.type_is_mut_ref(inner)
            }
            Type::Result(ok, err) => self.type_is_mut_ref(ok) || self.type_is_mut_ref(err),
            Type::Tuple(types) => types.iter().any(|t| self.type_is_mut_ref(t)),
            Type::Parameterized(_, args) => args.iter().any(|t| self.type_is_mut_ref(t)),
            _ => false,
        }
    }

    /// Check if function calls methods on self that require &mut self
    fn function_calls_mutating_self_methods(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_calls_mutating_self_methods(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if statement calls methods on self that require &mut self
    fn statement_calls_mutating_self_methods(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expression_calls_mutating_self_methods(expr),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_calls_mutating_self_methods(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_calls_mutating_self_methods(s))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_calls_mutating_self_methods(s)),
            _ => false,
        }
    }

    /// Check if expression calls methods on self that require &mut self
    fn expression_calls_mutating_self_methods(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                // Check if calling a method on self (not self.field, just self)
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        // THE WINDJAMMER WAY: Check if this method requires &mut self
                        // 1. Check hardcoded stdlib mutating methods
                        if self.is_mutating_method(method) {
                            return true;
                        }

                        // 2. Check methods in current impl block
                        if let Some(impl_functions) = &self.current_impl_functions {
                            if let Some(called_func) = impl_functions.get(method) {
                                // Check if the called method modifies self fields
                                // or returns &mut T (which requires &mut self)
                                if self.function_modifies_self_fields_recursive(called_func) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_calls_mutating_self_methods(s)),
            _ => false,
        }
    }

    /// Check if a statement modifies self fields
    fn statement_modifies_self_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if target is self.field OR self.field[index]
                self.expression_is_self_field_access(target)
                    || self.expression_is_self_field_index_access(target)
            }
            Statement::Expression { expr, .. } => {
                // Check for mutating method calls on self.field
                self.expression_mutates_self_fields(expr)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // THE WINDJAMMER WAY: Check condition for mutations!
                // if let Some(x) = self.field.get_mut() requires &mut self
                self.expression_mutates_self_fields(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_modifies_self_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_modifies_self_fields(s))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                body.iter().any(|s| self.statement_modifies_self_fields(s))
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Check match value for mutations!
                // match self.field.get_mut() requires &mut self
                self.expression_mutates_self_fields(value)
                    || arms.iter().any(|arm| {
                        // Match arms have an expression body, check if it contains modifications
                        self.expression_contains_self_field_mutations(&arm.body)
                    })
            }
            Statement::Return { value, .. } => {
                // Check if the return expression contains mutations of self fields
                // e.g., return self.allocator.allocate()
                value
                    .as_ref()
                    .is_some_and(|expr| self.expression_mutates_self_fields(expr))
            }
            Statement::Let { value, .. } => {
                // Check if the let binding contains mutations of self fields
                // e.g., let x = self.allocator.allocate()
                self.expression_mutates_self_fields(value)
            }
            _ => false,
        }
    }

    /// Check if an expression contains self field mutations (for match arms and blocks)
    fn expression_contains_self_field_mutations(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_modifies_self_fields(s)),
            Expression::MethodCall { object, method, .. } => {
                // Check if this is a mutating method call on a self field
                self.expression_is_self_field_access(object) && self.is_mutating_method(method)
            }
            _ => false,
        }
    }

    /// Check if expression is a self field access (self.field)
    fn expression_is_self_field_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                // Check if the object is 'self' OR if it's a nested field access starting with 'self'
                match &**object {
                    Expression::Identifier { name, .. } if name == "self" => true,
                    Expression::FieldAccess { .. } => self.expression_is_self_field_access(object),
                    // CRITICAL FIX: Handle index expressions like self.children[i].field
                    // The object of the field access is an Index, which itself may be a self field access
                    Expression::Index { .. } => self.expression_is_self_field_index_access(object),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Check if expression is an index access on a self field (self.field[index] or self.field[i][j])
    fn expression_is_self_field_index_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Index { object, .. } => {
                // Check if the object being indexed is a self field access
                // OR recursively check if it's a nested index access (self.field[i][j])
                self.expression_is_self_field_access(object)
                    || self.expression_is_self_field_index_access(object)
            }
            _ => false,
        }
    }

    /// Check if expression mutates self fields
    fn expression_mutates_self_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if self.expression_is_self_field_access(object) && self.is_mutating_method(method) {
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Check if a function returns Self (for builder pattern detection)
    fn function_returns_self(&self, func: &FunctionDecl) -> bool {
        use crate::parser::{Statement, Type};

        // First check if return type is a custom type (struct type)
        let return_type_name = match &func.return_type {
            Some(Type::Custom(name)) => name,
            _ => return false,
        };

        // Now check if the function body actually returns `self` or a struct literal of the same type
        // Check the last statement in the body
        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Explicit return self or struct literal
                    self.expression_returns_self_type(expr, return_type_name)
                }
                Statement::Expression { expr, .. } => {
                    // Implicit return self or struct literal (last expression)
                    self.expression_returns_self_type(expr, return_type_name)
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check if an expression returns the Self type (either `self` or a struct literal of the same type)
    fn expression_returns_self_type(&self, expr: &Expression, type_name: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { name, .. } if name == type_name => true,
            _ => false,
        }
    }

    /// Check if a function uses a specific identifier (e.g., "self")
    /// This is used for auto-self inference.
    fn function_uses_identifier(&self, name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            if self.statement_uses_identifier(name, stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement uses a specific identifier
    fn statement_uses_identifier(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expression_uses_identifier(name, expr),
            Statement::Let { value, .. } => self.expression_uses_identifier(name, value),
            Statement::Assignment { target, value, .. } => {
                self.expression_uses_identifier(name, target)
                    || self.expression_uses_identifier(name, value)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_uses_identifier(name, expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_uses_identifier(name, condition)
                    || self.function_uses_identifier(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| self.function_uses_identifier(name, block))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_uses_identifier(name, condition)
                    || self.function_uses_identifier(name, body)
            }
            Statement::For { iterable, body, .. } => {
                self.expression_uses_identifier(name, iterable)
                    || self.function_uses_identifier(name, body)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_uses_identifier(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expression_uses_identifier(name, &arm.body))
            }
            _ => false,
        }
    }

    /// Check if an expression uses a specific identifier
    fn expression_uses_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expression_uses_identifier(name, object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_uses_identifier(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_uses_identifier(name, arg))
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_uses_identifier(name, arg)),
            Expression::Binary { left, right, .. } => {
                self.expression_uses_identifier(name, left)
                    || self.expression_uses_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expression_uses_identifier(name, operand),
            Expression::Index { object, index, .. } => {
                self.expression_uses_identifier(name, object)
                    || self.expression_uses_identifier(name, index)
            }
            Expression::Block { statements, .. } => self.function_uses_identifier(name, statements),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_uses_identifier(name, el)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_uses_identifier(name, el)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expression_uses_identifier(name, k) || self.expression_uses_identifier(name, v)
            }),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_uses_identifier(name, v)),
            Expression::Cast { expr, .. } => self.expression_uses_identifier(name, expr),
            Expression::Range { start, end, .. } => {
                self.expression_uses_identifier(name, start)
                    || self.expression_uses_identifier(name, end)
            }
            _ => false,
        }
    }

    /// Check if a function accesses self fields (for impl methods)
    #[allow(dead_code)]
    fn function_accesses_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_accesses_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement accesses self fields
    fn statement_accesses_self_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_accesses_self_fields(expr),
            Statement::Let { value, .. } => self.expression_accesses_self_fields(value),
            Statement::Assignment { target, value, .. } => {
                self.expression_accesses_self_fields(target)
                    || self.expression_accesses_self_fields(value)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_accesses_self_fields(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_accesses_self_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_accesses_self_fields(s))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_accesses_self_fields(condition)
                    || body.iter().any(|s| self.statement_accesses_self_fields(s))
            }
            Statement::For { iterable, body, .. } => {
                self.expression_accesses_self_fields(iterable)
                    || body.iter().any(|s| self.statement_accesses_self_fields(s))
            }
            _ => false,
        }
    }

    /// Check if expression accesses self fields
    #[allow(clippy::only_used_in_recursion)]
    fn expression_accesses_self_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_accesses_self_fields(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_accesses_self_fields(arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expression_accesses_self_fields(left)
                    || self.expression_accesses_self_fields(right)
            }
            Expression::Unary { operand, .. } => self.expression_accesses_self_fields(operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_accesses_self_fields(arg)),
            // CRITICAL: Check macro arguments for self field access
            // This handles format!("...", self.field), println!("{}", self.x), etc.
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|arg| self.expression_accesses_self_fields(arg)),
            // Recurse into tuple elements
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_accesses_self_fields(elem)),
            // Recurse into array elements
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_accesses_self_fields(elem)),
            _ => false,
        }
    }

    /// Check if an expression references a variable
    #[allow(clippy::only_used_in_recursion)]
    fn expression_references_variable(&self, var_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var_name,
            Expression::Binary { left, right, .. } => {
                self.expression_references_variable(var_name, left)
                    || self.expression_references_variable(var_name, right)
            }
            Expression::MethodCall { object, .. } | Expression::FieldAccess { object, .. } => {
                self.expression_references_variable(var_name, object)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expression_references_variable(var_name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_references_variable(var_name, arg))
            }
            _ => false,
        }
    }

    /// Track which local variables are mutated in a function body
    /// This enables automatic `mut` inference - users don't need to write `let mut x`
    pub fn track_mutations(&mut self, statements: &[Statement]) {
        self.mutated_variables.clear();
        self.collect_mutations(statements);
    }

    /// Recursively collect all variable mutations
    fn collect_mutations(&mut self, statements: &[Statement]) {
        for stmt in statements {
            match stmt {
                Statement::Assignment {
                    target: Expression::Identifier { name, .. },
                    ..
                } => {
                    // Track the variable being assigned to
                    self.mutated_variables.insert(name.clone());
                }
                Statement::Assignment { target, .. } => {
                    // Track field mutations (x.field = ...) or index mutations (x[i] = ...)
                    self.collect_mutation_target(target);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.collect_mutations(then_block);
                    if let Some(else_stmts) = else_block {
                        self.collect_mutations(else_stmts);
                    }
                }
                Statement::Match { arms, .. } => {
                    // Match arms have Expression bodies, which may contain blocks
                    // For now, we'll skip mutation tracking in match arms
                    // TODO: Add expression-level mutation tracking
                    let _ = arms; // Suppress unused warning
                }
                Statement::For { pattern, body, .. } => {
                    // Collect mutations in loop body
                    self.collect_mutations(body);

                    // If the loop variable itself is mutated in the body, track it
                    // This helps infer `for x in &mut vec` when x is modified
                    if let Pattern::Identifier(var_name) = pattern {
                        // Check if this variable is mutated in the body
                        if self.is_variable_mutated_in_statements(var_name, body) {
                            // Track that this variable needs mutable access
                            self.mutated_variables
                                .insert(format!("__loop_var_{}", var_name));
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    self.collect_mutations(body);
                }
                // Track method calls that mutate (e.g., vec.push(x))
                Statement::Expression { expr, .. } => {
                    self.collect_mutations_in_expression(expr);
                }
                _ => {}
            }
        }
    }

    /// Track mutations in expressions (method calls that mutate)
    fn collect_mutations_in_expression(&mut self, expr: &Expression) {
        if let Expression::MethodCall { object, method, .. } = expr {
            // Common mutating methods
            let mutating_methods = [
                "push",
                "pop",
                "insert",
                "remove",
                "clear",
                "append",
                "extend",
                "truncate",
                "resize",
                "sort",
                "reverse",
                "dedup",
                "retain",
                "drain",
                "split_off",
                "swap_remove",
            ];

            if mutating_methods.contains(&method.as_str()) {
                // Mark the object as mutated
                if let Expression::Identifier { name, .. } = &**object {
                    self.mutated_variables.insert(name.clone());
                }
            }
        }
    }

    /// Check if a variable is mutated within a specific set of statements
    fn is_variable_mutated_in_statements(&self, var_name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment { target, .. } => {
                    if let Expression::Identifier { name, .. } = target {
                        if name == var_name {
                            return true;
                        }
                    }
                    // Also check for field assignments like power_up.position.y
                    if let Expression::FieldAccess { object, .. } = target {
                        if let Expression::Identifier { name, .. } = &**object {
                            if name == var_name {
                                return true;
                            }
                        }
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    if self.is_variable_mutated_in_statements(var_name, body) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_variable_mutated_in_statements(var_name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_variable_mutated_in_statements(var_name, else_stmts) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a variable is mutated (for automatic mut inference)
    pub fn is_variable_mutated(&self, var_name: &str) -> bool {
        self.mutated_variables.contains(var_name)
    }

    /// Track mutation target (left side of assignment)
    /// x.field = ... means x is mutated
    /// arr[i] = ... means arr is mutated
    fn collect_mutation_target(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier { name, .. } => {
                self.mutated_variables.insert(name.clone());
            }
            Expression::FieldAccess { object, .. } => {
                // x.field = ... means x is mutated
                self.collect_mutation_target(object);
            }
            Expression::Index { object, .. } => {
                // arr[i] = ... means arr is mutated
                self.collect_mutation_target(object);
            }
            _ => {}
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
