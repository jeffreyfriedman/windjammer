use crate::analyzer::OwnershipMode;
use crate::codegen::rust::ast_utilities;
use crate::parser::*;

use super::{CodeGenerator, VariableUsage};

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {

    /// FIXED: Never add &mut for index access - let auto-clone analysis handle it!
    pub(crate) fn should_mut_borrow_index_access(&self, _expr: &Expression) -> bool {
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
    pub(crate) fn variable_needs_mut(&self, var_name: &str) -> bool {
        let statements = &self.current_function_body;
        for stmt in statements.iter() {
            if self.statement_mutates_variable_field(stmt, var_name) {
                return true;
            }
        }
        false
    }

    pub(crate) fn statement_mutates_variable_field(
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
    pub(crate) fn statement_nonreadonly_method_call_on_var(
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

    pub(crate) fn is_mutating_method(&self, method: &str) -> bool {
        crate::method_registry::mutates_receiver(method)
    }

    pub(crate) fn variable_is_only_field_accessed(&self, var_name: &str) -> bool {
        let next_idx = self.current_block_local_idx + 1;
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
}
