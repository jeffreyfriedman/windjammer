//! Identities of AST nodes for facts the IR publishes (liveness, clones, numeric widths).
//!
//! The NUM-DEFAULT-INT pre-pass that used to live here is gone: since option C stage C2 the numeric solver
//! publishes every class at its width, the `int` default included (`ir::numeric_facts`), so a literal no use
//! constrains is `int` because the solver says so.

use crate::parser::ast::core::Expression;

/// Stable identity of an AST node for the duration of one compilation (arena address).
pub fn node_id(expr: &Expression<'_>) -> usize {
    expr as *const Expression<'_> as usize
}

/// Key of a literal that survives AST re-allocation between the analyzer and codegen: the
/// literal's own source position (unique among literals, unlike the position *after* an
/// expression that numeric inference uses).
pub fn literal_key(expr: &Expression<'_>) -> Option<(usize, usize)> {
    expr.location().as_ref().map(|l| (l.line, l.column))
}
