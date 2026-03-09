/// Float Type Inference Engine
///
/// Tracks constraints for float literals and unifies them across expressions.

use crate::parser::ast::core::{Expression, Statement, Item};
use crate::parser::ast::types::Type;
use crate::parser::Program;
use std::collections::HashMap;

/// Unique identifier for an expression
/// THE WINDJAMMER WAY: Sequential IDs ensure uniqueness even when expressions lack locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId {
    /// Sequential ID assigned during AST traversal (guaranteed unique)
    pub seq_id: usize,
    /// Optional source location for debugging (may be None or duplicate)
    pub line: usize,
    pub col: usize,
}

/// Float type (f32 or f64)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatType {
    F32,
    F64,
    Unknown, // Not yet inferred
}

/// Constraint on an expression's float type
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Expression must be f32
    MustBeF32(ExprId, String), // reason
    /// Expression must be f64
    MustBeF64(ExprId, String), // reason
    /// Two expressions must have the same type
    MustMatch(ExprId, ExprId, String), // reason
}

/// Float type inference state
pub struct FloatInference {
    /// Map expression ID → inferred float type
    pub inferred_types: HashMap<ExprId, FloatType>,
    /// Collected constraints
    constraints: Vec<Constraint>,
    /// Errors detected during inference
    pub errors: Vec<String>,
    /// Function signature registry: name → (param_types, return_type)
    function_signatures: HashMap<String, (Vec<Type>, Option<Type>)>,
    /// Variable assignment tracking: variable name → initial value ExprId
    var_assignments: HashMap<String, ExprId>,
    /// Variable type tracking: variable name → explicit Type (for let x: Type = ...)
    var_types: HashMap<String, Type>,
    /// Sequential ID counter for generating unique ExprIds
    next_seq_id: usize,
    /// Struct field types: struct_name → field_name → Type
    struct_field_types: HashMap<String, HashMap<String, Type>>,
    /// THE WINDJAMMER WAY: Cache ExprIds by location to ensure same expression = same ID
    /// Key: (line, col), Value: the first ExprId assigned to that location
    expr_id_cache: HashMap<(usize, usize), ExprId>,
}

impl FloatInference {
    pub fn new() -> Self {
        FloatInference {
            inferred_types: HashMap::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
            function_signatures: HashMap::new(),
            var_assignments: HashMap::new(),
            var_types: HashMap::new(),
            next_seq_id: 1, // Start at 1, 0 reserved for "unknown"
            struct_field_types: HashMap::new(),
            expr_id_cache: HashMap::new(),
        }
    }

    /// Main entry point: Infer float types for a program
    pub fn infer_program<'ast>(&mut self, program: &Program<'ast>) {
        // Pass 0: Build struct field registry and function signatures
        for item in &program.items {
            self.register_struct_fields(item);
            self.register_function_signature(item);
        }
        
        // Pass 1: Collect constraints from all expressions
        for (_i, item) in program.items.iter().enumerate() {
            self.collect_item_constraints(item);
        }

        // Pass 2: Solve constraints (unification)
        self.solve_constraints();
    }

    /// Register struct field types for constraint propagation
    fn register_struct_fields<'ast>(&mut self, item: &Item<'ast>) {
        if let Item::Struct { decl, .. } = item {
            let mut field_map = HashMap::new();
            for field in &decl.fields {
                field_map.insert(field.name.clone(), field.field_type.clone());
            }
            self.struct_field_types.insert(decl.name.clone(), field_map);
        }
    }

    /// Register function signatures for constraint propagation
    fn register_function_signature<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                let param_types: Vec<Type> = decl
                    .parameters
                    .iter()
                    .map(|p| p.type_.clone())
                    .collect();
                
                self.function_signatures.insert(
                    decl.name.clone(),
                    (param_types, decl.return_type.clone()),
                );
            }
            Item::Impl { block, .. } => {
                // Register associated functions (e.g., Vec3::new)
                let type_name = block.type_name.clone();
                for func_decl in &block.functions {
                    let param_types: Vec<Type> = func_decl
                        .parameters
                        .iter()
                        .map(|p| p.type_.clone())
                        .collect();
                    
                    // Register as "TypeName::method_name"
                    let full_name = format!("{}::{}", type_name, func_decl.name);
                    self.function_signatures.insert(
                        full_name,
                        (param_types, func_decl.return_type.clone()),
                    );
                }
            }
            _ => {}
        }
    }

    /// Collect constraints from a top-level item
    fn collect_item_constraints<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                // THE WINDJAMMER WAY: Clear cache for each function to ensure proper scoping
                self.expr_id_cache.clear();
                
                // Collect return type constraints
                for (_i, stmt) in decl.body.iter().enumerate() {
                    self.collect_statement_constraints(stmt, decl.return_type.as_ref());
                }
            }
            Item::Impl { block, .. } => {
                // Process methods in impl block
                for func in &block.functions {
                    // THE WINDJAMMER WAY: Clear cache for each method to ensure proper scoping
                    self.expr_id_cache.clear();
                    
                    for (_i, stmt) in func.body.iter().enumerate() {
                        self.collect_statement_constraints(stmt, func.return_type.as_ref());
                    }
                }
            }
            Item::Struct { .. } => {
            }
            Item::Enum { .. } => {
            }
            Item::Trait { .. } => {
            }
            _ => {}
        }
    }

    /// Collect constraints from a statement
    fn collect_statement_constraints<'ast>(&mut self, stmt: &Statement<'ast>, return_type: Option<&Type>) {
        match stmt {
            Statement::Let { pattern, value, type_, .. } => {

                self.collect_expression_constraints(value, return_type);
                
                // Track variable assignment for constraint propagation
                if let crate::parser::ast::core::Pattern::Identifier(var_name) = pattern {
                    let value_id = self.get_expr_id(value);
                    self.var_assignments.insert(var_name.clone(), value_id);
                    
                    // TDD FIX: Track explicit type annotations (let x: Type = ...)
                    if let Some(ty) = type_ {
                        self.var_types.insert(var_name.clone(), ty.clone());
                    }
                }
                
                // If this expression might be returned (in a function returning a float),
                // constrain it to the return type
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(value);
                        let constraint = match float_ty {
                            FloatType::F32 => Constraint::MustBeF32(expr_id, "function return type f32".to_string()),
                            FloatType::F64 => Constraint::MustBeF64(expr_id, "function return type f64".to_string()),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Expression { expr, .. } => {
                self.collect_expression_constraints(expr, return_type);
                
                // Implicit return: last expression in function body
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => Constraint::MustBeF32(expr_id, "implicit return f32".to_string()),
                            FloatType::F64 => Constraint::MustBeF64(expr_id, "implicit return f64".to_string()),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_expression_constraints(expr, return_type);
                    
                    // Return expression must match function return type
                    if let Some(ret_ty) = return_type {
                        if let Some(float_ty) = self.extract_float_type(ret_ty) {
                            let expr_id = self.get_expr_id(expr);
                            let constraint = match float_ty {
                                FloatType::F32 => Constraint::MustBeF32(expr_id, "return type".to_string()),
                                FloatType::F64 => Constraint::MustBeF64(expr_id, "return type".to_string()),
                                FloatType::Unknown => return,
                            };
                            self.constraints.push(constraint);
                        }
                    }
                }
            }
            Statement::If { condition, then_block, else_block, .. } => {
                // THE WINDJAMMER WAY: if-else branches that return floats must match return type
                self.collect_expression_constraints(condition, return_type);
                for stmt in then_block {
                    self.collect_statement_constraints(stmt, return_type);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_statement_constraints(stmt, return_type);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                // TDD FIX: Traverse while loop body to find float literals in struct fields
                self.collect_expression_constraints(condition, return_type);
                for stmt in body {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Statement::For { iterable, body, .. } => {
                // TDD FIX: Traverse for loop body (same as While loop)
                self.collect_expression_constraints(iterable, return_type);
                for stmt in body {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Match arms must have compatible types AND match return type
                self.collect_expression_constraints(value, return_type);
                
                // Traverse all arms to collect constraints
                for (i, arm) in arms.iter().enumerate() {
                    self.collect_expression_constraints(arm.body, return_type);
                    
                    // TDD FIX: Constrain arm to return type if function returns float
                    if let Some(ret_ty) = return_type {
                        if let Some(float_ty) = self.extract_float_type(ret_ty) {
                            let arm_id = self.get_expr_id(arm.body);
                            let constraint = match float_ty {
                                FloatType::F32 => Constraint::MustBeF32(arm_id, format!("match arm {} return type", i)),
                                FloatType::F64 => Constraint::MustBeF64(arm_id, format!("match arm {} return type", i)),
                                FloatType::Unknown => continue,
                            };
                            self.constraints.push(constraint);
                        }
                    }
                    
                    if let Some(guard) = arm.guard {
                        self.collect_expression_constraints(guard, return_type);
                    }
                }
                
                // TDD FIX: All match arms must have the same type
                if arms.len() > 1 {
                    for i in 0..arms.len() - 1 {
                        let id1 = self.get_expr_id(arms[i].body);
                        let id2 = self.get_expr_id(arms[i + 1].body);
                        self.constraints.push(Constraint::MustMatch(
                            id1,
                            id2,
                            format!("match arms {} and {}", i, i + 1),
                        ));
                    }
                }
            }
            _other => {
            }
        }
    }

    /// Collect constraints from an expression
    fn collect_expression_constraints<'ast>(&mut self, expr: &Expression<'ast>, return_type: Option<&Type>) {
        match expr {
            Expression::Binary { left, right, op, .. } => {
                // Binary ops require both operands to have same type
                self.collect_expression_constraints(left, return_type);
                self.collect_expression_constraints(right, return_type);
                
                // For arithmetic ops (+, -, *, /), operands must match
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div |
                    BinaryOp::Mod => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            format!("binary operation {:?}", op),
                        ));
                        
                        // Recursively constrain nested float literals
                        self.constrain_nested_floats(left, return_type);
                        self.constrain_nested_floats(right, return_type);
                    }
                    // THE WINDJAMMER WAY: Comparison ops also need matching operands
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            format!("comparison {:?} operands", op),
                        ));
                    }
                    _ => {
                        // Logical ops (&&, ||) don't constrain float types
                    }
                }
            }
            Expression::MethodCall { object, method, arguments, .. } => {
                // Method call: infer argument types from method signature
                self.collect_expression_constraints(object, return_type);
                
                // TDD FIX: Constrain method call return type if it returns f32/f64
                // For method() -> f32, constrain the MethodCall expression itself to F32
                // This propagates through MustMatch constraints in binary operations
                
                // Known methods that return f32:
                if matches!(method.as_str(), "get_cost" | "get" | "distance" | "length" | "dot" | "cross") {
                    let method_call_id = self.get_expr_id(expr);
                    self.constraints.push(Constraint::MustBeF32(
                        method_call_id,
                        format!("method {} returns f32", method),
                    ));
                }
                
                // TDD FIX: HashMap.insert(K, V) and Vec.push(T) - constrain arguments to collection element type
                if let Expression::Identifier { name, .. } = object {
                    if let Some(var_type) = self.var_types.get(name).cloned() {
                        // HashMap<K, V>.insert(K, V) - constrain second argument to V
                        if method == "insert" {
                            if let Some(value_type) = self.extract_hashmap_value_type(&var_type) {
                                if let Some(float_ty) = self.extract_float_type(&value_type) {
                                    if arguments.len() >= 2 {
                                        let value_arg = arguments[1].1;
                                        let value_id = self.get_expr_id(value_arg);
                                        match float_ty {
                                            FloatType::F32 => {
                                                self.constraints.push(Constraint::MustBeF32(
                                                    value_id,
                                                    format!("HashMap<K, f32>.insert(K, f32)"),
                                                ));
                                            }
                                            FloatType::F64 => {
                                                self.constraints.push(Constraint::MustBeF64(
                                                    value_id,
                                                    format!("HashMap<K, f64>.insert(K, f64)"),
                                                ));
                                            }
                                            FloatType::Unknown => {}
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Vec<T>.push(T) - constrain first argument to T
                        if method == "push" {
                            if let Some(elem_type) = self.extract_vec_element_type(&var_type) {
                                if let Some(float_ty) = self.extract_float_type(&elem_type) {
                                    if arguments.len() >= 1 {
                                        let value_arg = arguments[0].1;
                                        let value_id = self.get_expr_id(value_arg);
                                        match float_ty {
                                            FloatType::F32 => {
                                                self.constraints.push(Constraint::MustBeF32(
                                                    value_id,
                                                    format!("Vec<f32>.push(f32)"),
                                                ));
                                            }
                                            FloatType::F64 => {
                                                self.constraints.push(Constraint::MustBeF64(
                                                    value_id,
                                                    format!("Vec<f64>.push(f64)"),
                                                ));
                                            }
                                            FloatType::Unknown => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Recurse into ALL arguments to collect binary op constraints
                // This ensures that nested expressions like (x, y, method() * 1.414) are visited
                for (_label, arg) in arguments {
                    self.collect_expression_constraints(arg, return_type);
                }
            }
            Expression::Call { function, arguments, .. } => {
                // Look up function signature and constrain arguments
                self.collect_expression_constraints(function, return_type);
                
                // Extract function name (handles both simple and associated functions)
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        // Handle associated functions like Vec3::new
                        if let Expression::Identifier { name: type_name, .. } = object {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                
                let func_sig = if let Some(name) = &func_name {
                    self.function_signatures.get(name).cloned()
                } else {
                    None
                };
                
                if let Some((param_types, _)) = func_sig {
                    // Match arguments to parameters
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        self.collect_expression_constraints(arg, return_type);
                        
                        if let Some(param_type) = param_types.get(i) {
                            if let Some(float_ty) = self.extract_float_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                let func_name = if let Expression::Identifier { name, .. } = function {
                                    name.clone()
                                } else {
                                    "function".to_string()
                                };
                                let constraint = match float_ty {
                                    FloatType::F32 => Constraint::MustBeF32(
                                        arg_id,
                                        format!("parameter {} of {}", i, func_name),
                                    ),
                                    FloatType::F64 => Constraint::MustBeF64(
                                        arg_id,
                                        format!("parameter {} of {}", i, func_name),
                                    ),
                                    FloatType::Unknown => continue,
                                };
                                self.constraints.push(constraint);
                            }
                        }
                    }
                } else {
                    // Not a simple identifier or not found - still collect from arguments
                    for (_label, arg) in arguments {
                        self.collect_expression_constraints(arg, return_type);
                    }
                }
            }
            Expression::StructLiteral { name, fields, .. } => {
                // THE WINDJAMMER WAY: Constrain struct field expressions to match field types
                
                // Collect field type constraints (two-phase to avoid borrow checker issues)
                let field_constraints: Vec<(String, &'ast Expression<'ast>, FloatType)> = if let Some(struct_fields) = self.struct_field_types.get(name) {
                    fields.iter().filter_map(|(field_name, field_expr)| {
                        if let Some(field_type) = struct_fields.get(field_name) {
                            self.extract_float_type(field_type).map(|float_ty| {
                                (field_name.clone(), *field_expr, float_ty)
                            })
                        } else {
                            None
                        }
                    }).collect()
                } else {
                    Vec::new()
                };
                
                // Now create constraints with mutable access
                for (field_name, field_expr, float_ty) in field_constraints {
                    let expr_id = self.get_expr_id(field_expr);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(
                            expr_id,
                            format!("struct {}.{} is f32", name, field_name),
                        ),
                        FloatType::F64 => Constraint::MustBeF64(
                            expr_id,
                            format!("struct {}.{} is f64", name, field_name),
                        ),
                        FloatType::Unknown => continue,
                    };
                    self.constraints.push(constraint);
                }
                
                // Recursively collect constraints from field expressions
                for (_field_name, expr) in fields {
                    self.collect_expression_constraints(expr, return_type);
                }
            }
            Expression::Tuple { elements, .. } => {
                // Tuple expression: match elements with return type positions
                if let Some(Type::Tuple(tuple_types)) = return_type {
                    for (i, elem) in elements.iter().enumerate() {
                        if let Some(elem_type) = tuple_types.get(i) {
                            // Recurse with the specific type for this position
                            self.collect_expression_constraints(elem, Some(elem_type));
                            
                            // If this position is a float type, constrain the element
                            if let Some(float_ty) = self.extract_float_type(elem_type) {
                                let elem_id = self.get_expr_id(elem);
                                let constraint = match float_ty {
                                    FloatType::F32 => Constraint::MustBeF32(
                                        elem_id,
                                        format!("tuple element {}", i),
                                    ),
                                    FloatType::F64 => Constraint::MustBeF64(
                                        elem_id,
                                        format!("tuple element {}", i),
                                    ),
                                    FloatType::Unknown => continue,
                                };
                                self.constraints.push(constraint);
                                
                                // If element is an identifier, also constrain its assigned value
                                if let Expression::Identifier { name, .. } = elem {
                                    if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                                        let value_constraint = match float_ty {
                                            FloatType::F32 => Constraint::MustBeF32(
                                                value_id,
                                                format!("variable {} assigned value", name),
                                            ),
                                            FloatType::F64 => Constraint::MustBeF64(
                                                value_id,
                                                format!("variable {} assigned value", name),
                                            ),
                                            FloatType::Unknown => continue,
                                        };
                                        self.constraints.push(value_constraint);
                                    } else {
                                    }
                                }
                            }
                        } else {
                            // No type info for this position, recurse without constraint
                            self.collect_expression_constraints(elem, None);
                        }
                    }
                } else {
                    // No tuple type info, just recurse
                    for elem in elements {
                        self.collect_expression_constraints(elem, None);
                    }
                }
            }
            Expression::Cast { expr: inner, type_, .. } => {
                // Cast expression provides explicit type constraint
                self.collect_expression_constraints(inner, return_type);
                
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(inner_id, "cast to f32".to_string()),
                        FloatType::F64 => Constraint::MustBeF64(inner_id, "cast to f64".to_string()),
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(constraint);
                }
            }
            Expression::Block { statements, .. } => {
                // TDD FIX: Traverse block expressions (e.g., let x = { match ... })
                // Match statements inside blocks weren't being visited!
                for stmt in statements {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            _ => {}
        }
    }

    /// Extract FloatType from a Type
    fn extract_float_type(&self, ty: &Type) -> Option<FloatType> {
        match ty {
            Type::Custom(name) if name == "f32" => Some(FloatType::F32),
            Type::Custom(name) if name == "f64" => Some(FloatType::F64),
            Type::Tuple(types) => {
                // Search tuple for float types
                for t in types {
                    if let Some(float_ty) = self.extract_float_type(t) {
                        return Some(float_ty);
                    }
                }
                None
            }
            Type::Vec(inner) => self.extract_float_type(inner),
            Type::Array(inner, _) => self.extract_float_type(inner),
            _ => None,
        }
    }
    
    /// TDD FIX: Extract value type V from HashMap<K, V>
    fn extract_hashmap_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, type_args) if name == "HashMap" => {
                // HashMap<K, V> has 2 type arguments, V is at index 1
                if type_args.len() >= 2 {
                    Some(type_args[1].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    /// TDD FIX: Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, type_args) if name == "Vec" => {
                // Vec<T> has 1 type argument
                if type_args.len() >= 1 {
                    Some(type_args[0].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get unique ID for an expression (based on source location)
    /// Get unique ID for expression with location-based caching
    /// THE WINDJAMMER WAY: Cache by location to ensure same expression = same ID
    /// This fixes the problem where same expression got multiple IDs during traversal
    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let location = expr.location();
        let (line, col) = if let Some(loc) = location {
            (loc.line, loc.column)
        } else {
            (0, 0)
        };
        
        // Check cache first - if we've seen this location before, return same ID
        if line > 0 {  // Only cache expressions with valid locations
            if let Some(&cached_id) = self.expr_id_cache.get(&(line, col)) {
                return cached_id;
            }
        }
        
        // Generate new sequential ID
        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;
        
        let expr_id = ExprId { seq_id, line, col };
        
        // Cache it for future lookups
        if line > 0 {
            self.expr_id_cache.insert((line, col), expr_id);
        }
        
        expr_id
    }

    /// Solve constraints using unification
    fn solve_constraints(&mut self) {
        // Simple constraint solver: Apply constraints repeatedly until convergence
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for constraint in self.constraints.clone() {
                match constraint {
                    Constraint::MustBeF32(expr_id, reason) => {
                        // THE WINDJAMMER WAY: Always insert if missing, check conflicts if present
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(FloatType::F64) => {
                                self.errors.push(format!(
                                    "Type conflict at seq_id={}, {}:{}: must be f32 ({}) but was inferred as f64",
                                    expr_id.seq_id, expr_id.line, expr_id.col, reason
                                ));
                            }
                            Some(FloatType::F32) => {
                                // Already F32, no change needed
                            }
                            Some(FloatType::Unknown) => {
                                // Unknown -> F32
                                self.inferred_types.insert(expr_id, FloatType::F32);
                                changed = true;
                            }
                            None => {
                                // Not yet inferred, insert f32
                                self.inferred_types.insert(expr_id, FloatType::F32);
                                changed = true;
                            }
                        }
                    }
                    Constraint::MustBeF64(expr_id, reason) => {
                        // THE WINDJAMMER WAY: Always insert if missing, check conflicts if present
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(FloatType::F32) => {
                                self.errors.push(format!(
                                    "Type conflict at seq_id={}, {}:{}: must be f64 ({}) but was inferred as f32",
                                    expr_id.seq_id, expr_id.line, expr_id.col, reason
                                ));
                            }
                            Some(FloatType::F64) => {
                                // Already F64, no change needed
                            }
                            Some(FloatType::Unknown) => {
                                // Unknown -> F64
                                self.inferred_types.insert(expr_id, FloatType::F64);
                                changed = true;
                            }
                            None => {
                                // Not yet inferred, insert f64
                                self.inferred_types.insert(expr_id, FloatType::F64);
                                changed = true;
                            }
                        }
                    }
                    Constraint::MustMatch(id1, id2, reason) => {
                        let ty1 = self.inferred_types.get(&id1).copied();
                        let ty2 = self.inferred_types.get(&id2).copied();

                        match (ty1, ty2) {
                            // Conflict: f32 vs f64
                            (Some(FloatType::F32), Some(FloatType::F64)) |
                            (Some(FloatType::F64), Some(FloatType::F32)) => {
                                self.errors.push(format!(
                                    "Type mismatch at {:?} and {:?}: {} requires same float type",
                                    id1, id2, reason
                                ));
                            }
                            // Propagate f32 to unknown or untyped
                            (Some(FloatType::F32), Some(FloatType::Unknown)) |
                            (Some(FloatType::F32), None) => {
                                self.inferred_types.insert(id2, FloatType::F32);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F32)) |
                            (None, Some(FloatType::F32)) => {
                                self.inferred_types.insert(id1, FloatType::F32);
                                changed = true;
                            }
                            // Propagate f64 to unknown or untyped
                            (Some(FloatType::F64), Some(FloatType::Unknown)) |
                            (Some(FloatType::F64), None) => {
                                self.inferred_types.insert(id2, FloatType::F64);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F64)) |
                            (None, Some(FloatType::F64)) => {
                                self.inferred_types.insert(id1, FloatType::F64);
                                changed = true;
                            }
                            // Both same concrete type - no change
                            (Some(FloatType::F32), Some(FloatType::F32)) |
                            (Some(FloatType::F64), Some(FloatType::F64)) => {}
                            // Both unknown or untyped - wait for more constraints
                            _ => {}
                        }
                    }
                }
            }
        }

        // THE WINDJAMMER WAY: If no new changes occurred, we've converged successfully
        // Only error if we're still changing after max iterations (true infinite loop)
        if iterations >= MAX_ITERATIONS && changed {
            self.errors.push(format!(
                "Type inference did not converge after {} iterations",
                MAX_ITERATIONS
            ));
        }
    }

    /// Recursively constrain float literals in nested expressions
    /// Used for binary ops like: x * y * 0.5 (all must match)
    fn constrain_nested_floats<'ast>(&mut self, expr: &Expression<'ast>, return_type: Option<&Type>) {
        match expr {
            Expression::Binary { left, right, op, .. } => {
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div |
                    BinaryOp::Mod => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            "nested binary operation".to_string(),
                        ));
                        
                        // Recurse deeper
                        self.constrain_nested_floats(left, return_type);
                        self.constrain_nested_floats(right, return_type);
                    }
                    _ => {}
                }
            }
            Expression::Literal { .. } => {
                // Base case: literal found
            }
            Expression::Cast { expr: inner, type_, .. } => {
                // Cast expression provides explicit type hint
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(inner_id, "cast to f32".to_string()),
                        FloatType::F64 => Constraint::MustBeF64(inner_id, "cast to f64".to_string()),
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(constraint);
                }
                self.constrain_nested_floats(inner, return_type);
            }
            _ => {}
        }
    }

    /// Get inferred float type for an expression
    pub fn get_float_type<'ast>(&self, expr: &Expression<'ast>) -> FloatType {
        // Look up by location only (seq_id not available after inference)
        // Find ExprId with matching location
        let location = expr.location();
        let (line, col) = if let Some(loc) = location {
            (loc.line, loc.column)
        } else {
            (0, 0)
        };
        
        // DEBUG: Print all inferred types
        if line > 0 {
            for (expr_id, float_type) in &self.inferred_types {
                eprintln!("    seq_id={}, {}:{} -> {:?}", expr_id.seq_id, expr_id.line, expr_id.col, float_type);
            }
        }
        
        // Search for any ExprId with matching location
        for (expr_id, float_type) in &self.inferred_types {
            if expr_id.line == line && expr_id.col == col {
                return *float_type;
            }
        }
        
        FloatType::F64 // Default to f64
    }
}
