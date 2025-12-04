// Trait Bound Inference Engine
//
// This module implements automatic inference of trait bounds for generic type parameters.
// Instead of requiring users to write `fn func<T: Display + Clone>(x: T)`, they can write
// `fn func<T>(x: T)` and the compiler will infer the bounds from usage.
//
// ## Algorithm
//
// 1. **Constraint Collection**: Walk the function body and collect trait requirements
//    - `println!("{}", x)` → requires Display
//    - `x.clone()` → requires Clone
//    - `x + y` → requires Add
//    - etc.
//
// 2. **Constraint Simplification**: Deduplicate and merge constraints per type parameter
//
// 3. **Code Generation**: Generate Rust with inferred bounds added to explicit bounds
//
// ## Example
//
// ```windjammer
// fn print_and_clone<T>(x: T) {
//     println!("{}", x)  // Requires Display
//     let y = x.clone()  // Requires Clone
// }
// ```
//
// Infers: `fn print_and_clone<T: Display + Clone>(x: T)`

use crate::parser::{BinaryOp, Expression, FunctionDecl, Statement, TypeParam};
use std::collections::{HashMap, HashSet};

/// A trait constraint on a type parameter
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitConstraint {
    /// The type parameter name (e.g., "T")
    pub type_param: String,
    /// The trait name (e.g., "Display", "Clone")
    pub trait_name: String,
}

/// Inferred trait bounds for a function
#[derive(Debug, Clone)]
pub struct InferredBounds {
    /// Map from type parameter name to set of trait names
    pub bounds: HashMap<String, HashSet<String>>,
}

impl InferredBounds {
    pub fn new() -> Self {
        InferredBounds {
            bounds: HashMap::new(),
        }
    }

    /// Add a constraint to the inferred bounds
    pub fn add_constraint(&mut self, type_param: String, trait_name: String) {
        self.bounds
            .entry(type_param)
            .or_default()
            .insert(trait_name);
    }

    /// Get sorted trait bounds for a type parameter
    pub fn get_bounds(&self, type_param: &str) -> Vec<String> {
        self.bounds
            .get(type_param)
            .map(|traits| {
                let mut sorted: Vec<_> = traits.iter().cloned().collect();
                sorted.sort();
                sorted
            })
            .unwrap_or_default()
    }

    /// Merge explicit bounds from the AST with inferred bounds
    pub fn merge_with_explicit(&self, type_params: &[TypeParam]) -> Vec<TypeParam> {
        type_params
            .iter()
            .map(|param| {
                let mut merged_bounds = param.bounds.clone();

                // Add inferred bounds that aren't already explicit
                if let Some(inferred) = self.bounds.get(&param.name) {
                    for trait_name in inferred {
                        if !merged_bounds.contains(trait_name) {
                            merged_bounds.push(trait_name.clone());
                        }
                    }
                }

                // Sort for stability
                merged_bounds.sort();

                TypeParam {
                    name: param.name.clone(),
                    bounds: merged_bounds,
                }
            })
            .collect()
    }

    /// Check if any bounds were inferred
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }
}

impl Default for InferredBounds {
    fn default() -> Self {
        Self::new()
    }
}

/// The trait bound inference engine
pub struct InferenceEngine {
    /// Current function's type parameters
    type_params: HashSet<String>,
    /// Map from variable names to their type parameters (e.g., "x" -> "T")
    var_to_type_param: HashMap<String, String>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        InferenceEngine {
            type_params: HashSet::new(),
            var_to_type_param: HashMap::new(),
        }
    }

    /// Infer trait bounds for a function
    pub fn infer_function_bounds(&mut self, func: &FunctionDecl) -> InferredBounds {
        // Collect type parameter names
        self.type_params = func.type_params.iter().map(|p| p.name.clone()).collect();

        // Map function parameters to their type parameters
        self.var_to_type_param.clear();
        for param in &func.parameters {
            if let crate::parser::Type::Generic(type_param) = &param.type_ {
                self.var_to_type_param
                    .insert(param.name.clone(), type_param.clone());
            }
        }

        let mut bounds = InferredBounds::new();

        // Analyze function body
        self.collect_constraints_from_statements(&func.body, &mut bounds);

        bounds
    }

    /// Collect constraints from a list of statements
    fn collect_constraints_from_statements(
        &self,
        statements: &[Statement],
        bounds: &mut InferredBounds,
    ) {
        for stmt in statements {
            self.collect_constraints_from_statement(stmt, bounds);
        }
    }

    /// Collect constraints from a single statement
    fn collect_constraints_from_statement(&self, stmt: &Statement, bounds: &mut InferredBounds) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.collect_constraints_from_expression(expr, bounds);
            }
            Statement::Let { value, .. } => {
                self.collect_constraints_from_expression(value, bounds);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_constraints_from_expression(expr, bounds);
            }
            Statement::Return { value: None, .. } => {
                // No constraints from bare return
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_constraints_from_expression(condition, bounds);
                self.collect_constraints_from_statements(then_block, bounds);
                if let Some(else_block) = else_block {
                    self.collect_constraints_from_statements(else_block, bounds);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_constraints_from_expression(value, bounds);
                for arm in arms {
                    self.collect_constraints_from_expression(&arm.body, bounds);
                    if let Some(guard) = &arm.guard {
                        self.collect_constraints_from_expression(guard, bounds);
                    }
                }
            }
            Statement::For { iterable, body, .. } => {
                // for x in iterable requires IntoIterator
                self.infer_trait_for_expression(iterable, "IntoIterator", bounds);
                self.collect_constraints_from_statements(body, bounds);
            }
            Statement::While {
                condition, body, ..
            } => {
                self.collect_constraints_from_expression(condition, bounds);
                self.collect_constraints_from_statements(body, bounds);
            }
            Statement::Loop { body, .. } => {
                self.collect_constraints_from_statements(body, bounds);
            }
            Statement::Thread { body, .. } | Statement::Async { body, .. } => {
                self.collect_constraints_from_statements(body, bounds);
            }
            _ => {
                // Other statement types don't contribute constraints yet
            }
        }
    }

    /// Collect constraints from an expression
    fn collect_constraints_from_expression(&self, expr: &Expression, bounds: &mut InferredBounds) {
        match expr {
            // Binary operators
            Expression::Binary {
                op, left, right, ..
            } => {
                match op {
                    BinaryOp::Add => {
                        self.infer_trait_for_expression(left, "Add", bounds);
                        self.infer_trait_for_expression(right, "Add", bounds);
                    }
                    BinaryOp::Sub => {
                        self.infer_trait_for_expression(left, "Sub", bounds);
                        self.infer_trait_for_expression(right, "Sub", bounds);
                    }
                    BinaryOp::Mul => {
                        self.infer_trait_for_expression(left, "Mul", bounds);
                        self.infer_trait_for_expression(right, "Mul", bounds);
                    }
                    BinaryOp::Div => {
                        self.infer_trait_for_expression(left, "Div", bounds);
                        self.infer_trait_for_expression(right, "Div", bounds);
                    }
                    BinaryOp::Eq | BinaryOp::Ne => {
                        self.infer_trait_for_expression(left, "PartialEq", bounds);
                        self.infer_trait_for_expression(right, "PartialEq", bounds);
                    }
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        self.infer_trait_for_expression(left, "PartialOrd", bounds);
                        self.infer_trait_for_expression(right, "PartialOrd", bounds);
                    }
                    _ => {
                        // Other binary ops don't require traits (logical ops, etc.)
                    }
                }

                // Recurse into operands
                self.collect_constraints_from_expression(left, bounds);
                self.collect_constraints_from_expression(right, bounds);
            }

            // Method calls
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Common trait methods
                match method.as_str() {
                    "clone" => {
                        self.infer_trait_for_expression(object, "Clone", bounds);
                    }
                    "to_string" => {
                        self.infer_trait_for_expression(object, "ToString", bounds);
                    }
                    _ => {
                        // Custom methods - can't infer trait (would need type information)
                    }
                }

                // Recurse
                self.collect_constraints_from_expression(object, bounds);
                for (_, arg) in arguments {
                    self.collect_constraints_from_expression(arg, bounds);
                }
            }

            // Macro invocations (println!, format!, etc.)
            Expression::MacroInvocation { name, args, .. } => {
                if name == "println" || name == "format" {
                    // Analyze format string for Display vs Debug
                    if let Some(Expression::Literal {
                        value: crate::parser::Literal::String(fmt),
                        ..
                    }) = args.first()
                    {
                        // Convert Vec<Expression> to Vec<(Option<String>, Expression)>
                        let labeled_args: Vec<(Option<String>, Expression)> =
                            args[1..].iter().map(|e| (None, e.clone())).collect();
                        self.analyze_format_string(fmt, &labeled_args, bounds);
                    }
                }

                // Recurse into macro arguments
                for arg in args {
                    self.collect_constraints_from_expression(arg, bounds);
                }
            }

            // Function calls
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Recurse
                self.collect_constraints_from_expression(function, bounds);
                for (_, arg) in arguments {
                    self.collect_constraints_from_expression(arg, bounds);
                }
            }

            // Ternary operator

            // Block expression
            Expression::Block { statements, .. } => {
                self.collect_constraints_from_statements(statements, bounds);
            }

            // Other expressions: recurse as needed
            _ => {
                // TODO: Handle other expression types as needed
            }
        }
    }

    /// Analyze a format string to determine required traits
    fn analyze_format_string(
        &self,
        format_str: &str,
        arguments: &[(Option<String>, Expression)],
        bounds: &mut InferredBounds,
    ) {
        // Simple heuristic: check for {:?} (Debug) vs {} (Display)
        let has_debug = format_str.contains("{:?}") || format_str.contains("{:#?}");
        let has_display = format_str.contains("{}");

        if has_debug {
            for (_, arg) in arguments {
                self.infer_trait_for_expression(arg, "Debug", bounds);
            }
        } else if has_display {
            // Only infer Display if not Debug (Debug takes precedence)
            for (_, arg) in arguments {
                self.infer_trait_for_expression(arg, "Display", bounds);
            }
        }
    }

    /// Infer a trait requirement for an expression
    fn infer_trait_for_expression(
        &self,
        expr: &Expression,
        trait_name: &str,
        bounds: &mut InferredBounds,
    ) {
        // CONSERVATIVE APPROACH for v0.10.0:
        // If we see a trait usage and the function has type parameters,
        // assume ALL type parameters might need this trait.
        // This is conservative but simple and works for most cases.

        // Try to extract type parameter from expression
        if let Some(type_param) = self.extract_type_param(expr) {
            bounds.add_constraint(type_param, trait_name.to_string());
        } else {
            // Fallback: if we can't determine which variable, apply to ALL type parameters
            // This is conservative: better to over-constrain than under-constrain
            for type_param in &self.type_params {
                bounds.add_constraint(type_param.clone(), trait_name.to_string());
            }
        }
    }

    /// Extract type parameter name from an expression
    fn extract_type_param(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => {
                // Look up the type parameter for this variable
                self.var_to_type_param.get(name).cloned()
            }
            _ => None,
        }
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Literal, OwnershipHint, Parameter, Type};

    #[test]
    fn test_infer_display_from_println() {
        let mut engine = InferenceEngine::new();

        let func = FunctionDecl {
            name: "print".to_string(),
            is_pub: false,
            is_extern: false,
            decorators: vec![],
            type_params: vec![TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            }],
            parameters: vec![Parameter {
                name: "x".to_string(),
                pattern: None,
                type_: Type::Generic("T".to_string()),
                ownership: OwnershipHint::Inferred,
                is_mutable: false,
            }],
            return_type: None,
            is_async: false,
            body: vec![Statement::Expression {
                expr: Expression::MacroInvocation {
                    name: "println".to_string(),
                    args: vec![
                        Expression::Literal {
                            value: Literal::String("{}".to_string()),
                            location: None,
                        },
                        Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        },
                    ],
                    delimiter: crate::parser::MacroDelimiter::Parens,
                    location: None,
                },
                location: None,
            }],
            where_clause: vec![],
            parent_type: None,
        };

        let bounds = engine.infer_function_bounds(&func);

        assert!(!bounds.is_empty());
        let t_bounds = bounds.get_bounds("T");
        assert!(t_bounds.contains(&"Display".to_string()));
    }

    #[test]
    fn test_infer_clone_from_method_call() {
        let mut engine = InferenceEngine::new();

        let func = FunctionDecl {
            name: "duplicate".to_string(),
            is_pub: false,
            is_extern: false,
            decorators: vec![],
            type_params: vec![TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            }],
            parameters: vec![Parameter {
                name: "x".to_string(),
                pattern: None,
                type_: Type::Generic("T".to_string()),
                ownership: OwnershipHint::Inferred,
                is_mutable: false,
            }],
            return_type: Some(Type::Generic("T".to_string())),
            is_async: false,
            body: vec![Statement::Expression {
                expr: Expression::MethodCall {
                    object: Box::new(Expression::Identifier {
                        name: "x".to_string(),
                        location: None,
                    }),
                    method: "clone".to_string(),
                    type_args: None,
                    arguments: vec![],
                    location: None,
                },
                location: None,
            }],
            where_clause: vec![],
            parent_type: None,
        };

        let bounds = engine.infer_function_bounds(&func);

        assert!(!bounds.is_empty());
        let t_bounds = bounds.get_bounds("T");
        assert!(t_bounds.contains(&"Clone".to_string()));
    }

    #[test]
    fn test_infer_add_from_binary_op() {
        let mut engine = InferenceEngine::new();

        let func = FunctionDecl {
            is_pub: false,
            is_extern: false,
            name: "add".to_string(),
            decorators: vec![],
            type_params: vec![TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            }],
            parameters: vec![
                Parameter {
                    name: "x".to_string(),
                    pattern: None,
                    type_: Type::Generic("T".to_string()),
                    ownership: OwnershipHint::Inferred,
                    is_mutable: false,
                },
                Parameter {
                    name: "y".to_string(),
                    pattern: None,
                    type_: Type::Generic("T".to_string()),
                    ownership: OwnershipHint::Inferred,
                    is_mutable: false,
                },
            ],
            return_type: Some(Type::Generic("T".to_string())),
            is_async: false,
            body: vec![Statement::Expression {
                expr: Expression::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expression::Identifier {
                        name: "x".to_string(),
                        location: None,
                    }),
                    right: Box::new(Expression::Identifier {
                        name: "y".to_string(),
                        location: None,
                    }),
                    location: None,
                },
                location: None,
            }],
            where_clause: vec![],
            parent_type: None,
        };

        let bounds = engine.infer_function_bounds(&func);

        assert!(!bounds.is_empty());
        let t_bounds = bounds.get_bounds("T");
        assert!(t_bounds.contains(&"Add".to_string()));
    }
}
