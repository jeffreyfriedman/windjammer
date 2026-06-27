//! Returning `self`, `match self`, moves, and general identifier scans.
use std::collections::HashSet;

use crate::parser::*;

use super::Analyzer;
impl<'ast> Analyzer<'ast> {
    /// Check if a function returns Self (for builder pattern detection)
    pub(super) fn function_returns_self(&self, func: &FunctionDecl) -> bool {
        use crate::parser::{Statement, Type};

        // "returns Self" means the return type matches the parent type
        // (the type that `self` belongs to), indicating a builder pattern
        let parent_type = match &func.parent_type {
            Some(name) => name,
            None => return false,
        };
        let return_type_name = match &func.return_type {
            Some(Type::Custom(name)) if name == parent_type => name,
            _ => return false,
        };

        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => self.expression_returns_self_type(expr, return_type_name),
                Statement::Expression { expr, .. } => {
                    self.expression_returns_self_type(expr, return_type_name)
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// True when the method returns a new instance of the parent type built from `self.field`
    /// reads (snapshot/clone/factory), not by consuming bare `self`.
    ///
    /// Example: `fn snapshot(self) -> Scene { Scene { name: self.name, ... } }` → `&self`
    pub(super) fn function_returns_new_instance_from_self_fields(
        &self,
        func: &FunctionDecl,
    ) -> bool {
        use crate::parser::{Statement, Type};

        // Mutating `self` then returning `Self { field: self.field, ... }` moves fields; that is
        // a consuming builder, not a read-only snapshot that codegen can serve with `&self` + clone.
        if self.function_modifies_self_fields_with_registry(func, None) {
            return false;
        }

        let parent_type = match &func.parent_type {
            Some(name) => name,
            None => return false,
        };
        let return_type_name = match &func.return_type {
            Some(Type::Custom(name)) if name == parent_type => name,
            _ => return false,
        };

        let return_expr = match func.body.last() {
            Some(Statement::Return {
                value: Some(expr), ..
            }) => expr,
            Some(Statement::Expression { expr, .. }) => expr,
            _ => return false,
        };

        match return_expr {
            Expression::StructLiteral { name, fields, .. } if name == return_type_name => {
                // Consuming builder embeds bare `self`; snapshot/factory reads fields instead.
                !fields.iter().any(
                    |(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self"),
                )
            }
            _ => false,
        }
    }

    /// True when the function returns a non-Copy `self.field` expression (last statement).
    /// Check if self is moved into a returned struct literal (e.g., `OtherType { field: self }`)
    /// or returned directly as a value. This means self must be consumed (owned).
    pub(super) fn function_moves_self_into_return(&self, func: &FunctionDecl) -> bool {
        use crate::parser::Statement;
        if let Some(last_stmt) = func.body.last() {
            let expr = match last_stmt {
                Statement::Return { value: Some(e), .. } => Some(e),
                Statement::Expression { expr, .. } => Some(expr),
                _ => None,
            };
            if let Some(expr) = expr {
                return self.expression_consumes_self(expr);
            }
        }
        false
    }

    pub(crate) fn expression_consumes_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self")),
            _ => false,
        }
    }

    /// Check if bare `self` is consumed anywhere in the body (not just as a return value).
    /// Catches patterns like `let x = self` which moves self into a local variable.
    pub(super) fn function_body_consumes_bare_self(&self, func: &FunctionDecl) -> bool {
        func.body
            .iter()
            .any(|stmt| self.statement_consumes_bare_self(stmt))
    }

    fn statement_consumes_bare_self(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let { value, .. } => {
                matches!(value, Expression::Identifier { name, .. } if name == "self")
            }
            Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Expression { expr, .. } => {
                matches!(expr, Expression::Identifier { name, .. } if name == "self")
                    || matches!(
                        expr,
                        Expression::StructLiteral { fields, .. }
                            if fields.iter().any(|(_, v)| {
                                matches!(v, Expression::Identifier { name, .. } if name == "self")
                            })
                    )
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_consumes_bare_self(s))
                    || else_block
                        .as_ref()
                        .is_some_and(|b| b.iter().any(|s| self.statement_consumes_bare_self(s)))
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Expression::Block { statements, .. } = arm.body {
                    statements
                        .iter()
                        .any(|s| self.statement_consumes_bare_self(s))
                } else {
                    false
                }
            }),
            _ => false,
        }
    }

    /// Check if `self` is passed as receiver to a method that takes owned self.
    /// e.g., `self.input_uniform(buffer)` where `input_uniform` takes `self` (owned).
    pub(super) fn function_calls_consuming_method_on_self(
        &self,
        func: &FunctionDecl,
        registry: &super::SignatureRegistry,
    ) -> bool {
        let mut visited = HashSet::new();
        self.function_calls_consuming_method_on_self_with_visited(func, registry, &mut visited)
    }

    pub(super) fn function_calls_consuming_method_on_self_with_visited(
        &self,
        func: &FunctionDecl,
        registry: &super::SignatureRegistry,
        visited: &mut HashSet<String>,
    ) -> bool {
        func.body.iter().any(|stmt| {
            self.statement_calls_consuming_method_on_self(
                stmt,
                registry,
                func.parent_type.as_deref(),
                Some(func.name.as_str()),
                visited,
            )
        })
    }

    /// True when the body calls `self.callee(...)` and callee requires owned `self` after inference.
    pub(super) fn function_calls_explicit_owned_self_method(
        &self,
        func: &FunctionDecl,
        registry: &super::SignatureRegistry,
    ) -> bool {
        func.body.iter().any(|stmt| {
            self.statement_calls_explicit_owned_self_method(
                stmt,
                func.parent_type.as_deref(),
                registry,
            )
        })
    }

    /// Count `self.{method}(...)` calls in a function body (including nested blocks).
    pub(super) fn count_self_method_calls(&self, func: &FunctionDecl, method: &str) -> usize {
        func.body
            .iter()
            .map(|stmt| self.statement_count_self_method_calls(stmt, method))
            .sum()
    }

    fn statement_count_self_method_calls(&self, stmt: &Statement, method: &str) -> usize {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_count_self_method_calls(expr, method)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_count_self_method_calls(expr, method),
            Statement::Let { value, .. } => self.expression_count_self_method_calls(value, method),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .map(|s| self.statement_count_self_method_calls(s, method))
                    .sum::<usize>()
                    + else_block.as_ref().map_or(0, |b| {
                        b.iter()
                            .map(|s| self.statement_count_self_method_calls(s, method))
                            .sum::<usize>()
                    })
            }
            Statement::Match { arms, .. } => arms
                .iter()
                .map(|arm| {
                    if let Expression::Block { statements, .. } = arm.body {
                        statements
                            .iter()
                            .map(|s| self.statement_count_self_method_calls(s, method))
                            .sum::<usize>()
                    } else {
                        self.expression_count_self_method_calls(arm.body, method)
                    }
                })
                .sum(),
            Statement::For { body, .. } | Statement::While { body, .. } => body
                .iter()
                .map(|s| self.statement_count_self_method_calls(s, method))
                .sum(),
            _ => 0,
        }
    }

    fn expression_count_self_method_calls(&self, expr: &Expression, method: &str) -> usize {
        match expr {
            Expression::MethodCall {
                object,
                method: callee,
                ..
            } => {
                let direct = if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    && callee.as_str() == method
                {
                    1
                } else {
                    0
                };
                direct + self.expression_count_self_method_calls(object, method)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .map(|s| self.statement_count_self_method_calls(s, method))
                .sum(),
            Expression::Binary { left, right, .. } => {
                self.expression_count_self_method_calls(left, method)
                    + self.expression_count_self_method_calls(right, method)
            }
            Expression::Unary { operand, .. } => {
                self.expression_count_self_method_calls(operand, method)
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .map(|(_, arg)| self.expression_count_self_method_calls(arg, method))
                .sum(),
            _ => 0,
        }
    }

    fn statement_calls_explicit_owned_self_method(
        &self,
        stmt: &Statement,
        parent_type: Option<&str>,
        registry: &super::SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_calls_explicit_owned_self_method(expr, parent_type, registry)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_calls_explicit_owned_self_method(expr, parent_type, registry),
            Statement::Let { value, .. } => {
                self.expression_calls_explicit_owned_self_method(value, parent_type, registry)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| {
                    self.statement_calls_explicit_owned_self_method(s, parent_type, registry)
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.statement_calls_explicit_owned_self_method(s, parent_type, registry)
                    })
                })
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Expression::Block { statements, .. } = arm.body {
                    statements.iter().any(|s| {
                        self.statement_calls_explicit_owned_self_method(s, parent_type, registry)
                    })
                } else {
                    self.expression_calls_explicit_owned_self_method(
                        arm.body,
                        parent_type,
                        registry,
                    )
                }
            }),
            _ => false,
        }
    }

    fn expression_calls_explicit_owned_self_method(
        &self,
        expr: &Expression,
        parent_type: Option<&str>,
        registry: &super::SignatureRegistry,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                let is_self_receiver =
                    matches!(&**object, Expression::Identifier { name, .. } if name == "self");
                if is_self_receiver {
                    if let Some(pt) = parent_type {
                        let key = format!("{}::{}", pt, method);
                        if let Some(sig) = registry.get_signature(&key) {
                            return sig.has_self_receiver
                                && sig.param_ownership.first()
                                    == Some(&super::OwnershipMode::Owned);
                        }
                    }
                }
                self.expression_calls_explicit_owned_self_method(object, parent_type, registry)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_calls_explicit_owned_self_method(s, parent_type, registry)),
            _ => false,
        }
    }

    fn statement_calls_consuming_method_on_self(
        &self,
        stmt: &Statement,
        registry: &super::SignatureRegistry,
        parent_type: Option<&str>,
        caller_name: Option<&str>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expression_calls_consuming_method_on_self(
                expr,
                registry,
                parent_type,
                caller_name,
                visited,
            ),
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_calls_consuming_method_on_self(
                expr,
                registry,
                parent_type,
                caller_name,
                visited,
            ),
            Statement::Let { value, .. } => self.expression_calls_consuming_method_on_self(
                value,
                registry,
                parent_type,
                caller_name,
                visited,
            ),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| {
                    self.statement_calls_consuming_method_on_self(
                        s,
                        registry,
                        parent_type,
                        caller_name,
                        visited,
                    )
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.statement_calls_consuming_method_on_self(
                            s,
                            registry,
                            parent_type,
                            caller_name,
                            visited,
                        )
                    })
                })
            }
            _ => false,
        }
    }

    fn expression_calls_consuming_method_on_self(
        &self,
        expr: &Expression,
        registry: &super::SignatureRegistry,
        parent_type: Option<&str>,
        caller_name: Option<&str>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                let is_self_receiver =
                    matches!(&**object, Expression::Identifier { name, .. } if name == "self");
                if is_self_receiver {
                    if caller_name.is_some_and(|n| n == method.as_str()) {
                        return true;
                    }
                    if let Some(impl_functions) = &self.current_impl_functions {
                        if let Some(called_func) = impl_functions.get(method.as_str()) {
                            let callee_self_owned = called_func
                                .parameters
                                .iter()
                                .find(|p| p.name == "self")
                                .is_some_and(|p| {
                                    matches!(p.ownership, OwnershipHint::Owned)
                                        || self.infer_impl_self_receiver_ownership_inner(
                                            called_func,
                                            registry,
                                            visited,
                                        ) == super::OwnershipMode::Owned
                                });
                            if callee_self_owned {
                                return true;
                            }
                            if let Some(pt) = parent_type {
                                let key = format!("{}::{}", pt, method);
                                if let Some(sig) = registry.get_signature(&key) {
                                    if sig.has_self_receiver
                                        && sig.param_ownership.first()
                                            == Some(&super::OwnershipMode::Owned)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    if let Some(pt) = parent_type {
                        let key = format!("{}::{}", pt, method);
                        if let Some(sig) = registry.get_signature(&key) {
                            if sig.has_self_receiver
                                && sig
                                    .param_ownership
                                    .first()
                                    .is_some_and(|o| *o == super::OwnershipMode::Owned)
                            {
                                return true;
                            }
                        }
                    }
                }
                self.expression_calls_consuming_method_on_self(
                    object,
                    registry,
                    parent_type,
                    caller_name,
                    visited,
                )
            }
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.statement_calls_consuming_method_on_self(
                    s,
                    registry,
                    parent_type,
                    caller_name,
                    visited,
                )
            }),
            _ => false,
        }
    }

    /// Check if `self` is used as a match scrutinee (match self { ... })
    /// AND the match arms actually consume values from self.
    ///
    /// TDD FIX for E0606: match self { Value::Int(v) => v as f32, ... }
    /// If match arms return/use bound variables, self must be owned.
    /// If match arms only return literals, &self is sufficient.
    ///
    /// Examples:
    ///   match self { Value::Int(v) => v as f32 } → needs `self` (v is used)
    ///   match self { Condition::HasItem(_) => false } → needs `&self` (literal returned)
    pub(super) fn function_matches_on_self(&self, func: &FunctionDecl) -> bool {
        func.body
            .iter()
            .any(|stmt| self.statement_matches_on_self_consuming(stmt))
    }

    /// Check if function iterates over self.field and calls consuming methods on elements.
    ///
    /// TDD FIX for E0507: for cond in self.conditions { cond.check() }
    /// If loop elements call methods that consume self, the outer method must consume self.
    ///
    /// Example:
    ///   for item in self.items { item.consume() } → needs `self` (owned)
    pub(super) fn function_consumes_self_field_elements(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        func.body
            .iter()
            .any(|stmt| self.statement_consumes_self_field_elements(stmt, registry))
    }

    pub(crate) fn statement_consumes_self_field_elements(
        &self,
        stmt: &Statement,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                if !self.expression_is_self_field(iterable) {
                    return false;
                }

                let loop_var = match pattern {
                    Pattern::Identifier(name) => name.clone(),
                    _ => return false,
                };

                let elem_type = self.resolve_for_loop_element_type(iterable);

                body.iter().any(|s| {
                    self.statement_calls_consuming_method_on_var_typed(
                        s,
                        &loop_var,
                        elem_type.as_ref(),
                        registry,
                    )
                })
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_consumes_self_field_elements(s, registry))
                    || else_block.as_ref().is_some_and(|body| {
                        body.iter()
                            .any(|s| self.statement_consumes_self_field_elements(s, registry))
                    })
            }
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.statement_consumes_self_field_elements(s, registry)),
            _ => false,
        }
    }

    pub(crate) fn expression_is_self_field(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    /// Resolve the element type of a `self.field` iterable.
    /// e.g. `self.conditions` where `conditions: Vec<Cond>` → `Some(Type::Custom("Cond"))`
    fn resolve_for_loop_element_type(&self, iterable: &Expression) -> Option<Type> {
        if let Expression::FieldAccess { field, .. } = iterable {
            let field_type = self.lookup_field_type_for_self(field)?;
            Self::vec_element_type(&field_type)
        } else {
            None
        }
    }

    fn vec_element_type(ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some(inner.as_ref().clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => Self::vec_element_type(inner),
            _ => None,
        }
    }

    /// Type-qualified version: uses element type for `Type::method` registry lookup.
    pub(crate) fn statement_calls_consuming_method_on_var_typed(
        &self,
        stmt: &Statement,
        var_name: &str,
        elem_type: Option<&Type>,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Let { value: expr, .. }
            | Statement::Assignment { value: expr, .. } => self
                .expression_calls_consuming_method_on_var_typed(
                    expr, var_name, elem_type, registry,
                ),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_calls_consuming_method_on_var_typed(
                    condition, var_name, elem_type, registry,
                ) || then_block.iter().any(|s| {
                    self.statement_calls_consuming_method_on_var_typed(
                        s, var_name, elem_type, registry,
                    )
                }) || else_block.as_ref().is_some_and(|body| {
                    body.iter().any(|s| {
                        self.statement_calls_consuming_method_on_var_typed(
                            s, var_name, elem_type, registry,
                        )
                    })
                })
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body, .. } => body.iter().any(|s| {
                self.statement_calls_consuming_method_on_var_typed(s, var_name, elem_type, registry)
            }),
            _ => false,
        }
    }

    pub(crate) fn expression_calls_consuming_method_on_var_typed(
        &self,
        expr: &Expression,
        var_name: &str,
        elem_type: Option<&Type>,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        if let Some(reg) = registry {
                            // Type-qualified lookup first (e.g. "Cond::check")
                            if let Some(type_name) = elem_type.and_then(|t| match t {
                                Type::Custom(n) => Some(n.as_str()),
                                _ => None,
                            }) {
                                let qualified = format!("{}::{}", type_name, method);
                                if let Some(sig) = reg.get_signature(&qualified) {
                                    return sig.has_self_receiver
                                        && sig.param_ownership.first()
                                            == Some(&super::OwnershipMode::Owned);
                                }
                            }
                            // Fallback: bare method name
                            if let Some(sig) = reg.lookup_method(method) {
                                return sig.has_self_receiver
                                    && sig.param_ownership.first()
                                        == Some(&super::OwnershipMode::Owned);
                            }
                        }
                        return false;
                    }
                }
                false
            }
            Expression::Binary { left, right, .. } => {
                self.expression_calls_consuming_method_on_var_typed(
                    left, var_name, elem_type, registry,
                ) || self.expression_calls_consuming_method_on_var_typed(
                    right, var_name, elem_type, registry,
                )
            }
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expression_calls_consuming_method_on_var_typed(
                    arg, var_name, elem_type, registry,
                )
            }),
            Expression::Unary { operand, .. } => self
                .expression_calls_consuming_method_on_var_typed(
                    operand, var_name, elem_type, registry,
                ),
            _ => false,
        }
    }

    pub(crate) fn statement_matches_on_self_consuming(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Match { value, arms, .. } => {
                // Check if the match scrutinee is `self` or `&self`
                if self.expression_is_self_or_ref_self(value) {
                    // Now check if the match arms actually consume values
                    return self.match_arms_consume_bound_values(arms);
                }
                // Recursively check match arm bodies (which are expressions)
                arms.iter()
                    .any(|arm| self.expression_contains_match_on_self_consuming(arm.body))
            }
            Statement::Let { value: expr, .. }
            | Statement::Assignment { value: expr, .. }
            | Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_contains_match_on_self_consuming(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_contains_match_on_self_consuming(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_matches_on_self_consuming(s))
                    || else_block.as_ref().is_some_and(|body| {
                        body.iter()
                            .any(|s| self.statement_matches_on_self_consuming(s))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_contains_match_on_self_consuming(condition)
                    || body
                        .iter()
                        .any(|s| self.statement_matches_on_self_consuming(s))
            }
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_matches_on_self_consuming(s)),
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.statement_matches_on_self_consuming(s)),
            Statement::Defer { statement, .. } => {
                self.statement_matches_on_self_consuming(statement)
            }
            _ => false,
        }
    }

    /// Check if match arms consume bound values (return them, cast them, etc.)
    /// vs. just returning literals without using the bindings.
    pub(crate) fn match_arms_consume_bound_values(&self, arms: &[crate::parser::MatchArm]) -> bool {
        for arm in arms {
            // Get all variable names bound in the pattern
            let bound_vars = self.pattern_bound_variables(&arm.pattern);

            // Check if the arm body uses any of these variables in a consuming way
            if self.expression_uses_variables_consuming(arm.body, &bound_vars) {
                return true;
            }
        }

        false
    }

    pub(crate) fn pattern_bound_variables(&self, pattern: &Pattern) -> Vec<String> {
        use crate::parser::EnumPatternBinding;

        let mut vars = Vec::new();
        match pattern {
            Pattern::Identifier(name) if !name.starts_with('_') => {
                vars.push(name.clone());
            }
            Pattern::EnumVariant(_, binding) => match binding {
                EnumPatternBinding::Single(name) if !name.starts_with('_') => {
                    vars.push(name.clone());
                }
                EnumPatternBinding::Tuple(patterns) => {
                    for p in patterns {
                        vars.append(&mut self.pattern_bound_variables(p));
                    }
                }
                EnumPatternBinding::Struct(fields, _) => {
                    for (_, p) in fields {
                        vars.append(&mut self.pattern_bound_variables(p));
                    }
                }
                _ => {}
            },
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    vars.append(&mut self.pattern_bound_variables(p));
                }
            }
            Pattern::Ref(name) | Pattern::RefMut(name) if !name.starts_with('_') => {
                vars.push(name.clone());
            }
            _ => {}
        }
        vars
    }

    pub(crate) fn expression_uses_variables_consuming(
        &self,
        expr: &Expression,
        vars: &[String],
    ) -> bool {
        use crate::parser::Expression;

        match expr {
            // If the expression is just an identifier from our bound vars, it's consumed
            Expression::Identifier { name, .. } if vars.contains(name) => true,

            // Casts consume the value
            Expression::Cast { expr: inner, .. } => {
                self.expression_uses_variables_consuming(inner, vars)
            }

            // Comparison operators (>=, <=, ==, !=, >, <) work via references
            // (PartialOrd/PartialEq traits) and do NOT consume operands.
            // Arithmetic ops (+, -, *, /) can consume for non-Copy types,
            // so we only skip for comparisons.
            Expression::Binary {
                left, op, right, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                let is_comparison = matches!(
                    op,
                    BinaryOp::Eq
                        | BinaryOp::Ne
                        | BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                );
                if is_comparison {
                    false
                } else {
                    self.expression_uses_variables_consuming(left, vars)
                        || self.expression_uses_variables_consuming(right, vars)
                }
            }

            // Method calls consume self if self is a bound var
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_uses_variables_consuming(object, vars)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_uses_variables_consuming(arg, vars))
            }

            // Function calls consume arguments
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_uses_variables_consuming(arg, vars)),

            // Field access on a bound var consumes if the field is non-Copy
            Expression::FieldAccess { object, .. } => {
                self.expression_uses_variables_consuming(object, vars)
            }

            // Literals don't consume anything
            Expression::Literal { .. } => false,

            // Blocks: check last expression
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_uses_variables_consuming(s, vars)),

            _ => false,
        }
    }

    pub(crate) fn statement_uses_variables_consuming(
        &self,
        stmt: &Statement,
        vars: &[String],
    ) -> bool {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Expression { expr, .. } => {
                self.expression_uses_variables_consuming(expr, vars)
            }
            _ => false,
        }
    }

    pub(crate) fn expression_contains_match_on_self_consuming(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_matches_on_self_consuming(s)),
            Expression::Binary { left, right, .. } => {
                self.expression_contains_match_on_self_consuming(left)
                    || self.expression_contains_match_on_self_consuming(right)
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_contains_match_on_self_consuming(arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_contains_match_on_self_consuming(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_contains_match_on_self_consuming(arg))
            }
            _ => false,
        }
    }

    pub(crate) fn expression_is_self_or_ref_self(&self, expr: &Expression) -> bool {
        use crate::parser::UnaryOp;
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => {
                matches!(&**operand, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    /// Check if ANY statement in the function body moves a non-Copy field out of self.
    /// Walks the entire body for patterns like:
    ///   - `let x = self.field` (where field is non-Copy)
    ///   - `Foo { field: self.field }` (struct literal with non-Copy self field)
    ///   - `let mut x = self.field` (assignment from non-Copy self field)
    ///
    /// Direct `return self.field` / trailing `self.field` as implicit return are excluded: codegen
    /// emits `.clone()` for `&self` receivers (same rule as read-only getters).
    pub(super) fn function_body_moves_non_copy_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_moves_non_copy_self_field(stmt) {
                return true;
            }
        }
        false
    }

    pub(crate) fn statement_moves_non_copy_self_field(&self, stmt: &Statement) -> bool {
        use crate::parser::Statement;
        match stmt {
            Statement::Let { value, .. } => self.expression_moves_non_copy_self_field(value),
            Statement::Assignment { value, .. } => self.expression_moves_non_copy_self_field(value),
            Statement::Expression { expr, .. } => {
                if self.expression_is_self_field_access(expr) {
                    return false;
                }
                self.expression_moves_non_copy_self_field(expr)
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                if self.expression_is_self_field_access(expr) {
                    return false;
                }
                self.expression_moves_non_copy_self_field(expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_moves_non_copy_self_field(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_moves_non_copy_self_field(s))
                    })
            }
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            Statement::While { body, .. } => body
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.expression_moves_non_copy_self_field(arm.body)),
            _ => false,
        }
    }

    pub(crate) fn expression_moves_non_copy_self_field(&self, expr: &Expression) -> bool {
        match expr {
            // `self.field` or `self.a.b` used as a value (not in a method call position)
            Expression::FieldAccess { object, field, .. } => {
                if self.expression_is_self(object) {
                    let field_type = self.lookup_field_type_for_self(field);
                    if let Some(ft) = field_type {
                        return !self.is_copy_type(&ft);
                    }
                    return false;
                }
                // Nested field chain (e.g., self.compositor.mesh_render_width).
                // Resolve the FULL chain type: if the final field is Copy, reading
                // it through a reference is fine (no move of the intermediate parent).
                if let Some(chain_type) = self.resolve_self_field_chain_type(expr) {
                    return !self.is_copy_type(&chain_type);
                }
                // Fallback if chain resolution fails: recurse conservatively
                self.expression_moves_non_copy_self_field(object)
            }
            // Struct literal: Foo { field: self.field, ... }
            // Moving non-Copy `self.field` values into the struct consumes `self` (builder pattern).
            // Read-only snapshots clone under `&self`; those are excluded via snapshot_factory in
            // `infer_impl_self_receiver_ownership`.
            Expression::StructLiteral { fields, .. } => {
                if fields.iter().any(
                    |(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self"),
                ) {
                    return true;
                }
                fields.iter().any(|(_, v)| {
                    if self.expression_is_self_field_access(v) {
                        if let Expression::FieldAccess { field, .. } = v {
                            if let Some(ft) = self.lookup_field_type_for_self(field) {
                                return !self.is_copy_type(&ft);
                            }
                        }
                        return true;
                    }
                    self.expression_moves_non_copy_self_field(v)
                })
            }
            // Block expression
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            _ => false,
        }
    }

    pub(crate) fn expression_is_self(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Identifier { name, .. } if name == "self")
    }

    /// Resolve the final type of a self field chain like `self.a.b.c`.
    /// Returns `Some(type_of_c)` if the entire chain can be resolved, `None` otherwise.
    pub(crate) fn resolve_self_field_chain_type(&self, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::FieldAccess { object, field, .. } => {
                if self.expression_is_self(object) {
                    self.lookup_field_type_for_self(field)
                } else {
                    let parent_type = self.resolve_self_field_chain_type(object)?;
                    self.lookup_field_type_on_struct(&parent_type, field)
                }
            }
            _ => None,
        }
    }

    /// Look up the type of a field on an arbitrary struct type.
    /// Checks the current file's AST first, then the global cross-file registry.
    pub(crate) fn lookup_field_type_on_struct(&self, ty: &Type, field: &str) -> Option<Type> {
        let type_name = match ty {
            Type::Custom(name) => name.as_str(),
            _ => return None,
        };

        // First: check the current file's AST
        if let Some(ctx) = self.self_impl_context.as_ref() {
            let program = ctx.program();
            for item in &program.items {
                if let crate::parser::Item::Struct { decl, .. } = item {
                    if decl.name == type_name {
                        for sf in &decl.fields {
                            if sf.name == field {
                                return Some(sf.field_type.clone());
                            }
                        }
                    }
                }
            }
        }

        // Second: check the global cross-file struct field registry.
        // Try exact name first, then suffix match (for module-qualified keys
        // like "rendering::HybridCompositor" when we have just "HybridCompositor").
        if let Some(fields) = self.global_struct_field_types.get(type_name) {
            if let Some(field_type) = fields.get(field) {
                return Some(field_type.clone());
            }
        }
        let suffix = format!("::{}", type_name);
        for (key, fields) in self.global_struct_field_types.iter() {
            if key.ends_with(&suffix) || key == type_name {
                if let Some(field_type) = fields.get(field) {
                    return Some(field_type.clone());
                }
            }
        }

        None
    }

    /// Look up the type of a field on `self` using the struct definition from the program.
    pub(crate) fn lookup_field_type_for_self(&self, field: &str) -> Option<Type> {
        if let Some(ctx) = self.self_impl_context.as_ref() {
            let program = ctx.program();
            let struct_name = &ctx.impl_type_base;

            for item in &program.items {
                if let crate::parser::Item::Struct { decl, .. } = item {
                    if decl.name == *struct_name {
                        for sf in &decl.fields {
                            if sf.name == field {
                                return Some(sf.field_type.clone());
                            }
                        }
                    }
                }
            }
        }

        self.self_impl_context
            .as_ref()
            .map(|ctx| ctx.impl_type_base.as_str())
            .and_then(|struct_name| {
                self.global_struct_field_types
                    .get(struct_name)
                    .and_then(|fields| fields.get(field).cloned())
            })
    }

    pub(crate) fn expression_returns_self_type(&self, expr: &Expression, type_name: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { name, fields, .. } if name == type_name => {
                // Consuming builder embeds bare `self`. Snapshot/factory patterns construct a
                // new instance from `self.field` reads (or clones) without consuming `self`.
                fields.iter().any(
                    |(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self"),
                )
            }
            _ => false,
        }
    }

    /// Check if a function uses a specific identifier (e.g., "self")
    pub(super) fn function_uses_identifier(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            if self.statement_uses_identifier(name, stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement uses a specific identifier
    pub(crate) fn statement_uses_identifier(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expression_uses_identifier(name, expr),
            Statement::Let { value, .. } => self.expression_uses_identifier(name, value),
            Statement::Assignment { target, value, .. } => {
                self.expression_uses_identifier(name, target)
                    || self.expression_uses_identifier(name, value)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_uses_identifier(name, expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_uses_identifier(name, condition)
                    || self.function_uses_identifier(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| self.function_uses_identifier(name, block))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_uses_identifier(name, condition)
                    || self.function_uses_identifier(name, body)
            }
            Statement::For { iterable, body, .. } => {
                self.expression_uses_identifier(name, iterable)
                    || self.function_uses_identifier(name, body)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_uses_identifier(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expression_uses_identifier(name, arm.body))
            }
            _ => false,
        }
    }

    /// Check if an expression uses a specific identifier
    pub(super) fn expression_uses_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expression_uses_identifier(name, object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_uses_identifier(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_uses_identifier(name, arg))
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_uses_identifier(name, arg)),
            Expression::Binary { left, right, .. } => {
                self.expression_uses_identifier(name, left)
                    || self.expression_uses_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expression_uses_identifier(name, operand),
            Expression::Index { object, index, .. } => {
                self.expression_uses_identifier(name, object)
                    || self.expression_uses_identifier(name, index)
            }
            Expression::Block { statements, .. } => self.function_uses_identifier(name, statements),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_uses_identifier(name, el)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_uses_identifier(name, el)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expression_uses_identifier(name, k) || self.expression_uses_identifier(name, v)
            }),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_uses_identifier(name, v)),
            Expression::Cast { expr, .. } => self.expression_uses_identifier(name, expr),
            Expression::Range { start, end, .. } => {
                self.expression_uses_identifier(name, start)
                    || self.expression_uses_identifier(name, end)
            }
            Expression::TryOp { expr, .. } => self.expression_uses_identifier(name, expr),
            _ => false,
        }
    }
}
