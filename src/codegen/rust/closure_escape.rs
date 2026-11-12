//! Which let-bound closures never leave their function.
//!
//! A closure that is only called, or passed as a plain argument to a free function, borrows its
//! captures (the language intent: a captured variable is shared, so a mutation is visible to the
//! enclosing scope). Any other use (returned, stored in a struct/collection, wrapped in `Box`/`Some`,
//! sent to a thread) keeps Rust's `move`. This is a pure syntactic query; ownership of the
//! *captured values* is still decided elsewhere.

use crate::parser::{Expression, Pattern, Statement};
use std::collections::HashSet;

/// Names bound by `let [mut] f = |..| ..` whose every use is a call or a plain call argument.
pub(crate) fn non_escaping_closure_names(body: &[&Statement<'_>]) -> HashSet<String> {
    let mut bound = HashSet::new();
    collect_closure_lets(body, &mut bound);
    if bound.is_empty() {
        return bound;
    }
    let mut escaped = Escapes::default();
    for s in body {
        visit_stmt(s, &mut escaped);
    }
    bound.retain(|n| !escaped.names.contains(n));
    bound
}

/// What the visitor records: every name used other than as a callee or plain call argument (`names`), and the names
/// stored in a place that outlives the call (`stored`): a struct literal's field, an argument of a method on a field
/// (`self.handlers.push(f)`), the value assigned to a field.
#[derive(Default)]
struct Escapes {
    names: HashSet<String>,
    stored: HashSet<String>,
}

/// Names (bare identifiers) a function body stores in a struct field, directly or through a method on a field.
pub(crate) fn names_stored_in_fields(body: &[&Statement<'_>]) -> HashSet<String> {
    let mut esc = Escapes::default();
    for s in body {
        visit_stmt(s, &mut esc);
    }
    esc.stored
}

fn note_stored(e: &Expression<'_>, esc: &mut Escapes) {
    if let Expression::Identifier { name, .. } = e {
        esc.stored.insert(name.clone());
    }
}

fn collect_closure_lets(stmts: &[&Statement<'_>], out: &mut HashSet<String>) {
    for s in stmts {
        match s {
            Statement::Let { pattern, value, .. } => {
                if let (Pattern::Identifier(n) | Pattern::MutBinding(n), Expression::Closure { .. }) =
                    (pattern, value)
                {
                    out.insert(n.clone());
                }
            }
            Statement::If { then_block, else_block, .. } => {
                collect_closure_lets(then_block, out);
                if let Some(b) = else_block {
                    collect_closure_lets(b, out);
                }
            }
            Statement::For { body, .. }
            | Statement::Loop { body, .. }
            | Statement::While { body, .. } => collect_closure_lets(body, out),
            _ => {}
        }
    }
}

fn visit_stmts(stmts: &[&Statement<'_>], esc: &mut Escapes) {
    for s in stmts {
        visit_stmt(s, esc);
    }
}

fn visit_stmt(s: &Statement<'_>, esc: &mut Escapes) {
    match s {
        Statement::Let { value, else_block, .. } => {
            visit_expr(value, esc, false);
            if let Some(b) = else_block {
                visit_stmts(b, esc);
            }
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => visit_expr(value, esc, false),
        Statement::Assignment { target, value, .. } => {
            if matches!(target, Expression::FieldAccess { .. }) {
                note_stored(value, esc);
            }
            visit_expr(target, esc, false);
            visit_expr(value, esc, false);
        }
        Statement::Return { value, .. } => {
            if let Some(v) = value {
                visit_expr(v, esc, false);
            }
        }
        Statement::Expression { expr, .. } => visit_expr(expr, esc, false),
        Statement::If { condition, then_block, else_block, .. } => {
            visit_expr(condition, esc, false);
            visit_stmts(then_block, esc);
            if let Some(b) = else_block {
                visit_stmts(b, esc);
            }
        }
        Statement::Match { value, arms, .. } => {
            visit_expr(value, esc, false);
            for a in arms {
                if let Some(g) = a.guard {
                    visit_expr(g, esc, false);
                }
                visit_expr(a.body, esc, false);
            }
        }
        Statement::For { iterable, body, .. } => {
            visit_expr(iterable, esc, false);
            visit_stmts(body, esc);
        }
        Statement::While { condition, body, .. } => {
            visit_expr(condition, esc, false);
            visit_stmts(body, esc);
        }
        Statement::Loop { body, .. } | Statement::Thread { body, .. } | Statement::Async { body, .. } => {
            visit_stmts(body, esc)
        }
        Statement::Defer { statement, .. } => visit_stmt(statement, esc),
        Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
    }
}

/// `allowed`: this identifier sits in a position that does not let the closure escape.
fn visit_expr(e: &Expression<'_>, esc: &mut Escapes, allowed: bool) {
    match e {
        Expression::Identifier { name, .. } => {
            if !allowed {
                esc.names.insert(name.clone());
            }
        }
        Expression::Literal { .. } => {}
        Expression::Binary { left, right, .. } => {
            visit_expr(left, esc, false);
            visit_expr(right, esc, false);
        }
        Expression::Unary { operand, .. } => visit_expr(operand, esc, false),
        Expression::Call { function, arguments, .. } => {
            let plain_callee = matches!(function, Expression::Identifier { name, .. }
                if !name.contains("::") && name.chars().next().is_some_and(|c| c.is_lowercase() || c == '_'));
            if plain_callee {
                visit_expr(function, esc, true);
            } else {
                visit_expr(function, esc, false);
            }
            for (_, a) in arguments {
                visit_expr(a, esc, plain_callee);
            }
        }
        Expression::MethodCall { object, arguments, .. } => {
            if matches!(object, Expression::FieldAccess { .. }) {
                for (_, a) in arguments {
                    note_stored(a, esc);
                }
            }
            visit_expr(object, esc, false);
            for (_, a) in arguments {
                visit_expr(a, esc, false);
            }
        }
        Expression::FieldAccess { object, .. } => visit_expr(object, esc, false),
        Expression::StructLiteral { fields, .. } => {
            for (_, v) in fields {
                note_stored(v, esc);
                visit_expr(v, esc, false);
            }
        }
        Expression::MapLiteral { pairs, .. } => {
            for (k, v) in pairs {
                visit_expr(k, esc, false);
                visit_expr(v, esc, false);
            }
        }
        Expression::Range { start, end, .. } => {
            visit_expr(start, esc, false);
            visit_expr(end, esc, false);
        }
        Expression::Closure { body, .. } => visit_expr(body, esc, false),
        Expression::Cast { expr, .. }
        | Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::AsyncCall { expr, .. }
        | Expression::SpawnCall { expr, .. } => visit_expr(expr, esc, false),
        Expression::Index { object, index, .. } => {
            visit_expr(object, esc, false);
            visit_expr(index, esc, false);
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            for x in elements {
                visit_expr(x, esc, false);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for x in args {
                visit_expr(x, esc, false);
            }
        }
        Expression::ChannelSend { channel, value, .. } => {
            visit_expr(channel, esc, false);
            visit_expr(value, esc, false);
        }
        Expression::ChannelRecv { channel, .. } => visit_expr(channel, esc, false),
        Expression::Block { statements, .. } => visit_stmts(statements, esc),
    }
}
