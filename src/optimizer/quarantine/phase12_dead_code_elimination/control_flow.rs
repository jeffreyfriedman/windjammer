//! Control-flow helpers for unreachable / empty-body detection.

use crate::parser::Statement;

/// Check if a statement terminates control flow (return, break, continue)
pub(super) fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. }
    )
}

/// Check if a statement is empty and can be removed
pub(super) fn is_empty_statement(stmt: &Statement) -> bool {
    match stmt {
        Statement::If {
            then_block,
            else_block,
            ..
        } => then_block.is_empty() && else_block.as_ref().is_none_or(|e| e.is_empty()),
        Statement::While { body, .. } | Statement::For { body, .. } => body.is_empty(),
        // Match arms always have a body expression, so they're never considered empty
        _ => false,
    }
}
