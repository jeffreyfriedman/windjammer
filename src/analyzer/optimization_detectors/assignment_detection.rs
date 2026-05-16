//! Compound assignment detection (`x = x + y` → `x += y`).

use crate::parser::*;

use super::super::{Analyzer, AssignmentOptimization, CompoundOp};

impl<'ast> Analyzer<'ast> {
    pub(in crate::analyzer) fn detect_assignment_optimizations(
        &self,
        func: &FunctionDecl,
    ) -> Vec<AssignmentOptimization> {
        let mut optimizations = Vec::new();

        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_assignments(stmt, &mut optimizations, idx);
        }

        optimizations
    }

    #[allow(clippy::only_used_in_recursion)]
    fn analyze_statement_for_assignments(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<AssignmentOptimization>,
        idx: usize,
    ) {
        match stmt {
            Statement::Assignment {
                target: Expression::Identifier { name: var_name, .. },
                value:
                    Expression::Binary {
                        left, right: _, op, ..
                    },
                ..
            } => {
                if let Expression::Identifier { name: left_var, .. } = &**left {
                    if left_var == var_name {
                        let compound_op = match op {
                            BinaryOp::Add => Some(CompoundOp::AddAssign),
                            BinaryOp::Sub => Some(CompoundOp::SubAssign),
                            BinaryOp::Mul => Some(CompoundOp::MulAssign),
                            BinaryOp::Div => Some(CompoundOp::DivAssign),
                            _ => None,
                        };

                        if let Some(operation) = compound_op {
                            optimizations.push(AssignmentOptimization {
                                variable: var_name.clone(),
                                location: idx,
                                operation,
                            });
                        }
                    }
                }
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for stmt in then_block {
                    self.analyze_statement_for_assignments(stmt, optimizations, idx);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        self.analyze_statement_for_assignments(stmt, optimizations, idx);
                    }
                }
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => {
                for stmt in body {
                    self.analyze_statement_for_assignments(stmt, optimizations, idx);
                }
            }
            Statement::For { body, .. } => {
                for stmt in body {
                    self.analyze_statement_for_assignments(stmt, optimizations, idx);
                }
            }
            _ => {}
        }
    }
}
