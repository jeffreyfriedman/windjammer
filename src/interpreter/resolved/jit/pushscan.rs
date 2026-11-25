//! Which loops get the inline `push` (docs/design/interp-soundness/pushlen.md, "Cost control"): those whose whole body
//! is scalar code and `push` statements. This is a COST filter only: soundness does not depend on it (every helper
//! call flushes the pending length), but a loop with other calls would flush and re-arm on every iteration.

use crate::parser::{Expression, Pattern, Statement};

type S = &'static Statement<'static>;
type E = &'static Expression<'static>;

/// A scalar expression: literals, names, arithmetic, casts. No call, index, field or method.
fn scalar(e: E) -> bool {
    match e {
        Expression::Literal { .. } | Expression::Identifier { .. } => true,
        Expression::Binary { left, right, .. } => scalar(left) && scalar(right),
        Expression::Unary { operand, .. } => scalar(operand),
        Expression::Cast { expr, .. } => scalar(expr),
        _ => false,
    }
}

/// `name.push(scalar)` as an expression statement.
pub(super) fn is_push(s: S) -> bool {
    let Statement::Expression { expr: Expression::MethodCall { object: Expression::Identifier { .. }, method, type_args: None, arguments, .. }, .. } = s else {
        return false;
    };
    method == "push" && matches!(arguments.as_slice(), [(None, a)] if scalar(a))
}

fn stmt_ok(s: S) -> bool {
    match s {
        Statement::Let { pattern: Pattern::Identifier(_) | Pattern::MutBinding(_), value, else_block: None, .. } => scalar(value),
        Statement::Assignment { target: Expression::Identifier { .. }, value, .. } => scalar(value),
        Statement::Expression { .. } => is_push(s),
        Statement::If { condition, then_block, else_block, .. } => scalar(condition) && block_ok(then_block) && else_block.as_deref().is_none_or(block_ok),
        Statement::While { condition, body, .. } => scalar(condition) && block_ok(body),
        Statement::For { iterable: Expression::Range { start, end, .. }, body, .. } => scalar(start) && scalar(end) && block_ok(body),
        Statement::Break { value: None, .. } | Statement::Continue { .. } | Statement::Return { value: None, .. } => true,
        Statement::Return { value: Some(v), .. } => scalar(v),
        _ => false,
    }
}

fn block_ok(list: &[S]) -> bool {
    list.iter().all(|s| stmt_ok(s))
}

fn has_push(list: &[S]) -> bool {
    list.iter().any(|s| match s {
        Statement::Expression { .. } => is_push(s),
        Statement::If { then_block, else_block, .. } => has_push(then_block) || else_block.as_deref().is_some_and(has_push),
        Statement::While { body, .. } | Statement::For { body, .. } => has_push(body),
        _ => false,
    })
}

/// The loop with this condition (`None` for a `for`: its range is scalar-checked by the caller's statement) and body.
pub(super) fn loop_ok(cond: Option<E>, body: &[S]) -> bool {
    cond.is_none_or(scalar) && block_ok(body)
}

/// A `for` statement's own loop test (`for i in a..b`): its bounds are scalar.
pub(super) fn for_ok(iterable: E, body: &[S]) -> bool {
    matches!(iterable, Expression::Range { start, end, .. } if scalar(start) && scalar(end)) && block_ok(body)
}

/// Some loop in `list` (at any depth) is push-only and pushes.
pub(super) fn has_site(list: &[S]) -> bool {
    list.iter().any(|s| site(s))
}

pub(super) fn site(s: S) -> bool {
    match s {
        Statement::While { condition, body, .. } => (loop_ok(Some(condition), body) && has_push(body)) || has_site(body),
        Statement::For { iterable, body, .. } => (for_ok(iterable, body) && has_push(body)) || has_site(body),
        Statement::If { then_block, else_block, .. } => has_site(then_block) || else_block.as_deref().is_some_and(has_site),
        _ => false,
    }
}

/// The statement is a `while` / `for` whose own loop is push-only (the unit loop of a `for` unit).
pub(super) fn own_loop_ok(s: S) -> bool {
    match s {
        Statement::While { condition, body, .. } => loop_ok(Some(condition), body),
        Statement::For { iterable, body, .. } => for_ok(iterable, body),
        _ => false,
    }
}
