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

static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with `dir` as the working directory. The working directory is process-wide, so every test that changes
/// it goes through here (one lock for the whole test binary, however many modules include a helper); the previous
/// directory is restored even when `f` panics, and falls back to the manifest dir when it was removed meanwhile.
pub fn in_dir<T>(dir: &std::path::Path, f: impl FnOnce() -> T) -> T {
    let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    std::env::set_current_dir(dir).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::env::set_current_dir(&prev).unwrap_or_else(|_| std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap());
    r.unwrap_or_else(|e| std::panic::resume_unwind(e))
}
