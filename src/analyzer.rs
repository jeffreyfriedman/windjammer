// Ownership and borrow checking analyzer
use crate::auto_clone::AutoCloneAnalysis;
use crate::parser::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AnalyzedFunction {
    pub decl: FunctionDecl,
    pub inferred_ownership: HashMap<String, OwnershipMode>,
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
    pub param_ownership: Vec<OwnershipMode>,
    pub return_ownership: OwnershipMode,
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
    // Track which local variables are mutated (for automatic mut inference)
    mutated_variables: HashSet<String>,
}

use std::collections::HashSet;

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        let mut analyzer = Analyzer {
            variables: HashMap::new(),
            copy_enums: HashSet::new(),
            copy_structs: HashSet::new(),
            trait_definitions: HashMap::new(),
            mutated_variables: HashSet::new(),
        };

        // Pre-register standard library traits so the analyzer knows their signatures
        analyzer.register_stdlib_traits();

        analyzer
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
                        },
                        Parameter {
                            name: "rhs".to_string(),
                            pattern: None,
                            type_: Type::Custom("Rhs".to_string()),
                            ownership: OwnershipHint::Owned,
                        },
                    ],
                    return_type: Some(Type::Custom("Output".to_string())),
                    is_async: false,
                    body: None,
                }],
                associated_types: vec![AssociatedType {
                    name: "Output".to_string(),
                    concrete_type: None,
                }],
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

    pub fn analyze_program(
        &mut self,
        program: &Program,
    ) -> Result<(Vec<AnalyzedFunction>, SignatureRegistry), String> {
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
                    if has_copy_derive {
                        eprintln!("DEBUG: Registered Copy struct: {}", decl.name);
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
                            // Regular impl - infer as usual
                            self.analyze_function(func)?
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
                Item::Static { mutable, value, .. } => {
                    // Analyze static declarations for const promotion
                    if !mutable && self.is_const_evaluable(value) {
                        // This static can be promoted to const
                        // Store in a global optimization list (TODO: add to Program-level analysis)
                    }
                }
                _ => {}
            }
        }

        Ok((analyzed, registry))
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
                            // Check if it accesses fields (read-only)
                            let accesses_fields = self.function_accesses_self_fields(func);
                            if accesses_fields {
                                OwnershipMode::Borrowed
                            } else {
                                OwnershipMode::Owned
                            }
                        }
                    } else {
                        OwnershipMode::Owned
                    }
                }
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Ref => OwnershipMode::Borrowed,
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
                                let accesses_fields = self.function_accesses_self_fields(func);
                                if accesses_fields {
                                    OwnershipMode::Borrowed
                                } else {
                                    OwnershipMode::Owned
                                }
                            }
                        }
                    } else {
                        // For Copy types, default to Owned (pass by value)
                        // This allows seamless use of types like Vec2, Vec3 without references
                        if self.is_copy_type(&param.type_) {
                            OwnershipMode::Owned
                        } else {
                            // Perform inference based on usage in function body
                            self.infer_parameter_ownership(
                                &param.name,
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

        Ok(AnalyzedFunction {
            decl: func.clone(),
            inferred_ownership,
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

        if let Some(trait_decl) = self.trait_definitions.get(trait_key) {
            // Find the matching trait method
            if let Some(trait_method) = trait_decl.methods.iter().find(|m| m.name == func.name) {
                // Override ALL parameters to match trait signature
                // Trait implementations must match the trait's exact signature
                // Match by POSITION, not by name (trait uses "rhs", impl might use "other")
                for (i, trait_param) in trait_method.parameters.iter().enumerate() {
                    // Get the corresponding parameter from the implementation by position
                    if let Some(impl_param) = func.parameters.get(i) {
                        if let Some(inferred_mode) =
                            analyzed.inferred_ownership.get_mut(&impl_param.name)
                        {
                            // Convert trait's OwnershipHint to OwnershipMode
                            let trait_mode = match &trait_param.ownership {
                                OwnershipHint::Owned => OwnershipMode::Owned,
                                OwnershipHint::Ref => OwnershipMode::Borrowed,
                                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                                OwnershipHint::Inferred => OwnershipMode::Borrowed, // Default
                            };

                            // Use trait's ownership mode
                            *inferred_mode = trait_mode;
                        }
                    }
                }
            }
        }

        Ok(analyzed)
    }

    fn infer_parameter_ownership(
        &self,
        param_name: &str,
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

        // 5. Default to borrowed for read-only access
        Ok(OwnershipMode::Borrowed)
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

                    // Check if the assignment target is a field of the parameter
                    // e.g., p.x = ... or p.position.x = ...
                    if self.expression_uses_identifier(name, target) {
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
        for stmt in statements {
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expression_uses_identifier(name, expr) {
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
                _ => {}
            }
        }
        false
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
                            object, arguments, ..
                        },
                    ..
                } => {
                    // Check for method calls on fields: self.field.method(param)
                    if let Expression::FieldAccess {
                        object: field_obj, ..
                    } = &**object
                    {
                        if matches!(&**field_obj, Expression::Identifier { name: id, .. } if id == "self")
                        {
                            // Check if any argument uses the parameter
                            for (_label, arg) in arguments {
                                if self.expression_uses_identifier(name, arg) {
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
                }
                _ => {}
            }
        }
        false
    }

    #[allow(clippy::only_used_in_recursion)]
    fn expression_uses_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::Binary { left, right, .. } => {
                self.expression_uses_identifier(name, left)
                    || self.expression_uses_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expression_uses_identifier(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_label, arg)| self.expression_uses_identifier(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_uses_identifier(name, object)
                    || arguments
                        .iter()
                        .any(|(_label, arg)| self.expression_uses_identifier(name, arg))
            }
            Expression::FieldAccess { object, .. } => self.expression_uses_identifier(name, object),
            Expression::TryOp { expr: inner, .. } => self.expression_uses_identifier(name, inner),
            Expression::StructLiteral { fields, .. } => {
                // Check if parameter is used in any field of the struct literal
                fields.iter().any(|(_field_name, field_expr)| {
                    self.expression_uses_identifier(name, field_expr)
                })
            }
            _ => false,
        }
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
                Statement::Return { value, .. } => {
                    if let Some(expr) = value {
                        if self.expr_uses_in_binary_op(name, expr) {
                            return true;
                        }
                    }
                }
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
            Expression::FieldAccess { object, .. } => self.expr_uses_in_binary_op(name, object),
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_binary_op(name, object)
                    || self.expr_uses_in_binary_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_binary_op(name, statements),
            _ => false,
        }
    }

    fn expr_is_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } if id == name => true,
            // Also check for field access on the identifier (self.x, self.y, etc.)
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name: id, .. } = &**object {
                    id == name
                } else {
                    false
                }
            }
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
                    inferred
                }
            })
            .collect();

        FunctionSignature {
            name: func.decl.name.clone(),
            param_ownership,
            return_ownership: OwnershipMode::Owned, // For now, always owned
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
            Type::Custom(name) => {
                // Check if it's a known Copy enum
                if self.copy_enums.contains(name) {
                    eprintln!("DEBUG: is_copy_type('{}') = true (copy_enum)", name);
                    return true;
                }
                // Check if it's a known Copy struct (detected via @derive(Copy))
                if self.copy_structs.contains(name) {
                    eprintln!("DEBUG: is_copy_type('{}') = true (copy_struct)", name);
                    return true;
                }
                // Recognize common Rust primitive types by name
                let is_primitive = matches!(
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
                );
                if !is_primitive && (name == "Vec2" || name == "Vec3" || name == "Vec4") {
                    eprintln!(
                        "DEBUG: is_copy_type('{}') = {} (copy_structs: {:?})",
                        name, is_primitive, self.copy_structs
                    );
                }
                is_primitive
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
                    self.infer_parameter_ownership(&param.name, &func.body, &func.return_type)
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
            "push" | "push_str" | "clear" | "pop" | "remove" | "insert" | "append"
        )
    }

    /// Check if a function modifies self fields (for impl methods)
    fn function_modifies_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_modifies_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement modifies self fields
    fn statement_modifies_self_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if target is self.field
                self.expression_is_self_field_access(target)
            }
            Statement::Expression { expr, .. } => {
                // Check for mutating method calls on self.field
                self.expression_mutates_self_fields(expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_modifies_self_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_modifies_self_fields(s))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                body.iter().any(|s| self.statement_modifies_self_fields(s))
            }
            Statement::Match { arms, .. } => {
                // Check if any match arm modifies self fields
                arms.iter().any(|arm| {
                    // Match arms have an expression body, check if it contains modifications
                    self.expression_contains_self_field_mutations(&arm.body)
                })
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
                    _ => false,
                }
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

    /// Check if a function accesses self fields (for impl methods)
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
                Statement::Assignment { target, .. } => {
                    // Track the variable being assigned to
                    if let Expression::Identifier { name, .. } = target {
                        self.mutated_variables.insert(name.clone());
                    }
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
                _ => {}
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_borrowed() {
        let analyzer = Analyzer::new();

        // fn print(s: string) { println(s) }
        // Should infer borrowed
        let body = vec![Statement::Expression {
            expr: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "println".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Identifier {
                        name: "s".to_string(),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }];

        let mode = analyzer
            .infer_parameter_ownership("s", &body, &None)
            .unwrap();
        assert_eq!(mode, OwnershipMode::Borrowed);
    }
}
