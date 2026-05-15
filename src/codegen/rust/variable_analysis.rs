//! Variable Analysis Module
//!
//! Handles variable usage tracking, data flow analysis, borrow inference,
//! and mutability detection for code generation.
//!
//! Includes:
//! - For-loop borrow precomputation
//! - Unused binding detection
//! - Variable mutation analysis
//! - Iteration borrow semantics
//! - Self-reference detection for closures

use crate::analyzer::OwnershipMode;
use crate::codegen::rust::ast_utilities;
use crate::codegen::rust::pattern_analysis;
use crate::codegen::rust::self_analysis;
use crate::parser::*;

use super::CodeGenerator;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::codegen::rust) enum VariableUsage {
    NotUsed,
    FieldAccessOnly,
    Moved,
}

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
    pub(in crate::codegen::rust) fn precompute_for_loop_borrows(&mut self, body: &[&'ast Statement<'ast>]) {
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
    pub(in crate::codegen::rust) fn mark_usize_variables_in_condition(&mut self, condition: &Expression) {
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

    /// Recursively find unused let bindings and for-loop variables in a block of statements.
    pub(in crate::codegen::rust) fn find_unused_bindings(
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

    pub(in crate::codegen::rust) fn variable_used_in_statements(stmts: &[&Statement], var_name: &str) -> bool {
        for stmt in stmts {
            if Self::variable_used_in_statement(stmt, var_name) {
                return true;
            }
        }
        false
    }

    /// Check if a variable name appears in a single statement.
    pub(in crate::codegen::rust) fn variable_used_in_statement(stmt: &Statement, var_name: &str) -> bool {
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
    pub(in crate::codegen::rust) fn variable_used_in_expression(expr: &Expression, var_name: &str) -> bool {
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

    /// TDD FIX for E0507: Check if for-loop should borrow the iterable
    /// Only borrow if the base object is borrowed (not owned)
    pub(in crate::codegen::rust) fn should_borrow_for_iteration(&self, iterable: &Expression) -> bool {
        match iterable {
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return self.inferred_borrowed_params.contains("self")
                            || self.inferred_mut_borrowed_params.contains("self");
                    }
                    return self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name)
                        || self.borrowed_iterator_vars.contains(name);
                }
                if let Expression::FieldAccess { .. } = &**object {
                    return self.should_borrow_for_iteration(object);
                }
                false
            }
            Expression::Identifier { name, .. } => self.for_loop_borrow_needed.contains(name),
            _ => false,
        }
    }

    /// Check if we're iterating over a borrowed collection
    pub(in crate::codegen::rust) fn is_iterating_over_borrowed(&self, iterable: &Expression) -> bool {
        match iterable {
            Expression::Unary { op, .. } => {
                matches!(op, UnaryOp::Ref | UnaryOp::MutRef)
            }
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return self.inferred_borrowed_params.contains("self")
                            || self.inferred_mut_borrowed_params.contains("self");
                    }
                    return self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name)
                        || self.borrowed_iterator_vars.contains(name);
                }
                if let Expression::FieldAccess { .. } = &**object {
                    return self.should_borrow_for_iteration(object);
                }
                false
            }
            Expression::Identifier { name, .. } => {
                self.current_function_params.iter().any(|p| {
                    &p.name == name
                        && (matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                            || matches!(
                                &p.type_,
                                crate::parser::Type::Reference(_)
                                    | crate::parser::Type::MutableReference(_)
                            ))
                }) || self.inferred_borrowed_params.contains(name)
                    || self.borrowed_iterator_vars.contains(name)
                    || self.local_var_types.get(name).is_some_and(|t| {
                        matches!(
                            t,
                            crate::parser::Type::Reference(_)
                                | crate::parser::Type::MutableReference(_)
                        )
                    })
            }
            Expression::MethodCall { method, .. } => {
                super::stdlib_method_traits::method_returns_iterator(method)
            }
            _ => false,
        }
    }

    /// Check if a loop body modifies a variable
    pub(in crate::codegen::rust) fn loop_body_modifies_variable(
        &self,
        body: &[&'ast Statement<'ast>],
        var_name: &str,
    ) -> bool {
        for stmt in body {
            if self.statement_modifies_variable(stmt, var_name) {
                return true;
            }
        }
        false
    }

    /// For `for x in coll` when `coll` is borrowed (`&self.field`): if the body calls a method on
    /// `x` whose inferred receiver is `&mut self` (e.g. `System::update` on `Box<dyn System>`),
    /// the iterable must be `&mut coll`, not `&coll`.
    pub(in crate::codegen::rust) fn loop_body_calls_mut_dispatch_method(
        &self,
        iterable: &Expression<'ast>,
        body: &[&'ast Statement<'ast>],
        loop_var: &str,
    ) -> bool {
        let Some(iter_t) = self.infer_expression_type(iterable) else {
            return false;
        };
        let Some(elem) = Self::extract_iterator_element_type(&iter_t) else {
            return false;
        };
        body.iter()
            .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, &elem))
    }

    fn peel_dispatch_element_type(ty: &Type) -> &Type {
        match ty {
            Type::Parameterized(name, args) if Self::type_base_is_box(name) && args.len() == 1 => {
                Self::peel_dispatch_element_type(&args[0])
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::peel_dispatch_element_type(inner.as_ref())
            }
            _ => ty,
        }
    }

    fn type_base_is_box(name: &str) -> bool {
        name == "Box" || name.ends_with("::Box")
    }

    fn trait_method_self_ownership_for_loop(
        &self,
        trait_name: &str,
        method: &str,
    ) -> Option<crate::analyzer::OwnershipMode> {
        let from_map =
            |m: &std::collections::HashMap<String, crate::analyzer::AnalyzedFunction<'ast>>| {
                m.get(method)
                    .and_then(|af| af.inferred_ownership.get("self").copied())
            };
        if let Some(m) = self.analyzed_trait_methods.get(trait_name) {
            if let Some(o) = from_map(m) {
                return Some(o);
            }
        }
        let suffix = format!("::{}", trait_name);
        for (k, m) in &self.analyzed_trait_methods {
            if k == trait_name || k.ends_with(&suffix) {
                if let Some(o) = from_map(m) {
                    return Some(o);
                }
            }
        }
        None
    }

    pub(in crate::codegen::rust) fn method_requires_mut_receiver_for_element_type(
        &self,
        elem: &Type,
        method: &str,
    ) -> bool {
        let peeled = Self::peel_dispatch_element_type(elem);
        match peeled {
            Type::TraitObject(trait_name) => self
                .trait_method_self_ownership_for_loop(trait_name, method)
                .is_some_and(|o| o == crate::analyzer::OwnershipMode::MutBorrowed),
            Type::Custom(type_name) => self
                .signature_registry
                .get_signature(&format!("{}::{}", type_name, method))
                .filter(|s| s.has_self_receiver)
                .and_then(|s| s.param_ownership.first().copied())
                .is_some_and(|o| o == crate::analyzer::OwnershipMode::MutBorrowed),
            _ => false,
        }
    }

    fn stmt_contains_mut_dispatch_on_var(
        &self,
        stmt: &Statement<'ast>,
        loop_var: &str,
        elem: &Type,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem)
            }
            Statement::Let {
                value, else_block, ..
            } => {
                self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
                    })
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => {
                self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
            }
            Statement::Assignment { target, value, .. } => {
                self.expr_contains_mut_dispatch_on_var(target, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_contains_mut_dispatch_on_var(condition, loop_var, elem)
                    || then_block
                        .iter()
                        .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_contains_mut_dispatch_on_var(condition, loop_var, elem)
                    || body
                        .iter()
                        .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
            }
            Statement::For { iterable, body, .. } => {
                self.expr_contains_mut_dispatch_on_var(iterable, loop_var, elem)
                    || body
                        .iter()
                        .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem)),
            Statement::Match { value, arms, .. } => {
                self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
                    || arms.iter().any(|a| {
                        a.guard.is_some_and(|g| {
                            self.expr_contains_mut_dispatch_on_var(g, loop_var, elem)
                        }) || self.expr_contains_mut_dispatch_on_var(a.body, loop_var, elem)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.stmt_contains_mut_dispatch_on_var(statement, loop_var, elem)
            }
            _ => false,
        }
    }

    fn expr_contains_mut_dispatch_on_var(
        &self,
        expr: &Expression<'ast>,
        loop_var: &str,
        elem: &Type,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == loop_var
                        && self.method_requires_mut_receiver_for_element_type(elem, method)
                    {
                        return true;
                    }
                }
                if self.expr_contains_mut_dispatch_on_var(object, loop_var, elem) {
                    return true;
                }
                arguments
                    .iter()
                    .any(|(_, a)| self.expr_contains_mut_dispatch_on_var(a, loop_var, elem))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_contains_mut_dispatch_on_var(left, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(right, loop_var, elem)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_contains_mut_dispatch_on_var(function, loop_var, elem)
                    || arguments
                        .iter()
                        .any(|(_, a)| self.expr_contains_mut_dispatch_on_var(a, loop_var, elem))
            }
            Expression::Unary { operand, .. } => {
                self.expr_contains_mut_dispatch_on_var(operand, loop_var, elem)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expr_contains_mut_dispatch_on_var(k, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(v, loop_var, elem)
            }),
            Expression::FieldAccess { object, .. } => {
                self.expr_contains_mut_dispatch_on_var(object, loop_var, elem)
            }
            Expression::Index { object, index, .. } => {
                self.expr_contains_mut_dispatch_on_var(object, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(index, loop_var, elem)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expr_contains_mut_dispatch_on_var(v, loop_var, elem)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| self.expr_contains_mut_dispatch_on_var(e, loop_var, elem)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem)
            }
            Expression::Closure { body, .. } => {
                self.expr_contains_mut_dispatch_on_var(body, loop_var, elem)
            }
            Expression::Cast { expr, .. } => {
                self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem)
            }
            Expression::Range { start, end, .. } => {
                self.expr_contains_mut_dispatch_on_var(start, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(end, loop_var, elem)
            }
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|a| self.expr_contains_mut_dispatch_on_var(a, loop_var, elem)),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr_contains_mut_dispatch_on_var(channel, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
            }
            Expression::ChannelRecv { channel, .. } => {
                self.expr_contains_mut_dispatch_on_var(channel, loop_var, elem)
            }
            _ => false,
        }
    }

    fn statement_modifies_variable(&self, stmt: &Statement, var_name: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                self_analysis::expression_references_variable_or_field(target, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_modifies_variable(s, var_name))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_modifies_variable(s, var_name))
                    })
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_modifies_variable(s, var_name)),
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Expression::Block { statements, .. } = arm.body {
                    statements
                        .iter()
                        .any(|s| self.statement_modifies_variable(s, var_name))
                } else {
                    false
                }
            }),
            _ => false,
        }
    }

    /// FIXED: Never add &mut for index access - let auto-clone analysis handle it!
    pub(in crate::codegen::rust) fn should_mut_borrow_index_access(&self, _expr: &Expression) -> bool {
        false
    }

    /// Scan function body for `let var_name = Constructor::new(...)` and extract the type.
    fn infer_local_var_type_from_body(&self, var_name: &str) -> Option<String> {
        for stmt in self.current_function_body.iter() {
            if let Statement::Let { pattern, value, .. } = stmt {
                if let Pattern::Identifier(name) = pattern {
                    if name == var_name {
                        return self.infer_type_from_initializer(value);
                    }
                }
            }
        }
        None
    }

    fn infer_type_from_initializer(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Call { function, .. } => {
                // Case 1: Parser produces FieldAccess for `Type.method()` style
                if let Expression::FieldAccess { object, .. } = &**function {
                    if let Expression::Identifier { name, .. } = &**object {
                        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                            return Some(name.clone());
                        }
                    }
                }
                // Case 2: Parser produces Identifier("Type::method") for `Type::method()` style
                if let Expression::Identifier { name, .. } = &**function {
                    if let Some(type_name) = name.split("::").next() {
                        if type_name.chars().next().is_some_and(|c| c.is_uppercase())
                            && name.contains("::")
                        {
                            return Some(type_name.to_string());
                        }
                    }
                }
                None
            }
            Expression::StructLiteral { name, .. } => name.split('<').next().map(|s| s.to_string()),
            _ => None,
        }
    }

    /// TDD: Auto-mutability inference
    pub(in crate::codegen::rust) fn variable_needs_mut(&self, var_name: &str) -> bool {
        let statements = &self.current_function_body;
        for stmt in statements.iter() {
            if self.statement_mutates_variable_field(stmt, var_name) {
                return true;
            }
        }
        false
    }

    pub(in crate::codegen::rust) fn statement_mutates_variable_field(
        &self,
        stmt: &Statement,
        var_name: &str,
    ) -> bool {
        match stmt {
            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => {
                // Direct reassignment: `x = expr` requires `let mut x`
                if let Expression::Identifier { name, .. } = target {
                    if name == var_name {
                        return true;
                    }
                }
                if self.expression_is_field_of_variable(target, var_name) {
                    return true;
                }
                if compound_op.is_some() {
                    if let Expression::Identifier { name, .. } = target {
                        if name == var_name {
                            return true;
                        }
                    }
                }
                self.expression_mutates_variable_field(value, var_name)
            }
            Statement::Expression { expr, .. } => {
                self.expression_mutates_variable_field(expr, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_mutates_variable_field(s, var_name))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_mutates_variable_field(s, var_name))
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_mutates_variable_field(s, var_name)),
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_mutates_variable_field(s, var_name)),
            Statement::Let { value, .. } | Statement::Const { value, .. } => {
                self.expression_mutates_variable_field(value, var_name)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_mutates_variable_field(expr, var_name),
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Some(g) = arm.guard {
                    if self.expression_mutates_variable_field(g, var_name) {
                        return true;
                    }
                }
                if let Expression::Block { statements, .. } = arm.body {
                    statements
                        .iter()
                        .any(|s| self.statement_mutates_variable_field(s, var_name))
                } else {
                    self.expression_mutates_variable_field(arm.body, var_name)
                }
            }),
            _ => false,
        }
    }

    /// When matching on `&mut slots[i]`, a call `x.foo()` is a write through the borrow unless
    /// `foo` is a known `&self` stdlib API. User methods may still lower to `&self`, but we need
    /// `ref mut x` so updates reach the [`Vec`] element (see mutability_complete_test).
    pub(in crate::codegen::rust) fn statement_nonreadonly_method_call_on_var(
        &self,
        stmt: &Statement,
        var_name: &str,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_nonreadonly_method_call_on_var(expr, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_nonreadonly_method_call_on_var(s, var_name))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_nonreadonly_method_call_on_var(s, var_name))
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_nonreadonly_method_call_on_var(s, var_name)),
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_nonreadonly_method_call_on_var(s, var_name)),
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Some(g) = arm.guard {
                    if self.expression_nonreadonly_method_call_on_var(g, var_name) {
                        return true;
                    }
                }
                if let Expression::Block { statements, .. } = arm.body {
                    statements
                        .iter()
                        .any(|s| self.statement_nonreadonly_method_call_on_var(s, var_name))
                } else {
                    self.expression_nonreadonly_method_call_on_var(arm.body, var_name)
                }
            }),
            _ => false,
        }
    }

    fn expression_nonreadonly_method_call_on_var(&self, expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        return !crate::method_registry::is_known_readonly_method(method);
                    }
                }
                false
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_nonreadonly_method_call_on_var(s, var_name)),
            _ => false,
        }
    }

    /// `f(&mut v)` in the source or codegen requires `v` to be a mutable binding. Resolve the
    /// callee's [SignatureRegistry] entry and see if any argument position is [MutBorrowed] for
    /// this identifier (no hardcoded method names).
    fn call_passes_var_as_mut_borrowed(
        &self,
        function: &Expression,
        arguments: &[(Option<String>, &Expression)],
        var_name: &str,
    ) -> bool {
        let func_name = ast_utilities::extract_function_name(function);
        if func_name.is_empty() {
            return false;
        }
        if self.signature_registry.has_collision(&func_name) {
            return false;
        }
        let Some(sig) = self.signature_registry.get_signature(&func_name) else {
            return false;
        };
        for (i, (_label, arg)) in arguments.iter().enumerate() {
            let pidx = if sig.has_self_receiver {
                i.saturating_add(1)
            } else {
                i
            };
            let Some(&OwnershipMode::MutBorrowed) = sig.param_ownership.get(pidx) else {
                continue;
            };
            let matches_var = |e: &Expression| match e {
                Expression::Identifier { name, .. } => name == var_name,
                Expression::Unary {
                    op: crate::parser::UnaryOp::MutRef,
                    operand,
                    ..
                } => matches!(&**operand, Expression::Identifier { name, .. } if name == var_name),
                _ => false,
            };
            if matches_var(arg) {
                return true;
            }
        }
        false
    }

    fn expression_is_field_of_variable(&self, expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(**object, Expression::Identifier { ref name, .. } if name == var_name)
            }
            _ => false,
        }
    }

    fn expression_mutates_variable_field(&self, expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        if self.is_mutating_method(method) {
                            return true;
                        }

                        let type_name = self
                            .current_function_params
                            .iter()
                            .find(|p| p.name == var_name)
                            .and_then(|p| match &p.type_ {
                                crate::parser::Type::Custom(name) => Some(name.clone()),
                                crate::parser::Type::Parameterized(name, _) => Some(name.clone()),
                                _ => None,
                            })
                            .or_else(|| {
                                self.local_var_types
                                    .get(var_name)
                                    .and_then(Self::type_to_name)
                            })
                            .or_else(|| self.infer_local_var_type_from_body(var_name));

                        if let Some(type_name) = type_name {
                            let qualified_name = format!("{}::{}", type_name, method);
                            if let Some(sig) =
                                self.signature_registry.get_signature(&qualified_name)
                            {
                                if sig.has_self_receiver {
                                    if let Some(ownership) = sig.param_ownership.first() {
                                        if matches!(
                                            ownership,
                                            crate::analyzer::OwnershipMode::MutBorrowed
                                                | crate::analyzer::OwnershipMode::Owned
                                        ) {
                                            return true;
                                        }
                                    }
                                }
                            }

                            // Generic type param: resolve trait bounds and
                            // check if any bound trait declares &mut self
                            for (tp_name, bounds) in &self.current_function_type_bounds {
                                if tp_name == &type_name {
                                    for bound_trait in bounds {
                                        let trait_qualified =
                                            format!("{}::{}", bound_trait, method);
                                        if let Some(sig) =
                                            self.signature_registry.get_signature(&trait_qualified)
                                        {
                                            if sig.has_self_receiver {
                                                if let Some(ownership) = sig.param_ownership.first()
                                                {
                                                    if matches!(
                                                        ownership,
                                                        crate::analyzer::OwnershipMode::MutBorrowed
                                                            | crate::analyzer::OwnershipMode::Owned
                                                    ) {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                false
            }
            Expression::Binary { left, right, .. } => {
                self.expression_mutates_variable_field(left, var_name)
                    || self.expression_mutates_variable_field(right, var_name)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if self.call_passes_var_as_mut_borrowed(function, arguments, var_name) {
                    return true;
                }
                arguments
                    .iter()
                    .any(|(_, arg)| self.expression_mutates_variable_field(arg, var_name))
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|stmt| self.statement_mutates_variable_field(stmt, var_name)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                self.expression_mutates_variable_field(expr, var_name)
            }
            Expression::Unary { op, operand, .. } => {
                if matches!(op, crate::parser::UnaryOp::MutRef) {
                    if let Expression::Identifier { name, .. } = &**operand {
                        if name == var_name {
                            return true;
                        }
                    }
                }
                self.expression_mutates_variable_field(operand, var_name)
            }
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn is_mutating_method(&self, method: &str) -> bool {
        crate::method_registry::mutates_receiver(method)
    }

    pub(in crate::codegen::rust) fn variable_is_only_field_accessed(&self, var_name: &str) -> bool {
        let next_idx = self.current_statement_idx + 1;
        if next_idx >= self.current_function_body.len() {
            return true;
        }

        let statements_after_current = &self.current_function_body[next_idx..];

        for stmt in statements_after_current {
            match self.analyze_variable_usage_in_statement(var_name, stmt) {
                VariableUsage::FieldAccessOnly => continue,
                VariableUsage::Moved => return false,
                VariableUsage::NotUsed => continue,
            }
        }

        true
    }

    fn analyze_variable_usage_in_statement(
        &self,
        var_name: &str,
        stmt: &Statement,
    ) -> VariableUsage {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            } => self.analyze_variable_usage_in_expression(var_name, expr),
            Statement::Expression { expr, .. } => {
                self.analyze_variable_usage_in_expression(var_name, expr)
            }
            Statement::Let { value, .. } => {
                self.analyze_variable_usage_in_expression(var_name, value)
            }
            Statement::Assignment { target, value, .. } => {
                let target_usage = self.analyze_variable_usage_in_expression(var_name, target);
                if matches!(target_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                let value_usage = self.analyze_variable_usage_in_expression(var_name, value);
                if matches!(value_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                if matches!(target_usage, VariableUsage::FieldAccessOnly)
                    || matches!(value_usage, VariableUsage::FieldAccessOnly)
                {
                    VariableUsage::FieldAccessOnly
                } else {
                    VariableUsage::NotUsed
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond_usage = self.analyze_variable_usage_in_expression(var_name, condition);
                if matches!(cond_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }

                for s in then_block {
                    let usage = self.analyze_variable_usage_in_statement(var_name, s);
                    if matches!(usage, VariableUsage::Moved) {
                        return VariableUsage::Moved;
                    }
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        let usage = self.analyze_variable_usage_in_statement(var_name, s);
                        if matches!(usage, VariableUsage::Moved) {
                            return VariableUsage::Moved;
                        }
                    }
                }
                cond_usage
            }
            Statement::While {
                condition, body, ..
            } => {
                let cond_usage = self.analyze_variable_usage_in_expression(var_name, condition);
                if matches!(cond_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                for s in body {
                    let usage = self.analyze_variable_usage_in_statement(var_name, s);
                    if matches!(usage, VariableUsage::Moved) {
                        return VariableUsage::Moved;
                    }
                }
                cond_usage
            }
            Statement::Loop { body, .. } => {
                for s in body {
                    let usage = self.analyze_variable_usage_in_statement(var_name, s);
                    if matches!(usage, VariableUsage::Moved) {
                        return VariableUsage::Moved;
                    }
                }
                VariableUsage::NotUsed
            }
            Statement::For { body, iterable, .. } => {
                let iter_usage = self.analyze_variable_usage_in_expression(var_name, iterable);
                if matches!(iter_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                for s in body {
                    let usage = self.analyze_variable_usage_in_statement(var_name, s);
                    if matches!(usage, VariableUsage::Moved) {
                        return VariableUsage::Moved;
                    }
                }
                iter_usage
            }
            Statement::Match { value, arms, .. } => {
                let value_usage = self.analyze_variable_usage_in_expression(var_name, value);
                if matches!(value_usage, VariableUsage::Moved) {
                    return VariableUsage::Moved;
                }
                for arm in arms {
                    let usage = self.analyze_variable_usage_in_expression(var_name, arm.body);
                    if matches!(usage, VariableUsage::Moved) {
                        return VariableUsage::Moved;
                    }
                }
                value_usage
            }
            _ => VariableUsage::NotUsed,
        }
    }

    /// Check if an expression references `self` (for closure move semantics)
    pub(in crate::codegen::rust) fn expression_references_self(&self, expr: &Expression) -> bool {
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

    fn analyze_variable_usage_in_expression(
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

    /// Forward-scan the current function body for `.push()` / `.insert()` calls on a variable
    /// to infer the collection element type for `Vec::new()` / `HashSet::new()` declarations.
    /// Returns the inferred element `Type` if found.
    pub(in crate::codegen::rust) fn infer_collection_element_type_from_usage(&self, var_name: &str) -> Option<Type> {
        if let Some(ty) = self
            .scan_statements_for_struct_literal_vec_binding(var_name, &self.current_function_body)
        {
            return Some(ty);
        }
        self.scan_statements_for_collection_usage(var_name, &self.current_function_body)
    }

    /// When `data` is moved into `Struct { field: data, ... }` and `field` is `Vec<T>`, infer `T`
    /// for `let mut data = Vec::new()` (fixes `push(0)` typing vs `Vec<u8>` fields).
    fn scan_statements_for_struct_literal_vec_binding(
        &self,
        var_name: &str,
        stmts: &[&Statement<'_>],
    ) -> Option<Type> {
        for stmt in stmts {
            if let Some(ty) = self.check_statement_for_struct_literal_vec_binding(var_name, stmt) {
                return Some(ty);
            }
        }
        None
    }

    fn check_statement_for_struct_literal_vec_binding(
        &self,
        var_name: &str,
        stmt: &Statement<'_>,
    ) -> Option<Type> {
        match stmt {
            Statement::Return { value, .. } => {
                value.and_then(|e| self.check_expr_struct_literal_vec_binding(var_name, e))
            }
            Statement::Expression { expr, .. } => {
                self.check_expr_struct_literal_vec_binding(var_name, expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => self
                .scan_statements_for_struct_literal_vec_binding(var_name, then_block)
                .or_else(|| {
                    else_block.as_ref().and_then(|b| {
                        self.scan_statements_for_struct_literal_vec_binding(var_name, b)
                    })
                }),
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::For { body, .. } => {
                self.scan_statements_for_struct_literal_vec_binding(var_name, body)
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    if let Some(ty) = self.check_expr_struct_literal_vec_binding(var_name, arm.body)
                    {
                        return Some(ty);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn check_expr_struct_literal_vec_binding(
        &self,
        var_name: &str,
        expr: &Expression<'_>,
    ) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, fields, .. } => {
                for (fname, val) in fields {
                    if matches!(
                        val,
                        Expression::Identifier { name: n, .. } if n == var_name
                    ) {
                        if let Some(ft) = self.struct_field_types.get(name) {
                            if let Some(f_ty) = ft.get(fname) {
                                if let Type::Vec(inner) = f_ty {
                                    return Some((**inner).clone());
                                }
                            }
                        }
                    }
                }
                for (_fname, val) in fields {
                    if let Some(ty) = self.check_expr_struct_literal_vec_binding(var_name, val) {
                        return Some(ty);
                    }
                }
                None
            }
            Expression::Block { statements, .. } => {
                self.scan_statements_for_struct_literal_vec_binding(var_name, statements)
            }
            _ => None,
        }
    }

    fn scan_statements_for_collection_usage(
        &self,
        var_name: &str,
        stmts: &[&Statement<'_>],
    ) -> Option<Type> {
        for stmt in stmts {
            if let Some(ty) = self.check_statement_for_collection_usage(var_name, stmt) {
                return Some(ty);
            }
        }
        None
    }

    fn check_statement_for_collection_usage(
        &self,
        var_name: &str,
        stmt: &Statement<'_>,
    ) -> Option<Type> {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.check_expr_for_collection_usage(var_name, expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                if let Some(ty) = self.scan_statements_for_collection_usage(var_name, then_block) {
                    return Some(ty);
                }
                if let Some(else_stmts) = else_block {
                    return self.scan_statements_for_collection_usage(var_name, else_stmts);
                }
                None
            }
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::For { body, .. } => {
                self.scan_statements_for_collection_usage(var_name, body)
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    if let Some(ty) = self.check_expr_for_collection_usage(var_name, arm.body) {
                        return Some(ty);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn check_expr_for_collection_usage(
        &self,
        var_name: &str,
        expr: &Expression<'_>,
    ) -> Option<Type> {
        if let Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } = expr
        {
            let is_target =
                matches!(**object, Expression::Identifier { ref name, .. } if name == var_name);
            if !is_target {
                return None;
            }

            let is_push_or_insert = method == "push" || method == "insert";
            if !is_push_or_insert || arguments.is_empty() {
                return None;
            }

            // For .push(arg), the element type comes from the single argument
            // For .insert(arg), same for HashSet (single arg)
            let arg_expr = &arguments[arguments.len() - 1].1;
            return self.infer_expression_type(arg_expr);
        }
        None
    }
}
