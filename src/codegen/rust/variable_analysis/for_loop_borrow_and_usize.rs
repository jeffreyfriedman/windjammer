use crate::codegen::rust::pattern_analysis;
use crate::parser::*;

use super::CodeGenerator;
use std::collections::HashMap;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Pre-scan a function body (recursively) for `for x in iterable` patterns.
    ///
    /// Insert `iterable` into `for_loop_borrow_needed` when:
    /// - The `for` is nested inside another `for` / `while` / `loop`, so the outer body runs
    ///   multiple times and must not consume the same collection each time (E0382).
    /// - The same identifier appears as a `for` iterable two or more times anywhere in the
    ///   function (sequential loops), so the first loop must not move it (E0382).
    ///
    /// Applies to locals **and** parameters (parameters were incorrectly skipped before).
    pub(crate) fn precompute_for_loop_borrows(&mut self, body: &[&'ast Statement<'ast>]) {
        self.for_loop_borrow_needed.clear();
        let mut counts: HashMap<String, usize> = HashMap::new();
        Self::count_for_loop_iterable_identifiers(body, &mut counts);
        self.precompute_for_loop_borrows_walk(body, 0, &counts);
        self.mark_for_loop_borrow_when_iterable_used_after_siblings(body);
    }

    /// When `for x in items` is followed (in the same block) by statements that use `items`,
    /// the loop must not move the collection — same need as for sequential `for` loops.
    fn mark_for_loop_borrow_when_iterable_used_after_siblings(
        &mut self,
        stmts: &[&'ast Statement<'ast>],
    ) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let Statement::For { iterable, .. } = stmt {
                if let Expression::Identifier { name, .. } = iterable {
                    if Self::variable_used_in_statements(&stmts[i + 1..], name) {
                        self.for_loop_borrow_needed.insert(name.clone());
                    }
                }
            }
            match stmt {
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.mark_for_loop_borrow_when_iterable_used_after_siblings(then_block);
                    if let Some(e) = else_block {
                        self.mark_for_loop_borrow_when_iterable_used_after_siblings(e);
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    self.mark_for_loop_borrow_when_iterable_used_after_siblings(body);
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.mark_for_loop_borrow_when_iterable_used_after_siblings(statements);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn count_for_loop_iterable_identifiers(
        stmts: &[&Statement],
        counts: &mut HashMap<String, usize>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::For { iterable, body, .. } => {
                    if let Expression::Identifier { name, .. } = iterable {
                        *counts.entry(name.clone()).or_insert(0) += 1;
                    }
                    Self::count_for_loop_iterable_identifiers(body, counts);
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    Self::count_for_loop_iterable_identifiers(body, counts);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    Self::count_for_loop_iterable_identifiers(then_block, counts);
                    if let Some(e) = else_block {
                        Self::count_for_loop_iterable_identifiers(e, counts);
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            Self::count_for_loop_iterable_identifiers(statements, counts);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn precompute_for_loop_borrows_walk(
        &mut self,
        stmts: &[&'ast Statement<'ast>],
        loop_depth: usize,
        counts: &HashMap<String, usize>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::For {
                    iterable,
                    pattern,
                    body,
                    ..
                } => {
                    if let Expression::Identifier { name, .. } = iterable {
                        let pattern_name = pattern_analysis::extract_pattern_identifier(pattern);
                        if pattern_name.as_deref() != Some(name.as_str()) {
                            let n = counts.get(name).copied().unwrap_or(0);
                            if loop_depth > 0 || n >= 2 {
                                self.for_loop_borrow_needed.insert(name.clone());
                            }
                        }
                    }
                    self.precompute_for_loop_borrows_walk(body, loop_depth + 1, counts);
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    self.precompute_for_loop_borrows_walk(body, loop_depth + 1, counts);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.precompute_for_loop_borrows_walk(then_block, loop_depth, counts);
                    if let Some(e) = else_block {
                        self.precompute_for_loop_borrows_walk(e, loop_depth, counts);
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.precompute_for_loop_borrows_walk(statements, loop_depth, counts);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    #[allow(dead_code)]
    fn condition_compares_to_usize(&self, condition: &Expression, var_name: &str) -> bool {
        match condition {
            Expression::Binary {
                left, op, right, ..
            } => {
                let left_is_var = matches!(
                    **left,
                    Expression::Identifier { ref name, .. } if name == var_name
                );
                let right_is_var = matches!(
                    **right,
                    Expression::Identifier { ref name, .. } if name == var_name
                );

                let is_comparison = matches!(
                    op,
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
                );

                if !is_comparison {
                    return false;
                }

                if left_is_var {
                    self.expression_produces_usize(right)
                } else if right_is_var {
                    self.expression_produces_usize(left)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    #[allow(dead_code)]
    fn variable_is_incremented_in_body(body: &[&Statement], var_name: &str) -> bool {
        for stmt in body {
            match stmt {
                Statement::Assignment { target, value, .. } => {
                    if let Expression::Identifier { name, .. } = target {
                        if name == var_name {
                            if Self::expression_increments_variable(value, var_name) {
                                return true;
                            }
                        }
                    }
                }
                Statement::While { body, .. }
                | Statement::Loop { body, .. }
                | Statement::For { body, .. } => {
                    if Self::variable_is_incremented_in_body(body, var_name) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if Self::variable_is_incremented_in_body(then_block, var_name) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if Self::variable_is_incremented_in_body(else_stmts, var_name) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    #[allow(dead_code)]
    fn expression_increments_variable(expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::Binary { left, op, .. } => {
                matches!(op, BinaryOp::Add)
                    && matches!(
                        **left,
                        Expression::Identifier { ref name, .. } if name == var_name
                    )
            }
            _ => false,
        }
    }

    #[allow(dead_code)]
    fn variable_used_with_usize_in_function(&self, var_name: &str) -> bool {
        self.variable_used_with_usize_in_statements(&self.current_function_body, var_name)
    }

    #[allow(dead_code)]
    fn variable_used_with_usize_in_statements(&self, stmts: &[&Statement], var_name: &str) -> bool {
        for stmt in stmts {
            match stmt {
                Statement::While {
                    condition, body, ..
                } => {
                    if self.condition_compares_to_usize(condition, var_name) {
                        return true;
                    }
                    if self.variable_used_with_usize_in_statements(body, var_name) {
                        return true;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expression_uses_var_with_usize(condition, var_name) {
                        return true;
                    }
                    if self.variable_used_with_usize_in_statements(then_block, var_name) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.variable_used_with_usize_in_statements(else_stmts, var_name) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. } | Statement::For { body, .. } => {
                    if self.variable_used_with_usize_in_statements(body, var_name) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    #[allow(dead_code)]
    fn expression_uses_var_with_usize(&self, expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::Binary {
                left, op, right, ..
            } => {
                let is_comparison = matches!(
                    op,
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
                );
                if !is_comparison {
                    return false;
                }

                let left_is_var =
                    matches!(**left, Expression::Identifier { ref name, .. } if name == var_name);
                let right_is_var =
                    matches!(**right, Expression::Identifier { ref name, .. } if name == var_name);

                if left_is_var && self.expression_produces_usize(right) {
                    return true;
                }
                if right_is_var && self.expression_produces_usize(left) {
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Mark plain loop counters as `usize` when compared only against `usize` bounds.
    ///
    ///
    /// - `let mut i = 0` + `while i < vec.len()` → `i` is `usize` (push/swap_remove/index need it).
    /// - `while start < num_passes` with `num_passes` from `.len()` → `start` is `usize`.
    ///
    /// Does **not** apply to parameters declared as Windjammer `int` (`idx < vec.len()` stays `int`
    /// and codegen casts `.len()` to `i64`).
    pub(crate) fn mark_usize_variables_in_condition(&mut self, condition: &Expression) {
        self.walk_condition_mark_usize_loop_counters(condition);
    }

    fn walk_condition_mark_usize_loop_counters(&mut self, expr: &Expression) {
        if let Expression::Binary {
            left, op, right, ..
        } = expr
        {
            if matches!(op, BinaryOp::And | BinaryOp::Or) {
                self.walk_condition_mark_usize_loop_counters(left);
                self.walk_condition_mark_usize_loop_counters(right);
                return;
            }
            if matches!(
                op,
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
            ) {
                self.mark_identifier_usize_if_bound_is_usize(left, right);
                self.mark_identifier_usize_if_bound_is_usize(right, left);
            }
        }
    }

    fn mark_identifier_usize_if_bound_is_usize(
        &mut self,
        maybe_counter: &Expression,
        bound: &Expression,
    ) {
        let Expression::Identifier { name, .. } = maybe_counter else {
            return;
        };
        if self
            .current_function_params
            .iter()
            .any(|p| p.name == *name && matches!(&p.type_, Type::Int))
        {
            return;
        }
        if self.expression_is_usize_loop_bound(bound) {
            self.usize_variables.insert(name.clone());
        }
    }

    /// Upper/lower bound that is already `usize` in Rust (no need for the counter to stay `i64`).
    fn expression_is_usize_loop_bound(&self, expr: &Expression) -> bool {
        self.expression_produces_usize(expr)
            || self.infer_expression_type_is_usize(expr)
            || matches!(
                expr,
                Expression::Identifier { name, .. } if self.usize_variables.contains(name)
            )
    }
}
