/// Float Type Inference Engine
///
/// Tracks constraints for float literals and unifies them across expressions.

use crate::parser::ast::core::{Expression, Statement, Item, FunctionDecl};
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
    /// Sequential ID counter for generating unique ExprIds
    next_seq_id: usize,
}

impl FloatInference {
    pub fn new() -> Self {
        FloatInference {
            inferred_types: HashMap::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
            function_signatures: HashMap::new(),
            var_assignments: HashMap::new(),
            next_seq_id: 1, // Start at 1, 0 reserved for "unknown"
        }
    }

    /// Main entry point: Infer float types for a program
    pub fn infer_program<'ast>(&mut self, program: &Program<'ast>) {
        // Pass 0: Build function signature registry
        for item in &program.items {
            self.register_function_signature(item);
        }
        
        // Pass 1: Collect constraints from all expressions
        for (_i, item) in program.items.iter().enumerate() {
            self.collect_item_constraints(item);
        }


        // Pass 2: Solve constraints (unification)
        self.solve_constraints();
        
        for (expr_id, float_type) in &self.inferred_types {
            eprintln!("  seq_id={}, {}:{} -> {:?}", expr_id.seq_id, expr_id.line, expr_id.col, float_type);
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
                // Collect return type constraints
                for (_i, stmt) in decl.body.iter().enumerate() {
                    self.collect_statement_constraints(stmt, decl.return_type.as_ref());
                }
            }
            Item::Impl { block, .. } => {
                // Process methods in impl block
                for func in &block.functions {
                    for (i, stmt) in func.body.iter().enumerate() {
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
            Statement::Let { pattern, value, .. } => {

                self.collect_expression_constraints(value, return_type);
                
                // Track variable assignment for constraint propagation
                if let crate::parser::ast::core::Pattern::Identifier(var_name) = pattern {
                    let value_id = self.get_expr_id(value);
                    self.var_assignments.insert(var_name.clone(), value_id);
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
            other => {
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
                    _ => {
                        // Comparison/logical ops don't constrain types
                    }
                }
            }
            Expression::MethodCall { object, arguments, .. } => {
                // Method call requires object and arguments to match
                self.collect_expression_constraints(object, return_type);
                
                for (_label, arg) in arguments {
                    self.collect_expression_constraints(arg, return_type);
                    
                    let obj_id = self.get_expr_id(object);
                    let arg_id = self.get_expr_id(arg);
                    self.constraints.push(Constraint::MustMatch(
                        obj_id,
                        arg_id,
                        "method call".to_string(),
                    ));
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
                // TODO: Look up struct field types and constrain field expressions
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

    /// Get unique ID for an expression (based on source location)
    /// Get unique ID for expression with sequential ID assignment
    /// THE WINDJAMMER WAY: Guaranteed unique IDs prevent HashMap collisions
    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;
        
        let location = expr.location();
        if let Some(loc) = location {
            ExprId {
                seq_id,
                line: loc.line,
                col: loc.column,
            }
        } else {
            ExprId { seq_id, line: 0, col: 0 }
        }
    }

    /// Solve constraints using unification
    fn solve_constraints(&mut self) {
        for (i, constraint) in self.constraints.iter().enumerate() {
        }
        
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
                            (Some(FloatType::F32), Some(FloatType::F64)) |
                            (Some(FloatType::F64), Some(FloatType::F32)) => {
                                self.errors.push(format!(
                                    "Type mismatch at {:?} and {:?}: {} requires same float type",
                                    id1, id2, reason
                                ));
                            }
                            (Some(t), None) => {
                                self.inferred_types.insert(id2, t);
                                changed = true;
                            }
                            (None, Some(t)) => {
                                self.inferred_types.insert(id1, t);
                                changed = true;
                            }
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
