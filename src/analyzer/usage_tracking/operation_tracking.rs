//! Arithmetic and general binary-operator usage (Copy types, owned operands).

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    // TDD FIX (Bug #5): `is_used_in_arithmetic_op` checks ONLY arithmetic ops, not comparisons.
    pub(crate) fn is_used_in_arithmetic_op(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, value) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_arithmetic_op(name, then_block) {
                        return true;
                    }
                    if let Some(else_block) = else_block {
                        if self.is_used_in_arithmetic_op(name, else_block) {
                            return true;
                        }
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_arithmetic_op(name, body) {
                        return true;
                    }
                }
                Statement::For { body, .. } => {
                    if self.is_used_in_arithmetic_op(name, body) {
                        return true;
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn expr_uses_in_arithmetic_op(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary {
                op, left, right, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                // Only check for arithmetic operators, not comparisons
                let is_arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                );

                if is_arithmetic {
                    if self.expr_is_identifier(left, name) || self.expr_is_identifier(right, name) {
                        return true;
                    }
                }
                // Recursively check nested expressions
                self.expr_uses_in_arithmetic_op(name, left)
                    || self.expr_uses_in_arithmetic_op(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_uses_in_arithmetic_op(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_uses_in_arithmetic_op(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_uses_in_arithmetic_op(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_uses_in_arithmetic_op(name, arg))
            }
            Expression::FieldAccess { .. } => false,
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_arithmetic_op(name, object)
                    || self.expr_uses_in_arithmetic_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_arithmetic_op(name, statements),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_arithmetic_op(name, elem)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_arithmetic_op(name, elem)),
            Expression::TryOp { expr, .. } => self.expr_uses_in_arithmetic_op(name, expr),
            _ => false,
        }
    }

    pub(crate) fn is_used_in_binary_op(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expr_uses_in_binary_op(name, value) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expr_uses_in_binary_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_uses_in_binary_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_uses_in_binary_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_binary_op(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_used_in_binary_op(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. }
                | Statement::While { body, .. }
                | Statement::For { body, .. } => {
                    if self.is_used_in_binary_op(name, body) {
                        return true;
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expr_uses_in_binary_op(name, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn expr_uses_in_binary_op(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary { left, right, .. } => {
                // Check if the parameter is directly used in a binary operation
                // This is for Copy types like Vec2, Vec3 where `a + b` requires owned values
                if self.expr_is_identifier(left, name) || self.expr_is_identifier(right, name) {
                    return true;
                }
                // Recursively check nested expressions
                self.expr_uses_in_binary_op(name, left) || self.expr_uses_in_binary_op(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_uses_in_binary_op(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_uses_in_binary_op(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_uses_in_binary_op(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_uses_in_binary_op(name, arg))
            }
            // CRITICAL FIX: Don't recurse into FieldAccess for binary op detection
            // `self.field + value` doesn't mean `self` is used in a binary op
            // We only care about the DIRECT use of the parameter, like `param + value`
            Expression::FieldAccess { .. } => false,
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_binary_op(name, object)
                    || self.expr_uses_in_binary_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_binary_op(name, statements),
            // Recurse into tuple elements
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_binary_op(name, elem)),
            // Recurse into array elements
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_binary_op(name, elem)),
            Expression::TryOp { expr, .. } => self.expr_uses_in_binary_op(name, expr),
            _ => false,
        }
    }
}
