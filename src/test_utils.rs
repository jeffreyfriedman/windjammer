//! Test Utilities for Arena-Allocated AST Construction
//!
//! This module provides utilities for creating AST nodes in tests after the
//! arena allocation refactoring. Since tests don't need to manage memory
//! efficiently (they're short-lived), we use `Box::leak` to create 'static
//! references that satisfy the arena allocation requirements.
//!
//! IMPORTANT: Only use these helpers in `#[cfg(test)]` code!

use crate::parser::{Expression, Pattern, Statement};

/// Allocate an expression with 'static lifetime for testing
///
/// Uses Box::leak to create a 'static reference. This is acceptable in tests
/// because:
/// - Tests are short-lived processes
/// - Memory is reclaimed when test process exits  
/// - We prioritize test clarity over memory efficiency
pub fn test_alloc_expr<'a>(expr: Expression<'static>) -> &'a Expression<'a> {
    unsafe {
        // Transmute from 'static to any lifetime for test flexibility
        // Safe because tests don't outlive the leaked memory
        std::mem::transmute(Box::leak(Box::new(expr)))
    }
}

/// Allocate a statement with 'static lifetime for testing
pub fn test_alloc_stmt<'a>(stmt: Statement<'static>) -> &'a Statement<'a> {
    unsafe { std::mem::transmute(Box::leak(Box::new(stmt))) }
}

/// Allocate a pattern with 'static lifetime for testing
pub fn test_alloc_pattern<'a>(pattern: Pattern<'static>) -> &'a Pattern<'a> {
    unsafe { std::mem::transmute(Box::leak(Box::new(pattern))) }
}

/// Helper macro to create test expressions more ergonomically
///
/// Usage:
/// ```ignore
/// let expr = test_expr!(Expression::Identifier {
///     name: "x".to_string(),
///     location: None,
/// });
/// ```
#[macro_export]
macro_rules! test_expr {
    ($expr:expr) => {
        $crate::test_utils::test_alloc_expr($expr)
    };
}

/// Helper macro to create test statements more ergonomically
#[macro_export]
macro_rules! test_stmt {
    ($stmt:expr) => {
        $crate::test_utils::test_alloc_stmt($stmt)
    };
}

/// Helper macro to create test patterns more ergonomically
#[macro_export]
macro_rules! test_pattern {
    ($pattern:expr) => {
        $crate::test_utils::test_alloc_pattern($pattern)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Literal;

    #[test]
    fn test_alloc_expr_works() {
        let expr = test_alloc_expr(Expression::Literal {
            value: Literal::Int(42),
            location: None,
        });

        match expr {
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } => assert_eq!(*n, 42),
            _ => panic!("Wrong expression type"),
        }
    }

    #[test]
    fn test_macro_works() {
        let expr = test_expr!(Expression::Literal {
            value: Literal::String("test".to_string()),
            location: None,
        });

        match expr {
            Expression::Literal {
                value: Literal::String(s),
                ..
            } => {
                assert_eq!(s, "test");
            }
            _ => panic!("Wrong expression type"),
        }
    }
}
