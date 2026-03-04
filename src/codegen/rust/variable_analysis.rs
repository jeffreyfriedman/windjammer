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

use crate::codegen::rust::pattern_analysis;
use crate::codegen::rust::self_analysis;
use crate::parser::*;

use super::CodeGenerator;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum VariableUsage {
    NotUsed,
    FieldAccessOnly,
    Moved,
}

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Pre-scan a function body to find local variables that are iterated in for-loops
    /// and also used after the loop. These need auto-borrow (`&`) in the for-loop.
    pub(super) fn precompute_for_loop_borrows(&mut self, body: &[&'ast Statement<'ast>]) {
        self.for_loop_borrow_needed.clear();
        for (i, stmt) in body.iter().enumerate() {
            if let Statement::For {
                iterable, pattern, ..
            } = stmt
            {
                if let Expression::Identifier { name, .. } = iterable {
                    let is_param = self.current_function_params.iter().any(|p| &p.name == name);
                    if is_param {
                        continue;
                    }

                    let pattern_name = pattern_analysis::extract_pattern_identifier(pattern);
                    if pattern_name.as_deref() == Some(name.as_str()) {
                        continue;
                    }

                    let remaining = &body[i + 1..];
                    if Self::variable_used_in_statements(remaining, name) {
                        self.for_loop_borrow_needed.insert(name.clone());
                    }
                }
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

    /// TDD FIX (Bug #3): Mark variables as usize if they're compared to .len()
    pub(super) fn mark_usize_variables_in_condition(&mut self, condition: &Expression) {
        if let Expression::Binary {
            left, op, right, ..
        } = condition
        {
            let is_comparison = matches!(
                op,
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
            );
            if !is_comparison {
                return;
            }

            if let Expression::Identifier { name, .. } = &**left {
                if self.expression_produces_usize(right) {
                    self.usize_variables.insert(name.clone());
                }
            }

            if let Expression::Identifier { name, .. } = &**right {
                if self.expression_produces_usize(left) {
                    self.usize_variables.insert(name.clone());
                }
            }
        }
    }

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

    pub(super) fn should_borrow_for_iteration(&self, iterable: &Expression) -> bool {
        match iterable {
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { .. } = &**object {
                    return true;
                }
                false
            }
            Expression::Identifier { name, .. } => self.for_loop_borrow_needed.contains(name),
            _ => false,
        }
    }

    /// Check if we're iterating over a borrowed collection
    pub(super) fn is_iterating_over_borrowed(&self, iterable: &Expression) -> bool {
        match iterable {
            Expression::Unary { op, .. } => {
                matches!(op, UnaryOp::Ref | UnaryOp::MutRef)
            }
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return self.current_function_params.iter().any(|p| {
                            p.name == "self"
                                && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                        });
                    }
                    return self.current_function_params.iter().any(|p| {
                        &p.name == name
                            && (matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                                || matches!(
                                    &p.type_,
                                    crate::parser::Type::Reference(_)
                                        | crate::parser::Type::MutableReference(_)
                                ))
                    });
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
            }
            Expression::MethodCall { method, .. } => {
                matches!(
                    method.as_str(),
                    "keys" | "values" | "iter" | "iter_mut" | "lines" | "chars" | "bytes"
                )
            }
            _ => false,
        }
    }

    /// Check if a loop body modifies a variable
    pub(super) fn loop_body_modifies_variable(
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
            Statement::While { body, .. } | Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_modifies_variable(s, var_name)),
            _ => false,
        }
    }

    /// FIXED: Never add &mut for index access - let auto-clone analysis handle it!
    pub(super) fn should_mut_borrow_index_access(&self, _expr: &Expression) -> bool {
        false
    }

    /// TDD: Auto-mutability inference
    pub(super) fn variable_needs_mut(&self, var_name: &str) -> bool {
        let statements = &self.current_function_body;

        for stmt in statements.iter() {
            if self.statement_mutates_variable_field(stmt, var_name) {
                return true;
            }
        }

        false
    }

    fn statement_mutates_variable_field(&self, stmt: &Statement, var_name: &str) -> bool {
        match stmt {
            Statement::Assignment {
                target,
                compound_op,
                ..
            } => {
                if self.expression_is_field_of_variable(target, var_name) {
                    return true;
                }
                if compound_op.is_some() {
                    if let Expression::Identifier { name, .. } = target {
                        return name == var_name;
                    }
                }
                false
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
            _ => false,
        }
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
                            });

                        if let Some(type_name) = type_name {
                            let qualified_name = format!("{}::{}", type_name, method);
                            if let Some(sig) =
                                self.signature_registry.get_signature(&qualified_name)
                            {
                                if sig.has_self_receiver {
                                    if let Some(&crate::analyzer::OwnershipMode::MutBorrowed) =
                                        sig.param_ownership.first()
                                    {
                                        return true;
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
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_mutates_variable_field(arg, var_name)),
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|stmt| self.statement_mutates_variable_field(stmt, var_name)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                self.expression_mutates_variable_field(expr, var_name)
            }
            Expression::Unary { operand, .. } => {
                self.expression_mutates_variable_field(operand, var_name)
            }
            _ => false,
        }
    }

    pub(super) fn is_mutating_method(&self, method: &str) -> bool {
        if matches!(
            method,
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "clear"
                | "append"
                | "extend"
                | "push_front"
                | "push_back"
                | "pop_front"
                | "pop_back"
                | "retain"
                | "dedup"
                | "sort"
                | "reverse"
                | "swap"
                | "drain"
                | "truncate"
                | "resize"
                | "reserve"
                | "shrink_to_fit"
        ) {
            return true;
        }

        if method.starts_with("add_")
            || method.starts_with("remove_")
            || method.starts_with("delete_")
            || method.starts_with("set_")
            || method.starts_with("update_")
            || method.starts_with("reset_")
            || method.starts_with("clear_")
            || method.starts_with("insert_")
            || method.starts_with("append_")
        {
            return true;
        }

        matches!(
            method,
            "increment"
                | "decrement"
                | "add"
                | "subtract"
                | "multiply"
                | "divide"
                | "apply"
                | "modify"
                | "mutate"
                | "change"
                | "toggle"
                | "enable"
                | "disable"
                | "activate"
                | "deactivate"
        )
    }

    pub(super) fn variable_is_only_field_accessed(&self, var_name: &str) -> bool {
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
            _ => VariableUsage::NotUsed,
        }
    }

    /// Check if an expression references `self` (for closure move semantics)
    pub(super) fn expression_references_self(&self, expr: &Expression) -> bool {
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
                for (_, arg) in arguments {
                    if let Expression::Identifier { name, .. } = arg {
                        if name == var_name {
                            return VariableUsage::Moved;
                        }
                    }
                    if let Expression::FieldAccess { object, .. } = arg {
                        if let Expression::Identifier { name, .. } = &**object {
                            if name == var_name {
                                return VariableUsage::FieldAccessOnly;
                            }
                        }
                    }
                }
                VariableUsage::NotUsed
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
