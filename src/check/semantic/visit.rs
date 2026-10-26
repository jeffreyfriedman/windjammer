//! A small pre-order AST visitor (statements, expressions, patterns) used by the semantic checks.

use crate::parser::{EnumPatternBinding, Expression, Pattern, Statement};

pub trait Visitor {
    fn stmt(&mut self, _s: &Statement<'_>) {}
    fn expr(&mut self, _e: &Expression<'_>) {}
    /// Called for every pattern node, including nested ones.
    fn pattern(&mut self, _p: &Pattern<'_>) {}
    fn closure(&mut self, _params: &[String]) {}
    /// Entering / leaving a macro or interpolation whose arguments carry foreign locations.
    fn macro_enter(&mut self, _e: &Expression<'_>) {}
    fn macro_exit(&mut self) {}
}

pub fn walk_block<V: Visitor>(v: &mut V, stmts: &[&Statement<'_>]) {
    for s in stmts {
        walk_stmt(v, s);
    }
}

pub fn walk_pattern<V: Visitor>(v: &mut V, p: &Pattern<'_>) {
    v.pattern(p);
    match p {
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|q| walk_pattern(v, q)),
        Pattern::Reference(q) => walk_pattern(v, q),
        Pattern::EnumVariant(_, b) => match b {
            EnumPatternBinding::Tuple(ps) => ps.iter().for_each(|q| walk_pattern(v, q)),
            EnumPatternBinding::Struct(fs, _) => fs.iter().for_each(|(_, q)| walk_pattern(v, q)),
            _ => {}
        },
        _ => {}
    }
}

pub fn walk_stmt<V: Visitor>(v: &mut V, s: &Statement<'_>) {
    v.stmt(s);
    match s {
        Statement::Let {
            pattern,
            value,
            else_block,
            ..
        } => {
            walk_expr(v, value);
            walk_pattern(v, pattern);
            if let Some(b) = else_block {
                walk_block(v, b);
            }
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => walk_expr(v, value),
        Statement::Assignment { target, value, .. } => {
            walk_expr(v, target);
            walk_expr(v, value);
        }
        Statement::Return { value, .. } => {
            if let Some(e) = value {
                walk_expr(v, e);
            }
        }
        Statement::Expression { expr, .. } => walk_expr(v, expr),
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(v, condition);
            walk_block(v, then_block);
            if let Some(b) = else_block {
                walk_block(v, b);
            }
        }
        Statement::Match { value, arms, .. } => {
            walk_expr(v, value);
            for a in arms {
                walk_pattern(v, &a.pattern);
                if let Some(g) = a.guard {
                    walk_expr(v, g);
                }
                walk_expr(v, a.body);
            }
        }
        Statement::For {
            pattern,
            iterable,
            body,
            ..
        } => {
            walk_expr(v, iterable);
            walk_pattern(v, pattern);
            walk_block(v, body);
        }
        Statement::Loop { body, .. }
        | Statement::Thread { body, .. }
        | Statement::Async { body, .. } => walk_block(v, body),
        Statement::While {
            condition, body, ..
        } => {
            walk_expr(v, condition);
            walk_block(v, body);
        }
        Statement::Defer { statement, .. } => walk_stmt(v, statement),
        Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
    }
}

pub fn walk_expr<V: Visitor>(v: &mut V, e: &Expression<'_>) {
    v.expr(e);
    match e {
        Expression::Literal { .. } | Expression::Identifier { .. } => {}
        Expression::Binary { left, right, .. } => {
            walk_expr(v, left);
            walk_expr(v, right);
        }
        Expression::Unary { operand, .. } => walk_expr(v, operand),
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            walk_expr(v, function);
            arguments.iter().for_each(|(_, a)| walk_expr(v, a));
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            walk_expr(v, object);
            arguments.iter().for_each(|(_, a)| walk_expr(v, a));
        }
        Expression::FieldAccess { object, .. } => walk_expr(v, object),
        Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, f)| walk_expr(v, f)),
        Expression::MapLiteral { pairs, .. } => pairs.iter().for_each(|(k, val)| {
            walk_expr(v, k);
            walk_expr(v, val);
        }),
        Expression::Range { start, end, .. } => {
            walk_expr(v, start);
            walk_expr(v, end);
        }
        Expression::Closure {
            parameters, body, ..
        } => {
            v.closure(parameters);
            walk_expr(v, body);
        }
        Expression::Cast { expr, .. }
        | Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::AsyncCall { expr, .. }
        | Expression::SpawnCall { expr, .. } => walk_expr(v, expr),
        Expression::Index { object, index, .. } => {
            walk_expr(v, object);
            walk_expr(v, index);
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            elements.iter().for_each(|x| walk_expr(v, x))
        }
        Expression::MacroInvocation { args, .. } => {
            v.macro_enter(e);
            args.iter().for_each(|x| walk_expr(v, x));
            v.macro_exit();
        }
        Expression::ChannelSend { channel, value, .. } => {
            walk_expr(v, channel);
            walk_expr(v, value);
        }
        Expression::ChannelRecv { channel, .. } => walk_expr(v, channel),
        Expression::Block { statements, .. } => walk_block(v, statements),
    }
}
