//! Loop Invariant Code Motion (LICM).

use super::loop_analysis::expression_uses_variable;
use crate::parser::Statement;

use super::LoopOptimizationStats;

/// Hoist loop-invariant statements outside the loop
pub(in crate::optimizer) fn hoist_loop_invariants<'ast>(
    body: &[&'ast Statement<'ast>],
    loop_var: &str,
    stats: &mut LoopOptimizationStats,
    _optimizer: &crate::optimizer::Optimizer,
) -> (Vec<&'ast Statement<'ast>>, Vec<&'ast Statement<'ast>>) {
    let mut hoisted = Vec::new();
    let mut remaining = Vec::new();

    for stmt in body {
        if is_loop_invariant(stmt, loop_var) {
            hoisted.push(*stmt);
            stats.invariants_hoisted += 1;
        } else {
            remaining.push(*stmt);
        }
    }

    (hoisted, remaining)
}

/// Check if a statement is loop-invariant (doesn't depend on loop variable)
fn is_loop_invariant<'ast>(stmt: &'ast Statement<'ast>, loop_var: &str) -> bool {
    // Only hoist Let statements that don't depend on the loop variable
    match stmt {
        Statement::Let { value, .. } => !expression_uses_variable(value, loop_var),
        _ => false,
    }
}
