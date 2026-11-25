//! Binding identity for `let` initializer lookups: which `let` a name refers to at a given point.

use super::generator::CodeGenerator;
use crate::parser::{Expression, Statement};

impl<'ast> CodeGenerator<'ast> {
    /// The `let` initializer `name` denotes: the first `let name` of the body, or, when `scope` is the
    /// initializer being read, the last one BEFORE that `let`.
    pub(super) fn find_let_rhs_scoped<'b>(
        stmts: &[&'b Statement<'ast>],
        name: &str,
        scope: Option<&Expression<'ast>>,
    ) -> Option<&'b Expression<'ast>> {
        match scope {
            Some(limit) => Self::find_let_rhs_before(stmts, name, limit),
            None => Self::find_let_rhs_in_stmts(stmts, name),
        }
    }

    /// The binding a name refers to INSIDE the initializer `limit` of a `let`: the last `let name` that precedes
    /// that `let` in source order (pre-order walk), compared by node identity. `let n = n * 2` therefore resolves
    /// its inner `n` to the earlier binding (or the parameter, when none), never back to itself.
    fn find_let_rhs_before<'b>(
        stmts: &[&'b crate::parser::Statement<'ast>],
        name: &str,
        limit: &Expression<'ast>,
    ) -> Option<&'b Expression<'ast>> {
        fn walk<'b, 'ast>(
            stmts: &[&'b crate::parser::Statement<'ast>],
            name: &str,
            limit: &Expression<'ast>,
            last: &mut Option<&'b Expression<'ast>>,
        ) -> bool {
            use crate::parser::{Pattern, Statement};
            for stmt in stmts {
                match stmt {
                    Statement::Let { pattern, value, .. } => {
                        if std::ptr::eq::<Expression>(*value, limit) {
                            return true;
                        }
                        if matches!(pattern, Pattern::Identifier(n) | Pattern::MutBinding(n) if n == name) {
                            *last = Some(value);
                        }
                    }
                    Statement::While { body, .. } | Statement::For { body, .. } | Statement::Loop { body, .. } => {
                        if walk(body, name, limit, last) {
                            return true;
                        }
                    }
                    Statement::If { then_block, else_block, .. } => {
                        if walk(then_block, name, limit, last) {
                            return true;
                        }
                        if let Some(eb) = else_block {
                            if walk(eb, name, limit, last) {
                                return true;
                            }
                        }
                    }
                    Statement::Match { arms, .. } => {
                        for arm in arms {
                            if let Expression::Block { statements, .. } = arm.body {
                                if walk(statements, name, limit, last) {
                                    return true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        let mut last = None;
        walk(stmts, name, limit, &mut last);
        last
    }
}
