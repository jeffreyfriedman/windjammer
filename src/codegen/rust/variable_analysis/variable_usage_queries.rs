use crate::parser::*;

use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {

    /// Recursively find unused let bindings and for-loop variables in a block of statements.
    pub(super) fn find_unused_bindings(
        stmts: &[&Statement],
        out: &mut std::collections::HashSet<(usize, usize)>,
    ) {
        for (i, stmt) in stmts.iter().enumerate() {
            let binding_info: Option<(&str, &SourceLocation)> = match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(name),
                    location,
                    ..
                } => Some((name.as_str(), location)),
                Statement::Const { name, location, .. } => Some((name.as_str(), location)),
                _ => None,
            };

            if let Some((name, location)) = binding_info {
                let remaining = &stmts[i + 1..];
                if !Self::variable_used_in_statements(remaining, name) {
                    if let Some(loc) = location {
                        out.insert((loc.line, loc.column));
                    }
                }
            }

            match stmt {
                Statement::For {
                    pattern,
                    body,
                    location,
                    ..
                } => {
                    if let Pattern::Identifier(var_name) = pattern {
                        if !Self::variable_used_in_statements(body, var_name) {
                            if let Some(loc) = location {
                                out.insert((loc.line, loc.column));
                            }
                        }
                    }
                    Self::find_unused_bindings(body, out);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    Self::find_unused_bindings(then_block, out);
                    if let Some(else_stmts) = else_block {
                        Self::find_unused_bindings(else_stmts, out);
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    Self::find_unused_bindings(body, out);
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            Self::find_unused_bindings(statements, out);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn variable_used_in_statements(stmts: &[&Statement], var_name: &str) -> bool {
        for stmt in stmts {
            if Self::variable_used_in_statement(stmt, var_name) {
                return true;
            }
        }
        false
    }

    /// Check if a variable name appears in a single statement.
    pub(super) fn variable_used_in_statement(stmt: &Statement, var_name: &str) -> bool {
        match stmt {
            Statement::Let { value, .. } | Statement::Const { value, .. } => {
                Self::variable_used_in_expression(value, var_name)
            }
            Statement::Assignment { target, value, .. } => {
                Self::variable_used_in_expression(target, var_name)
                    || Self::variable_used_in_expression(value, var_name)
            }
            Statement::Expression { expr, .. } => Self::variable_used_in_expression(expr, var_name),
            Statement::Return {
                value: Some(expr), ..
            } => Self::variable_used_in_expression(expr, var_name),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::variable_used_in_expression(condition, var_name)
                    || Self::variable_used_in_statements(then_block, var_name)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| Self::variable_used_in_statements(b, var_name))
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::variable_used_in_expression(condition, var_name)
                    || Self::variable_used_in_statements(body, var_name)
            }
            Statement::For { iterable, body, .. } => {
                Self::variable_used_in_expression(iterable, var_name)
                    || Self::variable_used_in_statements(body, var_name)
            }
            Statement::Loop { body, .. } => Self::variable_used_in_statements(body, var_name),
            Statement::Match { value, arms, .. } => {
                Self::variable_used_in_expression(value, var_name)
                    || arms.iter().any(|arm| {
                        Self::variable_used_in_expression(arm.body, var_name)
                            || arm
                                .guard
                                .as_ref()
                                .is_some_and(|g| Self::variable_used_in_expression(g, var_name))
                    })
            }
            _ => false,
        }
    }

    /// Check if a variable name appears in an expression.
    pub(super) fn variable_used_in_expression(expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::Literal { .. } => false,
            Expression::Identifier { name, .. } => name == var_name,
            Expression::FieldAccess { object, .. } => {
                Self::variable_used_in_expression(object, var_name)
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                Self::variable_used_in_expression(object, var_name)
                    || arguments
                        .iter()
                        .any(|(_, arg)| Self::variable_used_in_expression(arg, var_name))
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                Self::variable_used_in_expression(function, var_name)
                    || arguments
                        .iter()
                        .any(|(_, arg)| Self::variable_used_in_expression(arg, var_name))
            }
            Expression::Binary { left, right, .. } => {
                Self::variable_used_in_expression(left, var_name)
                    || Self::variable_used_in_expression(right, var_name)
            }
            Expression::Unary { operand, .. } => {
                Self::variable_used_in_expression(operand, var_name)
            }
            Expression::Index { object, index, .. } => {
                Self::variable_used_in_expression(object, var_name)
                    || Self::variable_used_in_expression(index, var_name)
            }
            Expression::Block { statements, .. } => {
                Self::variable_used_in_statements(statements, var_name)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, val)| Self::variable_used_in_expression(val, var_name)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                Self::variable_used_in_expression(k, var_name)
                    || Self::variable_used_in_expression(v, var_name)
            }),
            Expression::Range { start, end, .. } => {
                Self::variable_used_in_expression(start, var_name)
                    || Self::variable_used_in_expression(end, var_name)
            }
            Expression::Closure { body, .. } => Self::variable_used_in_expression(body, var_name),
            Expression::Cast { expr, .. } => Self::variable_used_in_expression(expr, var_name),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| Self::variable_used_in_expression(e, var_name)),
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|a| Self::variable_used_in_expression(a, var_name)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                Self::variable_used_in_expression(expr, var_name)
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::variable_used_in_expression(channel, var_name)
                    || Self::variable_used_in_expression(value, var_name)
            }
            Expression::ChannelRecv { channel, .. } => {
                Self::variable_used_in_expression(channel, var_name)
            }
        }
    }
}
