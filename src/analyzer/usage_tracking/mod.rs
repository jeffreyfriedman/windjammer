//! Identifiers in function bodies: returns, storage, iteration, operators, patterns, and let-alias expansion.
//!
//! These walks support [`Analyzer::infer_parameter_ownership`] in `parameter_analysis.rs`.

mod operation_tracking;
mod return_tracking;
mod storage_tracking;

use std::collections::HashSet;

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    pub(crate) fn is_used_in_if_else_expression(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        // Check if parameter is used in an if/else expression
        // Example:
        //   let x = if cond { Thing::new(...) } else { param }
        //
        // When param is in an if/else that gets assigned or returned,
        // it needs to be owned to match the other branch's type

        for stmt in statements {
            if self.stmt_has_if_else_with_param(name, stmt) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_has_if_else_with_param(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let { value, .. } => {
                // let x = if ... { ... } else { param }
                self.expr_is_if_else_with_param(name, value)
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                // return if ... { ... } else { param }
                self.expr_is_if_else_with_param(name, expr)
            }
            Statement::Expression { expr, .. } => {
                // Implicit return or assignment
                self.expr_is_if_else_with_param(name, expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                // Check nested if statements
                self.stmts_have_if_else_with_param(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| self.stmts_have_if_else_with_param(name, block))
            }
            _ => false,
        }
    }

    pub(crate) fn stmts_have_if_else_with_param(&self, name: &str, stmts: &[&'ast Statement<'ast>]) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_has_if_else_with_param(name, stmt))
    }

    pub(crate) fn expr_is_if_else_with_param(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                // Check if block contains an if statement with the parameter
                for stmt in statements {
                    if let Statement::If {
                        then_block,
                        else_block,
                        ..
                    } = stmt
                    {
                        let in_then = self.stmts_mention_identifier(name, then_block);
                        let in_else = else_block
                            .as_ref()
                            .is_some_and(|block| self.stmts_mention_identifier(name, block));
                        if in_then || in_else {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn stmts_mention_identifier(&self, name: &str, stmts: &[&'ast Statement<'ast>]) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_mentions_identifier(name, stmt))
    }

    pub(crate) fn stmt_mentions_identifier(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expr_mentions_identifier(name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_mentions_identifier(name, expr),
            Statement::Let { value, .. } => self.expr_mentions_identifier(name, value),
            _ => false,
        }
    }

    pub(crate) fn expr_mentions_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expr_mentions_identifier(name, object),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_mentions_identifier(name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_mentions_identifier(name, arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_identifier(name, left)
                    || self.expr_mentions_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_mentions_identifier(name, operand),
            Expression::TryOp { expr, .. } => self.expr_mentions_identifier(name, expr),
            _ => false,
        }
    }

    /// TDD: Check if a parameter is iterated over in a for loop (consumed by iteration)
    /// e.g., `for item in items` (not `for item in &items`)
    /// When you iterate over a Vec without `&`, the Vec is consumed and elements are moved.
    pub(crate) fn is_iterated_over(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::For { iterable, body, .. } => {
                    // Check if the iterable is exactly the parameter (direct iteration)
                    if let Expression::Identifier { name: id, .. } = iterable {
                        if id == name {
                            return true;
                        }
                    }

                    // Recursively check nested for loops
                    if self.is_iterated_over(name, body) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_iterated_over(name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_iterated_over(name, else_stmts) {
                            return true;
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if self.is_iterated_over(name, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// `let v = param` (and chains) so that `if let` / `match` on `v` must still require an owned
    /// parameter when the arm moves out of `Option`/`Result`, etc.
    pub(crate) fn simple_let_alias_ids_for_param(
        &self,
        param_name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert(param_name.to_string());
        let mut changed = true;
        while changed {
            changed = false;
            self.simple_let_alias_expand_pass(&mut set, statements, &mut changed);
        }
        set
    }

    pub(crate) fn simple_let_alias_expand_pass(
        &self,
        set: &mut HashSet<String>,
        statements: &[&'ast Statement<'ast>],
        changed: &mut bool,
    ) {
        for stmt in statements {
            match stmt {
                Statement::Let { pattern, value, .. } => {
                    if let Pattern::Identifier(local) = pattern {
                        if let Expression::Identifier { name: src, .. } = &**value {
                            if set.contains(src) && set.insert(local.clone()) {
                                *changed = true;
                            }
                        }
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.simple_let_alias_expand_pass(set, then_block, changed);
                    if let Some(else_b) = else_block {
                        self.simple_let_alias_expand_pass(set, else_b, changed);
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    self.simple_let_alias_expand_pass(set, body, changed);
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.simple_let_alias_expand_pass(set, statements, changed);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check if a parameter is pattern matched with field extraction
    /// e.g., `match param { Enum::Variant { field: f } => ... }`
    /// If we borrow the parameter, `f` becomes a reference, breaking calls expecting owned values
    pub(crate) fn is_pattern_matched_with_fields(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        let aliases = self.simple_let_alias_ids_for_param(name, statements);
        self.match_arm_destructures_enum_subpatterns_in_stmts(&aliases, statements)
    }

    pub(crate) fn match_arm_destructures_enum_subpatterns_in_stmts(
        &self,
        aliases: &HashSet<String>,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Match { value, arms, .. } => {
                    if let Expression::Identifier { name: id, .. } = value {
                        if aliases.contains(id) {
                            for arm in arms {
                                if self.pattern_has_field_bindings(&arm.pattern) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, else_b) {
                            return true;
                        }
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a pattern has field bindings (not just wildcards or simple identifiers)
    pub(crate) fn pattern_has_field_bindings(&self, pattern: &Pattern) -> bool {
        use crate::parser::EnumPatternBinding;

        match pattern {
            Pattern::EnumVariant(_, binding) => {
                // Check if the binding extracts fields
                matches!(
                    binding,
                    EnumPatternBinding::Single(_)
                        | EnumPatternBinding::Tuple(_)
                        | EnumPatternBinding::Struct(_, _)
                )
            }
            Pattern::Tuple(patterns) => patterns.iter().any(|p| self.pattern_has_field_bindings(p)),
            Pattern::Or(patterns) => patterns.iter().any(|p| self.pattern_has_field_bindings(p)),
            _ => false,
        }
    }
}
