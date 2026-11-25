//! Binding identity for parameters: the part of a function body in which a parameter's NAME still denotes the
//! parameter.
//!
//! The ownership/mutation analyses are keyed by name. A `let`, `for` pattern or `match` arm pattern that rebinds
//! the parameter's name declares a different binding, so a `push`, an assignment or a call argument on that name
//! after the rebinding is not a use of the parameter. [`Analyzer::param_live_body`] hands those analyses a body
//! in which every block is cut at the rebinding `let` (the `let` stays: its initializer still reads the
//! parameter) and every `for` / `match` arm body that rebinds the name is emptied. A body that rebinds nothing
//! is returned borrowed and unchanged.
//!
//! Not cut: closure parameters that shadow the name (closures are expressions; the analyses that read closure
//! bodies see the shadow as the parameter).

use super::Analyzer;
use crate::parser::*;
use std::borrow::Cow;

impl<'ast> Analyzer<'ast> {
    /// The statements of `body` in which `param` still names the parameter (see the module docs).
    pub(crate) fn param_live_body<'b>(param: &str, body: &'b [&'ast Statement<'ast>]) -> Cow<'b, [&'ast Statement<'ast>]> {
        match prune_block(param, body) {
            Some(cut) => Cow::Owned(cut),
            None => Cow::Borrowed(body),
        }
    }
}

pub(crate) fn binds(pattern: &Pattern<'_>, name: &str) -> bool {
    match pattern {
        Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => n == name,
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().any(|p| binds(p, name)),
        Pattern::Reference(inner) => binds(inner, name),
        Pattern::EnumVariant(_, binding) => match binding {
            EnumPatternBinding::Single(n) => n == name,
            EnumPatternBinding::Tuple(ps) => ps.iter().any(|p| binds(p, name)),
            EnumPatternBinding::Struct(fields, _) => fields.iter().any(|(_, p)| binds(p, name)),
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => false,
        },
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => false,
    }
}

/// Leak a rebuilt node: the cut copies live as long as the AST arena would (analysis of a function that
/// shadows one of its parameters; the copies are tiny and only built in that case).
fn alloc<'ast>(stmt: Statement<'ast>) -> &'ast Statement<'ast> {
    Box::leak(Box::new(stmt))
}

/// `None` when nothing in `stmts` rebinds `name`; otherwise the cut copy.
fn prune_block<'ast>(name: &str, stmts: &[&'ast Statement<'ast>]) -> Option<Vec<&'ast Statement<'ast>>> {
    let mut out: Vec<&'ast Statement<'ast>> = Vec::with_capacity(stmts.len());
    let mut changed = false;
    for (i, &stmt) in stmts.iter().enumerate() {
        if let Statement::Let { pattern, .. } = stmt {
            if binds(pattern, name) {
                out.push(stmt);
                // everything after this `let` is about the new binding
                return if i + 1 < stmts.len() || changed { Some(out) } else { None };
            }
        }
        match prune_statement(name, stmt) {
            Some(new) => {
                changed = true;
                out.push(new);
            }
            None => out.push(stmt),
        }
    }
    changed.then_some(out)
}

fn prune_statement<'ast>(name: &str, stmt: &'ast Statement<'ast>) -> Option<&'ast Statement<'ast>> {
    match stmt {
        Statement::If { condition, then_block, else_block, location } => {
            let t = prune_block(name, then_block);
            let e = else_block.as_ref().and_then(|b| prune_block(name, b));
            if t.is_none() && e.is_none() {
                return None;
            }
            Some(alloc(Statement::If {
                condition,
                then_block: t.unwrap_or_else(|| then_block.clone()),
                else_block: match (e, else_block) {
                    (Some(e), _) => Some(e),
                    (None, other) => other.clone(),
                },
                location: location.clone(),
            }))
        }
        Statement::While { condition, body, label, location } => prune_block(name, body).map(|b| alloc(Statement::While { condition, body: b, label: label.clone(), location: location.clone() })),
        Statement::Loop { body, label, location } => prune_block(name, body).map(|b| alloc(Statement::Loop { body: b, label: label.clone(), location: location.clone() })),
        Statement::For { pattern, iterable, body, label, location } => {
            let new_body = if binds(pattern, name) { Some(Vec::new()) } else { prune_block(name, body) };
            new_body.map(|b| alloc(Statement::For { pattern: pattern.clone(), iterable, body: b, label: label.clone(), location: location.clone() }))
        }
        Statement::Match { value, arms, location } => {
            let mut changed = false;
            let new_arms: Vec<MatchArm<'ast>> = arms
                .iter()
                .map(|arm| {
                    let body = if binds(&arm.pattern, name) {
                        changed = true;
                        Some(Vec::new())
                    } else if let Expression::Block { statements, .. } = arm.body {
                        let cut = prune_block(name, statements);
                        changed |= cut.is_some();
                        cut
                    } else {
                        None
                    };
                    match body {
                        Some(statements) => {
                            let (is_unsafe, loc) = match arm.body {
                                Expression::Block { is_unsafe, location, .. } => (*is_unsafe, location.clone()),
                                other => (false, other.location()),
                            };
                            let block: &'ast Expression<'ast> = Box::leak(Box::new(Expression::Block { statements, is_unsafe, location: loc }));
                            MatchArm { pattern: arm.pattern.clone(), guard: arm.guard, body: block }
                        }
                        None => arm.clone(),
                    }
                })
                .collect();
            changed.then(|| alloc(Statement::Match { value, arms: new_arms, location: location.clone() }))
        }
        _ => None,
    }
}
