#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Self/Field Analysis Module
//
// This module provides functions to analyze AST nodes and determine if they:
// - Access struct fields
// - Mutate struct fields
// - Modify self parameters
// - Modify variables
//
// These functions are used by the ownership inference system to determine
// whether methods need `&self`, `&mut self`, or `self` parameters.
use crate::parser::{Expression, FunctionDecl, Statement};
use std::collections::HashSet;

/// Context needed for field/self analysis
pub struct AnalysisContext<'a, 'ast> {
    /// Current function parameters (to distinguish params from fields)
    pub current_function_params: &'a [crate::parser::Parameter<'ast>],
    /// Fields of the current struct (if in impl block)
    pub current_struct_fields: &'a HashSet<String>,
}

impl<'a, 'ast> AnalysisContext<'a, 'ast> {
    pub fn new(params: &'a [crate::parser::Parameter<'ast>], fields: &'a HashSet<String>) -> Self {
        Self {
            current_function_params: params,
            current_struct_fields: fields,
        }
    }
}

// =============================================================================
// Function-Level Analysis
// =============================================================================

/// Check if a function accesses any struct fields
pub fn function_accesses_fields(ctx: &AnalysisContext, func: &FunctionDecl) -> bool {
    for stmt in &func.body {
        if statement_accesses_fields(ctx, stmt) {
            return true;
        }
    }
    false
}

/// Check if a function mutates any struct fields
pub fn function_mutates_fields(ctx: &AnalysisContext, func: &FunctionDecl) -> bool {
    for stmt in &func.body {
        if statement_mutates_fields(ctx, stmt) {
            return true;
        }
    }
    false
}

/// Check if a function returns Self (for builder pattern detection)
pub fn function_returns_self_type(func: &FunctionDecl) -> bool {
    use crate::parser::{Expression, Statement, Type};

    // First check if return type is a custom type (struct type)
    let returns_custom_type = matches!(&func.return_type, Some(Type::Custom(_)));

    if !returns_custom_type {
        return false;
    }

    // Now check if the function body actually returns `self`
    // Check the last statement in the body
    if let Some(last_stmt) = func.body.last() {
        match last_stmt {
            Statement::Return {
                value: Some(expr), ..
            } => {
                // Explicit return self
                matches!(expr, Expression::Identifier { name, .. } if name == "self")
            }
            Statement::Expression { expr, .. } => {
                // Implicit return self (last expression)
                matches!(expr, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    } else {
        false
    }
}

/// Check if a function modifies self (for self parameter inference)
pub fn function_modifies_self(func: &FunctionDecl) -> bool {
    for stmt in &func.body {
        if statement_modifies_self(stmt) {
            return true;
        }
    }
    false
}

// =============================================================================
// Statement-Level Analysis
// =============================================================================

/// Check if a statement modifies self
pub fn statement_modifies_self(stmt: &Statement) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => {
            // Check if target is self.field
            expression_is_self_field_modification(target)
        }
        Statement::Expression { expr, .. } => {
            // Check for mutating method calls like self.field.push()
            expression_modifies_self(expr)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| statement_modifies_self(s))
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.iter().any(|s| statement_modifies_self(s)))
        }
        Statement::While { body, .. } => body.iter().any(|s| statement_modifies_self(s)),
        Statement::For { iterable, body, .. } => {
            // Check BOTH iterable and body for self mutations
            // e.g., `for x in self.field.values_mut()` requires &mut self
            expression_modifies_self(iterable) || body.iter().any(|s| statement_modifies_self(s))
        }
        Statement::Match { arms, .. } => arms.iter().any(|arm| {
            // Match arms have a body expression, check if it contains modifications
            expression_modifies_self(arm.body)
        }),
        _ => false,
    }
}

/// Check if a statement accesses struct fields
pub fn statement_accesses_fields(ctx: &AnalysisContext, stmt: &Statement) -> bool {
    match stmt {
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expression_accesses_fields(ctx, expr),
        Statement::Let { value, .. } => expression_accesses_fields(ctx, value),
        Statement::Assignment { target, value, .. } => {
            expression_accesses_fields(ctx, target) || expression_accesses_fields(ctx, value)
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_accesses_fields(ctx, condition)
                || then_block.iter().any(|s| statement_accesses_fields(ctx, s))
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.iter().any(|s| statement_accesses_fields(ctx, s)))
        }
        Statement::While {
            condition, body, ..
        } => {
            expression_accesses_fields(ctx, condition)
                || body.iter().any(|s| statement_accesses_fields(ctx, s))
        }
        Statement::For { iterable, body, .. } => {
            expression_accesses_fields(ctx, iterable)
                || body.iter().any(|s| statement_accesses_fields(ctx, s))
        }
        Statement::Match { value, arms, .. } => {
            expression_accesses_fields(ctx, value)
                || arms
                    .iter()
                    .any(|arm| expression_accesses_fields(ctx, arm.body))
        }
        _ => false,
    }
}

/// Check if a statement mutates struct fields
pub fn statement_mutates_fields(ctx: &AnalysisContext, stmt: &Statement) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => {
            // Check if we're assigning to a field: self.field = ...
            expression_is_field_access(ctx, target)
        }
        Statement::Expression { expr, .. } => {
            // Check for mutating method calls on fields: self.field.push(...)
            expression_mutates_fields(ctx, expr)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| statement_mutates_fields(ctx, s))
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.iter().any(|s| statement_mutates_fields(ctx, s)))
        }
        Statement::While { body, .. } => body.iter().any(|s| statement_mutates_fields(ctx, s)),
        Statement::For { iterable, body, .. } => {
            // Check iterable for field mutations too (e.g., self.field.values_mut())
            expression_mutates_fields(ctx, iterable)
                || body.iter().any(|s| statement_mutates_fields(ctx, s))
        }
        Statement::Match { arms, .. } => {
            arms.iter().any(|arm| {
                // MatchArm body is an Expression, need to check for blocks
                expression_mutates_fields(ctx, arm.body)
            })
        }
        _ => false,
    }
}

/// Check if a statement modifies a specific variable
pub fn statement_modifies_variable(stmt: &Statement, var_name: &str) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => {
            // Check if we're assigning to var_name or var_name.field
            expression_references_variable_or_field(target, var_name)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block
                .iter()
                .any(|s| statement_modifies_variable(s, var_name))
                || else_block.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|s| statement_modifies_variable(s, var_name))
                })
        }
        Statement::While { body, .. } | Statement::For { body, .. } => body
            .iter()
            .any(|s| statement_modifies_variable(s, var_name)),
        _ => false,
    }
}

// =============================================================================
// Expression-Level Analysis
// =============================================================================

/// Check if an expression is a self.field modification
pub fn expression_is_self_field_modification(expr: &Expression) -> bool {
    match expr {
        Expression::FieldAccess { object, .. } => {
            matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        }
        _ => false,
    }
}

/// Check if an expression modifies self
pub fn expression_modifies_self(expr: &Expression) -> bool {
    match expr {
        Expression::Block { statements, .. } => {
            statements.iter().any(|s| statement_modifies_self(s))
        }
        Expression::MethodCall { object, method, .. } => {
            // Check if this is a mutating method call on self.field
            // Common mutating methods: push, pop, remove, insert, clear, etc.
            // THE WINDJAMMER WAY: Comprehensive mutation detection
            // Methods ending in _mut are always mutating (values_mut, iter_mut, etc.)
            let is_mutating_method = method.ends_with("_mut")
                || matches!(
                    method.as_str(),
                    "push"
                        | "pop"
                        | "remove"
                        | "insert"
                        | "clear"
                        | "append"
                        | "extend"
                        | "drain"
                        | "truncate"
                        | "resize"
                        | "swap_remove"
                        | "retain"
                        | "sort"
                        | "sort_by"
                        | "sort_by_key"
                        | "sort_unstable"
                        | "sort_unstable_by"
                        | "dedup"
                        | "reverse"
                        | "swap"
                        | "update"
                );

            if is_mutating_method {
                // Check if the object is self.field
                if let Expression::FieldAccess {
                    object: field_obj, ..
                } = &**object
                {
                    if matches!(&**field_obj, Expression::Identifier { name, .. } if name == "self")
                    {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Check if an expression accesses struct fields
pub fn expression_accesses_fields(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier { name, .. } => {
            // Check if this is a field name, but NOT a parameter name
            // Parameters shadow fields, so if it's a parameter, it's not a field access
            let is_param = ctx.current_function_params.iter().any(|p| p.name == *name);
            !is_param && ctx.current_struct_fields.contains(name)
        }
        Expression::FieldAccess { object, .. } => {
            // Check for self.field or nested field access
            if let Expression::Identifier { name: obj_name, .. } = &**object {
                obj_name == "self"
            } else {
                expression_accesses_fields(ctx, object)
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            expression_accesses_fields(ctx, object)
                || arguments
                    .iter()
                    .any(|(_, arg)| expression_accesses_fields(ctx, arg))
        }
        Expression::Call { arguments, .. } => arguments
            .iter()
            .any(|(_, arg)| expression_accesses_fields(ctx, arg)),
        Expression::Binary { left, right, .. } => {
            expression_accesses_fields(ctx, left) || expression_accesses_fields(ctx, right)
        }
        Expression::Unary { operand, .. } => expression_accesses_fields(ctx, operand),
        Expression::Index { object, index, .. } => {
            expression_accesses_fields(ctx, object) || expression_accesses_fields(ctx, index)
        }
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, expr)| expression_accesses_fields(ctx, expr)),
        Expression::MapLiteral { pairs, .. } => pairs
            .iter()
            .any(|(k, v)| expression_accesses_fields(ctx, k) || expression_accesses_fields(ctx, v)),
        Expression::Array { elements, .. } => {
            elements.iter().any(|e| expression_accesses_fields(ctx, e))
        }
        Expression::Tuple { elements, .. } => {
            elements.iter().any(|e| expression_accesses_fields(ctx, e))
        }
        Expression::Closure { body, .. } => expression_accesses_fields(ctx, body),
        Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::Cast { expr, .. } => expression_accesses_fields(ctx, expr),
        Expression::MacroInvocation { args, .. } => {
            // Check if any macro arguments access fields
            args.iter().any(|arg| expression_accesses_fields(ctx, arg))
        }
        Expression::Range { start, end, .. } => {
            expression_accesses_fields(ctx, start) || expression_accesses_fields(ctx, end)
        }
        Expression::ChannelSend { channel, value, .. } => {
            expression_accesses_fields(ctx, channel) || expression_accesses_fields(ctx, value)
        }
        Expression::ChannelRecv { channel, .. } => expression_accesses_fields(ctx, channel),
        Expression::Block { statements, .. } => {
            // Check if any statement in the block accesses fields
            statements.iter().any(|s| statement_accesses_fields(ctx, s))
        }
        _ => false,
    }
}

/// Check if an expression is a field access (self.field or just field)
pub fn expression_is_field_access(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier { name, .. } => ctx.current_struct_fields.contains(name),
        Expression::FieldAccess { object, .. } => {
            if let Expression::Identifier { name: obj_name, .. } = &**object {
                obj_name == "self"
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if an expression mutates struct fields
pub fn expression_mutates_fields(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Block { statements, .. } => {
            // Check if any statement in the block mutates fields
            statements.iter().any(|s| statement_mutates_fields(ctx, s))
        }
        Expression::MethodCall { object, method, .. } => {
            // Check if this is a mutating method call on a field: self.field.push(...)
            if expression_is_field_access(ctx, object) {
                // Methods ending in _mut are always mutating (values_mut, iter_mut, etc.)
                method.ends_with("_mut")
                    || matches!(
                        method.as_str(),
                        "push"
                            | "pop"
                            | "insert"
                            | "remove"
                            | "clear"
                            | "append"
                            | "extend"
                            | "push_str"
                            | "truncate"
                            | "drain"
                            | "retain"
                            | "sort"
                            | "sort_by"
                            | "sort_by_key"
                            | "sort_unstable"
                            | "sort_unstable_by"
                            | "reverse"
                            | "dedup"
                            | "swap"
                            | "fill"
                            | "rotate_left"
                            | "rotate_right"
                            | "update"
                    )
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if an expression references a variable or its fields
pub fn expression_references_variable_or_field(expr: &Expression, var_name: &str) -> bool {
    match expr {
        Expression::Identifier { name, .. } => name == var_name,
        Expression::FieldAccess { object, .. } => {
            // Check if object is the variable
            if let Expression::Identifier { name, .. } = &**object {
                name == var_name
            } else {
                expression_references_variable_or_field(object, var_name)
            }
        }
        _ => false,
    }
}

// =============================================================================
// Loop-Specific Analysis
// =============================================================================

/// Check if a loop body modifies a variable
pub fn loop_body_modifies_variable(body: &[Statement], var_name: &str) -> bool {
    for stmt in body {
        if statement_modifies_variable(stmt, var_name) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_alloc_expr;

    // Basic smoke tests - comprehensive tests are in tests/codegen_self_analysis_test.rs

    #[test]
    fn test_expression_is_self_field_modification_basic() {
        use crate::parser::Expression;
        use crate::source_map::Location;
        use std::path::PathBuf;

        let loc = Location {
            file: PathBuf::from("test.wj"),
            line: 1,
            column: 1,
        };
        let self_expr = test_alloc_expr(Expression::Identifier {
            name: "self".to_string(),
            location: Some(loc.clone()),
        });
        let field_access = Expression::FieldAccess {
            object: self_expr,
            field: "x".to_string(),
            location: Some(loc),
        };

        assert!(expression_is_self_field_modification(&field_access));
    }

    #[test]
    fn test_expression_is_self_field_modification_not_self() {
        use crate::parser::Expression;
        use crate::source_map::Location;
        use std::path::PathBuf;

        let loc = Location {
            file: PathBuf::from("test.wj"),
            line: 1,
            column: 1,
        };
        let other_expr = test_alloc_expr(Expression::Identifier {
            name: "other".to_string(),
            location: Some(loc.clone()),
        });
        let field_access = Expression::FieldAccess {
            object: other_expr,
            field: "x".to_string(),
            location: Some(loc),
        };

        assert!(!expression_is_self_field_modification(&field_access));
    }

    // More comprehensive tests will be added later
    // These are just basic smoke tests for the module
}
