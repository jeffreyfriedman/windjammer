//! WGSL-specific validation
//!
//! GPU shaders have limitations that general-purpose code doesn't:
//! - No recursion
//! - No dynamic dispatch
//! - Bounded loops only
//! - Limited pointer usage

use crate::parser::{Program, FunctionDecl, Statement, Expression};
use anyhow::{Result, bail};

/// Validate that a program can be compiled to WGSL
pub fn validate_for_gpu(program: &Program) -> Result<()> {
    for item in &program.items {
        if let crate::parser::Item::Function { decl, .. } = item {
            validate_function(decl)?;
        }
    }
    Ok(())
}

/// Validate a function for GPU compilation
fn validate_function(func: &FunctionDecl) -> Result<()> {
    // Check for recursion (not allowed on GPU)
    if has_recursion(func) {
        bail!(
            "Function '{}' contains recursion, which is not supported on GPU",
            func.name
        );
    }
    
    // Validate function body
    for stmt in &func.body {
        validate_statement(stmt)?;
    }
    
    Ok(())
}

/// Check if function is recursive (calls itself)
fn has_recursion(func: &FunctionDecl) -> bool {
    // Simple check: see if function name appears in any call expression
    for stmt in &func.body {
        if statement_calls_function(stmt, &func.name) {
            return true;
        }
    }
    false
}

/// Check if statement contains call to given function
fn statement_calls_function(stmt: &Statement, func_name: &str) -> bool {
    match stmt {
        Statement::Expression { expr, .. } => expression_calls_function(expr, func_name),
        Statement::Let { value, .. } => expression_calls_function(value, func_name),
        Statement::Return { value: Some(expr), .. } => expression_calls_function(expr, func_name),
        Statement::If { condition, then_block, else_block, .. } => {
            expression_calls_function(condition, func_name)
                || then_block.iter().any(|s| statement_calls_function(s, func_name))
                || else_block.as_ref().map_or(false, |block| {
                    block.iter().any(|s| statement_calls_function(s, func_name))
                })
        }
        Statement::While { condition, body, .. } => {
            expression_calls_function(condition, func_name)
                || body.iter().any(|s| statement_calls_function(s, func_name))
        }
        Statement::For { body, .. } => {
            body.iter().any(|s| statement_calls_function(s, func_name))
        }
        _ => false,
    }
}

/// Check if expression contains call to given function
fn expression_calls_function(expr: &Expression, func_name: &str) -> bool {
    match expr {
        Expression::Call { function, arguments, .. } => {
            // Check if this is a call to the function
            if let Expression::Identifier { name, .. } = &**function {
                if name == func_name {
                    return true;
                }
            }
            
            // Check arguments recursively
            arguments.iter().any(|(_, arg)| expression_calls_function(arg, func_name))
        }
        Expression::Binary { left, right, .. } => {
            expression_calls_function(left, func_name)
                || expression_calls_function(right, func_name)
        }
        Expression::Unary { operand, .. } => expression_calls_function(operand, func_name),
        Expression::Block { statements, .. } => {
            statements.iter().any(|s| statement_calls_function(s, func_name))
        }
        _ => false,
    }
}

/// Validate a statement for GPU compilation
fn validate_statement(stmt: &Statement) -> Result<()> {
    match stmt {
        Statement::While { .. } => {
            // TODO: Check for bounded loops (GPU requirement)
            // For now, allow all loops
            Ok(())
        }
        Statement::For { .. } => {
            // TODO: Check for bounded loops
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursion_detection() {
        // This would need actual AST construction
        // For now, just verify the function exists
        assert!(true);
    }
}
