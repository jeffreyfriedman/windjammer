//! Arithmetic and general binary-operator usage (Copy types, owned operands).

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    // TDD FIX (Bug #5): `is_used_in_arithmetic_op` checks ONLY arithmetic ops, not comparisons.
    pub(crate) fn is_used_in_arithmetic_op(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
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
                    // String concatenation: any operand position consumes a `string` param.
                    if matches!(op, BinaryOp::Add) {
                        if self.expr_is_identifier(left, name)
                            || self.expr_is_identifier(right, name)
                        {
                            return true;
                        }
                    } else if self.expr_is_identifier(left, name)
                        || self.expr_is_identifier(right, name)
                    {
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

    /// True when a field/index of `name` is an operand of arithmetic/`+` (string concat).
    /// Distinguishes `deps.tag + ":"` (owned composition) from `graph.count + q` (readonly).
    pub(crate) fn param_projected_field_consumed_in_arithmetic(
        &self,
        param_name: &str,
        param_type: &Type,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. }
                | Statement::Expression { expr: value, .. }
                | Statement::Return {
                    value: Some(value), ..
                }
                | Statement::Assignment { value, .. } => {
                    if self.expr_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        value,
                    ) {
                        return true;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        condition,
                    ) || self.param_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        then_block,
                    ) || else_block.as_ref().is_some_and(|b| {
                        self.param_projected_field_consumed_in_arithmetic(
                            param_name,
                            param_type,
                            b,
                        )
                    }) {
                        return true;
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if self.expr_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        condition,
                    ) || self.param_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        body,
                    ) {
                        return true;
                    }
                }
                Statement::For { body, .. } | Statement::Loop { body, .. } => {
                    if self.param_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        body,
                    ) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expr_projected_field_consumed_in_arithmetic(
        &self,
        param_name: &str,
        param_type: &Type,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::Binary {
                op, left, right, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                // Only `+` on non-Copy / text projected fields (string concat). Numeric
                // `graph.count + q` reads Copy fields and must not force owned `graph`.
                if matches!(op, BinaryOp::Add) {
                    if (self.expr_is_param_field_or_index_value(param_name, left)
                        && self.projected_add_field_consumes_parent(
                            param_name,
                            param_type,
                            left,
                        ))
                        || (self.expr_is_param_field_or_index_value(param_name, right)
                            && self.projected_add_field_consumes_parent(
                                param_name,
                                param_type,
                                right,
                            ))
                    {
                        return true;
                    }
                }
                self.expr_projected_field_consumed_in_arithmetic(param_name, param_type, left)
                    || self.expr_projected_field_consumed_in_arithmetic(
                        param_name,
                        param_type,
                        right,
                    )
            }
            Expression::Unary { operand, .. } | Expression::TryOp { expr: operand, .. } => {
                self.expr_projected_field_consumed_in_arithmetic(param_name, param_type, operand)
            }
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expr_projected_field_consumed_in_arithmetic(param_name, param_type, arg)
            }),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_projected_field_consumed_in_arithmetic(param_name, param_type, object)
                    || arguments.iter().any(|(_, arg)| {
                        self.expr_projected_field_consumed_in_arithmetic(
                            param_name,
                            param_type,
                            arg,
                        )
                    })
            }
            Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, v)| {
                self.expr_projected_field_consumed_in_arithmetic(param_name, param_type, v)
            }),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| {
                    self.expr_projected_field_consumed_in_arithmetic(param_name, param_type, e)
                }),
            Expression::Block { statements, .. } => {
                self.param_projected_field_consumed_in_arithmetic(
                    param_name,
                    param_type,
                    statements,
                )
            }
            _ => false,
        }
    }

    /// True when a projected field in `+` is string-like or non-Copy (owned composition).
    fn projected_add_field_consumes_parent(
        &self,
        param_name: &str,
        param_type: &Type,
        expr: &Expression,
    ) -> bool {
        let Some(field_ty) = self.infer_projected_field_type(param_name, param_type, expr) else {
            return false;
        };
        if Self::is_windjammer_text_param_type(&field_ty) {
            return true;
        }
        !self.is_copy_type(&field_ty)
    }

    fn infer_projected_field_type(
        &self,
        param_name: &str,
        param_type: &Type,
        expr: &Expression,
    ) -> Option<Type> {
        match expr {
            Expression::FieldAccess { object, field, .. } => {
                let base_ty = if self.expr_is_identifier(object, param_name) {
                    param_type.clone()
                } else {
                    self.infer_projected_field_type(param_name, param_type, object)?
                };
                self.lookup_field_type_on_struct(&base_ty, field)
            }
            Expression::Index { object, .. } => {
                let base_ty = if self.expr_is_identifier(object, param_name) {
                    param_type.clone()
                } else {
                    self.infer_projected_field_type(param_name, param_type, object)?
                };
                match &base_ty {
                    Type::Vec(inner) => Some((**inner).clone()),
                    Type::Array(inner, _) => Some((**inner).clone()),
                    Type::Parameterized(name, args) if name == "Vec" && args.len() == 1 => {
                        Some(args[0].clone())
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn expr_is_param_field_or_index_value(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                if self.expr_is_identifier(object, name) {
                    return true;
                }
                // Nested `param.a.b` as a binary operand still consumes projected data.
                matches!(
                    &**object,
                    Expression::FieldAccess { object: inner, .. }
                        | Expression::Index { object: inner, .. }
                        if self.expr_is_identifier(inner, name)
                            || self.expr_is_param_field_or_index_value(name, object)
                )
            }
            _ => false,
        }
    }

    pub(crate) fn is_used_in_binary_op(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
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
