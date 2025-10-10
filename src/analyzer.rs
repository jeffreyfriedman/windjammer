// Ownership and borrow checking analyzer
use crate::parser::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AnalyzedFunction {
    pub decl: FunctionDecl,
    pub inferred_ownership: HashMap<String, OwnershipMode>,
    // PHASE 2 OPTIMIZATION: Track unnecessary clones that can be eliminated
    pub clone_optimizations: Vec<CloneOptimization>,
    // PHASE 3 OPTIMIZATION: Track struct mapping opportunities
    pub struct_mapping_optimizations: Vec<StructMappingOptimization>,
    // PHASE 4 OPTIMIZATION: Track string operations for optimization
    pub string_optimizations: Vec<StringOptimization>,
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
        SignatureRegistry {
            signatures: HashMap::new(),
        }
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
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            variables: HashMap::new(),
        }
    }

    pub fn analyze_program(
        &mut self,
        program: &Program,
    ) -> Result<(Vec<AnalyzedFunction>, SignatureRegistry), String> {
        let mut analyzed = Vec::new();
        let mut registry = SignatureRegistry::new();

        for item in &program.items {
            match item {
                Item::Function(func) => {
                    let analyzed_func = self.analyze_function(func)?;
                    let signature = self.build_signature(&analyzed_func);
                    registry.add_function(func.name.clone(), signature);
                    analyzed.push(analyzed_func);
                }
                Item::Impl(impl_block) => {
                    // Analyze methods in impl blocks
                    for func in &impl_block.functions {
                        let analyzed_func = self.analyze_function(func)?;
                        let signature = self.build_signature(&analyzed_func);
                        registry.add_function(func.name.clone(), signature);
                        analyzed.push(analyzed_func);
                    }
                }
                _ => {}
            }
        }

        Ok((analyzed, registry))
    }

    fn analyze_function(&mut self, func: &FunctionDecl) -> Result<AnalyzedFunction, String> {
        let mut inferred_ownership = HashMap::new();

        // Analyze each parameter to infer ownership mode
        for param in &func.parameters {
            let mode = match param.ownership {
                OwnershipHint::Owned => OwnershipMode::Owned,
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Ref => OwnershipMode::Borrowed,
                OwnershipHint::Inferred => {
                    // Perform inference based on usage in function body
                    self.infer_parameter_ownership(&param.name, &func.body, &func.return_type)?
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

        Ok(AnalyzedFunction {
            decl: func.clone(),
            inferred_ownership,
            clone_optimizations,
            struct_mapping_optimizations,
            string_optimizations,
        })
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

        // 4. Default to borrowed for read-only access
        Ok(OwnershipMode::Borrowed)
    }

    fn is_mutated(&self, name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment {
                    target: Expression::Identifier(id),
                    ..
                } => {
                    if id == name {
                        return true;
                    }
                }
                Statement::Expression(expr) => {
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
                Statement::Loop { body }
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
                if let Expression::Identifier(id) = &**object {
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
                Statement::Return(Some(expr)) => {
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
            if let Statement::Let {
                value: Expression::StructLiteral { fields, .. },
                ..
            } = stmt
            {
                for (_, field_expr) in fields {
                    if self.expression_uses_identifier(name, field_expr) {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[allow(clippy::only_used_in_recursion)]
    fn expression_uses_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(id) => id == name,
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
            Expression::TryOp(inner) => self.expression_uses_identifier(name, inner),
            _ => false,
        }
    }

    fn build_signature(&self, func: &AnalyzedFunction) -> FunctionSignature {
        let param_ownership: Vec<OwnershipMode> = func
            .decl
            .parameters
            .iter()
            .map(|param| {
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
                // Recognize common Rust primitive types by name
                matches!(
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
                )
            }
            _ => false,
        }
    }

    /// PHASE 2 OPTIMIZATION: Detect unnecessary .clone() calls
    /// Returns a list of clones that can be optimized away
    fn detect_unnecessary_clones(&self, func: &FunctionDecl) -> Vec<CloneOptimization> {
        let mut optimizations = Vec::new();

        // Track variable usage: (variable_name, (read_count, write_count, escapes))
        let mut usage = HashMap::new();

        // First pass: analyze usage patterns
        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_clones(stmt, &mut usage, idx);
        }

        // Second pass: identify unnecessary clones
        for (var_name, (reads, writes, escapes)) in usage {
            // Clone is unnecessary if:
            // 1. Variable is only read (never written) -> can use borrow
            if writes == 0 && !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name.clone(),
                    location: 0, // TODO: track actual location
                    reason: CloneEliminationReason::OnlyRead,
                });
            }
            // 2. Variable is used once and doesn't escape -> can move
            else if reads == 1 && writes == 0 && !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name.clone(),
                    location: 0,
                    reason: CloneEliminationReason::SingleUse,
                });
            }
            // 3. Variable doesn't escape function -> use local borrow
            else if !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name,
                    location: 0,
                    reason: CloneEliminationReason::LocalOnly,
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
            Statement::Let { value, .. } | Statement::Return(Some(value)) => {
                self.analyze_expression_for_struct_mappings(value, optimizations);
            }
            Statement::Expression(expr) => {
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
            Expression::StructLiteral { name, fields } => {
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
            Expression::Identifier(name) => Some(name.clone()),
            Expression::FieldAccess { object, .. } => {
                // Extract base object
                if let Expression::Identifier(name) = &**object {
                    Some(name.clone())
                } else {
                    None
                }
            }
            Expression::MethodCall { object, .. } => {
                if let Expression::Identifier(name) = &**object {
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
            Expression::Identifier(name) => name.clone(),
            Expression::FieldAccess { object, field } => {
                format!("{}.{}", self.expression_to_string(object), field)
            }
            Expression::MethodCall { object, method, .. } => {
                format!("{}.{}()", self.expression_to_string(object), method)
            }
            Expression::Literal(lit) => format!("{:?}", lit),
            _ => "expr".to_string(),
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
    fn analyze_statement_for_string_ops(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<StringOptimization>,
        idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. } | Statement::Return(Some(value)) => {
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

                // Check for concatenation chains (a + b + c)
                self.detect_concatenation_chain(value, optimizations, idx);
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body } => {
                // Check for string building in loops
                for s in body {
                    if self.is_string_accumulation(s) {
                        optimizations.push(StringOptimization {
                            optimization_type: StringOptimizationType::LoopAccumulation,
                            estimated_capacity: Some(256), // Default for loop accumulation
                            location: idx,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// Detect concatenation chains (a + b + c + ...)
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
    #[allow(clippy::only_used_in_recursion)]
    fn count_concatenations(&self, expr: &Expression, count: &mut usize) {
        if let Expression::Binary { op, left, right } = expr {
            if matches!(op, BinaryOp::Add) {
                *count += 1;
                self.count_concatenations(left, count);
                self.count_concatenations(right, count);
            }
        }
    }

    /// Check if a statement is accumulating strings (s += ...)
    fn is_string_accumulation(&self, stmt: &Statement) -> bool {
        matches!(
            stmt,
            Statement::Assignment {
                target: Expression::Identifier(_),
                ..
            }
        )
    }

    /// Helper to analyze a statement for clone patterns
    fn analyze_statement_for_clones(
        &self,
        stmt: &Statement,
        usage: &mut HashMap<String, (usize, usize, bool)>,
        _idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. } => {
                self.analyze_expression_for_clones(value, usage);
            }
            Statement::Assignment { target, value, .. } => {
                // Track writes
                if let Expression::Identifier(name) = target {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false));
                    entry.1 += 1; // increment write count
                }
                self.analyze_expression_for_clones(value, usage);
            }
            Statement::Return(Some(expr)) => {
                // Returned values escape the function
                if let Expression::Identifier(name) = expr {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false));
                    entry.2 = true; // mark as escapes
                }
                self.analyze_expression_for_clones(expr, usage);
            }
            Statement::Expression(expr) => {
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
                for stmt in body {
                    self.analyze_statement_for_clones(stmt, usage, _idx);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.analyze_expression_for_clones(iterable, usage);
                for stmt in body {
                    self.analyze_statement_for_clones(stmt, usage, _idx);
                }
            }
            Statement::Loop { body } => {
                for stmt in body {
                    self.analyze_statement_for_clones(stmt, usage, _idx);
                }
            }
            _ => {}
        }
    }

    /// Helper to analyze an expression for variable usage
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_expression_for_clones(
        &self,
        expr: &Expression,
        usage: &mut HashMap<String, (usize, usize, bool)>,
    ) {
        match expr {
            Expression::Identifier(name) => {
                // Track reads
                let entry = usage.entry(name.clone()).or_insert((0, 0, false));
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
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                self.analyze_expression_for_clones(condition, usage);
                self.analyze_expression_for_clones(true_expr, usage);
                self.analyze_expression_for_clones(false_expr, usage);
            }
            Expression::Cast { expr, .. } => {
                self.analyze_expression_for_clones(expr, usage);
            }
            _ => {}
        }
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
        let body = vec![Statement::Expression(Expression::Call {
            function: Box::new(Expression::Identifier("println".to_string())),
            arguments: vec![(None, Expression::Identifier("s".to_string()))],
        })];

        let mode = analyzer
            .infer_parameter_ownership("s", &body, &None)
            .unwrap();
        assert_eq!(mode, OwnershipMode::Borrowed);
    }
}
