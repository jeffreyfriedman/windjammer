//! String operation optimization detection (format!, concatenation, etc.).

use crate::parser::*;

use super::super::{Analyzer, StringOptimization, StringOptimizationType};

impl<'ast> Analyzer<'ast> {
    pub(in crate::analyzer) fn detect_string_optimizations(
        &self,
        func: &FunctionDecl,
    ) -> Vec<StringOptimization> {
        let mut optimizations = Vec::new();

        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_string_ops(stmt, &mut optimizations, idx);
        }

        optimizations
    }

    /// Analyze a statement for string optimization opportunities
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_statement_for_string_ops(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<StringOptimization>,
        idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. }
            | Statement::Return {
                value: Some(value), ..
            } => {
                // Check for format! macro calls (string interpolation is converted to format!)
                if let Expression::MacroInvocation { name, .. } = value {
                    if name == "format" {
                        // String interpolation detected - could pre-allocate capacity
                        optimizations.push(StringOptimization {
                            optimization_type: StringOptimizationType::InterpolationWithCapacity,
                            estimated_capacity: Some(64), // Default estimate
                            location: idx,
                        });
                    }
                }
            }
            // Recursively analyze nested blocks
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for nested_stmt in then_block {
                    self.analyze_statement_for_string_ops(nested_stmt, optimizations, idx);
                }
                if let Some(else_b) = else_block {
                    for nested_stmt in else_b {
                        self.analyze_statement_for_string_ops(nested_stmt, optimizations, idx);
                    }
                }
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body, .. } => {
                for nested_stmt in body {
                    self.analyze_statement_for_string_ops(nested_stmt, optimizations, idx);
                }
            }
            _ => {}
        }
    }

    /// Detect concatenation chains (a + b + c + ...)
    #[allow(dead_code)] // TODO: Implement concatenation optimization in future version
    pub(crate) fn detect_concatenation_chain(
        &self,
        expr: &Expression,
        optimizations: &mut Vec<StringOptimization>,
        idx: usize,
    ) {
        let mut concat_count = 0;
        self.count_concatenations(expr, &mut concat_count);

        if concat_count >= 3 {
            // Multiple concatenations, could benefit from pre-allocation
            optimizations.push(StringOptimization {
                optimization_type: StringOptimizationType::ConcatenationChain,
                estimated_capacity: Some(concat_count * 32), // Rough estimate
                location: idx,
            });
        }
    }

    /// Count the number of concatenation operations
    #[allow(dead_code)] // TODO: Implement concatenation optimization in future version
    #[allow(clippy::only_used_in_recursion)]
    fn count_concatenations(&self, expr: &Expression, count: &mut usize) {
        if let Expression::Binary {
            op, left, right, ..
        } = expr
        {
            if matches!(op, BinaryOp::Add) {
                *count += 1;
                self.count_concatenations(left, count);
                self.count_concatenations(right, count);
            }
        }
    }

    /// Check if a statement is accumulating strings (s += ...)
    #[allow(dead_code)] // TODO: Implement loop accumulation optimization in future version
    pub(crate) fn is_string_accumulation(&self, stmt: &Statement) -> bool {
        matches!(
            stmt,
            Statement::Assignment {
                target: Expression::Identifier { .. },
                ..
            }
        )
    }
}
