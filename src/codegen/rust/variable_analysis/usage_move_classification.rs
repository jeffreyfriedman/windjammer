use crate::parser::*;

use super::{CodeGenerator, VariableUsage};

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Check if an expression references `self` (for closure move semantics)
    pub(crate) fn expression_references_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == "self",
            Expression::FieldAccess { object, .. } => self.expression_references_self(object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_references_self(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_references_self(arg))
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expression_references_self(function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_references_self(arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expression_references_self(left) || self.expression_references_self(right)
            }
            Expression::Unary { operand, .. } => self.expression_references_self(operand),
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|stmt| self.statement_references_self(stmt)),
            _ => false,
        }
    }

    fn statement_references_self(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expression_references_self(value),
            Statement::Assignment { target, value, .. } => {
                self.expression_references_self(target) || self.expression_references_self(value)
            }
            Statement::Return { value, .. } => {
                value.is_some_and(|v| self.expression_references_self(v))
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_references_self(condition)
                    || then_block.iter().any(|s| self.statement_references_self(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_references_self(s))
                    })
            }
            Statement::Match { value, arms, .. } => {
                self.expression_references_self(value)
                    || arms
                        .iter()
                        .any(|arm| self.expression_references_self(arm.body))
            }
            _ => false,
        }
    }

    pub(in crate::codegen::rust::variable_analysis) fn analyze_variable_usage_in_expression(
        &self,
        var_name: &str,
        expr: &Expression,
    ) -> VariableUsage {
        match expr {
            Expression::Identifier { name, .. } if name == var_name => VariableUsage::Moved,
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        return VariableUsage::FieldAccessOnly;
                    }
                }
                VariableUsage::NotUsed
            }
            Expression::Call { arguments, .. } => {
                let mut best = VariableUsage::NotUsed;
                for (_, arg) in arguments {
                    let usage = self.analyze_variable_usage_in_expression(var_name, arg);
                    match usage {
                        VariableUsage::Moved => return VariableUsage::Moved,
                        VariableUsage::FieldAccessOnly => best = VariableUsage::FieldAccessOnly,
                        VariableUsage::NotUsed => {}
                    }
                }
                best
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                let obj_usage = self.analyze_variable_usage_in_expression(var_name, object);
                if matches!(obj_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                let mut best = match obj_usage {
                    VariableUsage::FieldAccessOnly => VariableUsage::FieldAccessOnly,
                    _ => VariableUsage::NotUsed,
                };
                for (_, arg) in arguments {
                    let usage = self.analyze_variable_usage_in_expression(var_name, arg);
                    match usage {
                        VariableUsage::Moved => return VariableUsage::Moved,
                        VariableUsage::FieldAccessOnly => best = VariableUsage::FieldAccessOnly,
                        VariableUsage::NotUsed => {}
                    }
                }
                best
            }
            Expression::StructLiteral { fields, .. } => {
                let mut best = VariableUsage::NotUsed;
                for (_, field_value) in fields {
                    let usage = self.analyze_variable_usage_in_expression(var_name, field_value);
                    match usage {
                        VariableUsage::Moved => return VariableUsage::Moved,
                        VariableUsage::FieldAccessOnly => best = VariableUsage::FieldAccessOnly,
                        VariableUsage::NotUsed => {}
                    }
                }
                best
            }
            Expression::Index { object, index, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        return VariableUsage::FieldAccessOnly;
                    }
                }
                let obj_usage = self.analyze_variable_usage_in_expression(var_name, object);
                let idx_usage = self.analyze_variable_usage_in_expression(var_name, index);
                match (obj_usage, idx_usage) {
                    (VariableUsage::Moved, _) | (_, VariableUsage::Moved) => VariableUsage::Moved,
                    (VariableUsage::FieldAccessOnly, _) | (_, VariableUsage::FieldAccessOnly) => {
                        VariableUsage::FieldAccessOnly
                    }
                    _ => VariableUsage::NotUsed,
                }
            }
            Expression::Binary { left, right, .. } => {
                let left_usage = self.analyze_variable_usage_in_expression(var_name, left);
                if matches!(left_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                let right_usage = self.analyze_variable_usage_in_expression(var_name, right);
                if matches!(right_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }

                match (left_usage, right_usage) {
                    (VariableUsage::FieldAccessOnly, _) => VariableUsage::FieldAccessOnly,
                    (_, VariableUsage::FieldAccessOnly) => VariableUsage::FieldAccessOnly,
                    _ => VariableUsage::NotUsed,
                }
            }
            _ => VariableUsage::NotUsed,
        }
    }
}
