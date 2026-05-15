//! Clone optimization detection, including loop-sensitive usage analysis.

use crate::parser::*;
use std::collections::HashMap;

use super::super::{Analyzer, CloneEliminationReason, CloneOptimization};

impl<'ast> Analyzer<'ast> {
    /// PHASE 2 OPTIMIZATION: Detect unnecessary .clone() calls
    /// Returns a list of clones that can be optimized away
    pub(in crate::analyzer) fn detect_unnecessary_clones(
        &self,
        func: &FunctionDecl,
    ) -> Vec<CloneOptimization> {
        let mut optimizations = Vec::new();

        // Track variable usage: (variable_name, (read_count, write_count, escapes, in_loop))
        let mut usage: HashMap<String, (usize, usize, bool, bool)> = HashMap::new();

        // First pass: analyze usage patterns
        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_statement_for_clones(stmt, &mut usage, idx);
        }

        // Second pass: identify unnecessary clones
        for (var_name, (reads, writes, escapes, in_loop)) in usage {
            // NEVER optimize away clones for variables used in loops
            // Each loop iteration needs its own copy
            if in_loop {
                continue;
            }

            // Clone is unnecessary if:
            // 1. Variable is only read (never written) AND not in loop -> can use borrow
            if writes == 0 && !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name.clone(),
                    location: 0, // TODO: track actual location
                    reason: CloneEliminationReason::OnlyRead,
                });
            }
            // 2. Variable is used once and doesn't escape AND not in loop -> can move
            else if reads == 1 && writes == 0 && !escapes {
                optimizations.push(CloneOptimization {
                    variable: var_name.clone(),
                    location: 0,
                    reason: CloneEliminationReason::SingleUse,
                });
            }
        }

        optimizations
    }
    /// Helper to analyze a statement for clone patterns
    fn analyze_statement_for_clones(
        &self,
        stmt: &Statement,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
        _idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. } => {
                self.analyze_expression_for_clones(value, usage);
            }
            Statement::Assignment { target, value, .. } => {
                // Track writes
                if let Expression::Identifier { name, .. } = target {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.1 += 1; // increment write count
                }
                self.analyze_expression_for_clones(value, usage);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                // Returned values escape the function
                if let Expression::Identifier { name, .. } = expr {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.2 = true; // mark as escapes
                }
                self.analyze_expression_for_clones(expr, usage);
            }
            Statement::Expression { expr, .. } => {
                self.analyze_expression_for_clones(expr, usage);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression_for_clones(condition, usage);
                for stmt in then_block {
                    self.analyze_statement_for_clones(stmt, usage, _idx);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        self.analyze_statement_for_clones(stmt, usage, _idx);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.analyze_expression_for_clones(condition, usage);
                // Mark all variables used in loop body as in_loop
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.analyze_expression_for_clones(iterable, usage);
                // Mark all variables used in loop body as in_loop
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::Loop { body, .. } => {
                // Mark all variables used in loop body as in_loop
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            _ => {}
        }
    }

    /// Helper to analyze a statement in loop context (marks variables as in_loop)
    fn analyze_statement_for_clones_in_loop(
        &self,
        stmt: &Statement,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
        _idx: usize,
    ) {
        match stmt {
            Statement::Let { value, .. } => {
                self.analyze_expression_for_clones_in_loop(value, usage);
            }
            Statement::Assignment { target, value, .. } => {
                if let Expression::Identifier { name, .. } = target {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.1 += 1; // increment write count
                    entry.3 = true; // mark as in_loop
                }
                self.analyze_expression_for_clones_in_loop(value, usage);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                if let Expression::Identifier { name, .. } = expr {
                    let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                    entry.2 = true; // mark as escapes
                    entry.3 = true; // mark as in_loop
                }
                self.analyze_expression_for_clones_in_loop(expr, usage);
            }
            Statement::Expression { expr, .. } => {
                self.analyze_expression_for_clones_in_loop(expr, usage);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression_for_clones_in_loop(condition, usage);
                for stmt in then_block {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.analyze_expression_for_clones_in_loop(condition, usage);
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.analyze_expression_for_clones_in_loop(iterable, usage);
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body {
                    self.analyze_statement_for_clones_in_loop(stmt, usage, _idx);
                }
            }
            _ => {}
        }
    }

    /// Helper to analyze an expression for variable usage in loop context
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_expression_for_clones_in_loop(
        &self,
        expr: &Expression,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                entry.0 += 1; // increment read count
                entry.3 = true; // mark as in_loop
            }
            Expression::Binary { left, right, .. } => {
                self.analyze_expression_for_clones_in_loop(left, usage);
                self.analyze_expression_for_clones_in_loop(right, usage);
            }
            Expression::Unary { operand, .. } => {
                self.analyze_expression_for_clones_in_loop(operand, usage);
            }
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                for (_, arg) in arguments {
                    self.analyze_expression_for_clones_in_loop(arg, usage);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.analyze_expression_for_clones_in_loop(object, usage);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.analyze_expression_for_clones_in_loop(value, usage);
                }
            }
            Expression::Cast { expr, .. } => {
                self.analyze_expression_for_clones_in_loop(expr, usage);
            }
            _ => {}
        }
    }

    /// Helper to analyze an expression for variable usage
    #[allow(clippy::only_used_in_recursion)]
    fn analyze_expression_for_clones(
        &self,
        expr: &Expression,
        usage: &mut HashMap<String, (usize, usize, bool, bool)>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                // Track reads
                let entry = usage.entry(name.clone()).or_insert((0, 0, false, false));
                entry.0 += 1; // increment read count
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.analyze_expression_for_clones(object, usage);
                for (_, arg) in arguments {
                    self.analyze_expression_for_clones(arg, usage);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.analyze_expression_for_clones(function, usage);
                for (_, arg) in arguments {
                    self.analyze_expression_for_clones(arg, usage);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.analyze_expression_for_clones(left, usage);
                self.analyze_expression_for_clones(right, usage);
            }
            Expression::Unary { operand, .. } => {
                self.analyze_expression_for_clones(operand, usage);
            }
            Expression::FieldAccess { object, .. } => {
                self.analyze_expression_for_clones(object, usage);
            }
            Expression::Index { object, index, .. } => {
                self.analyze_expression_for_clones(object, usage);
                self.analyze_expression_for_clones(index, usage);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    self.analyze_expression_for_clones(field_expr, usage);
                }
            }
            Expression::Cast { expr, .. } => {
                self.analyze_expression_for_clones(expr, usage);
            }
            _ => {}
        }
    }
}
