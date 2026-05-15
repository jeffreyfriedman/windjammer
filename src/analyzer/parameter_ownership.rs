//! Core per-parameter ownership inference (`infer_parameter_ownership`) and “only borrowed” analysis.

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    pub(super) fn infer_parameter_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
        return_type: &Option<Type>,
        registry: &SignatureRegistry,
        current_func_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> Result<OwnershipMode, String> {
        // 0a. Generic type parameters and impl Trait always stay Owned.
        // Adding & would change trait bounds: `impl Foo` -> `&impl Foo` breaks dispatch.
        // Generic types like T, G, S should always be passed by value.
        if Self::is_generic_type_param(param_type) {
            return Ok(OwnershipMode::Owned);
        }

        // 0b. Explicit Rust type `String` (not Windjammer `string`) stays Owned.
        // When user writes `path: String`, they're explicitly requesting Rust's String type,
        // which should be respected as owned. Do NOT infer it as borrowed.
        // This is different from `path: string` (lowercase), which can infer to &str.
        if matches!(param_type, Type::Custom(name) if name == "String") {
            return Ok(OwnershipMode::Owned);
        }

        // 0c. Return-type-aware ownership: When return type contains param type, we need Owned.
        // Bug: save_migration.wj - migrate(data) -> Result<GameSaveData, string> was inferring
        // &GameSaveData because we only read data fields. But we assign to current_data and
        // return that - we need to own the input to produce the output.
        // Handles: fn(T) -> T, fn(T) -> Result<T,E>, fn(T) -> Option<T>
        //
        // TDD FIX: Skip when param is ONLY used as &param (e.g., a + &b + &c).
        // For concatenate(a, b, c) -> a + &b + &c, b and c are borrowed, not consumed.
        // param_type_matches_return would incorrectly infer Owned for all string params.
        if !self.is_only_used_as_borrow(param_name, body) {
            if let Some(return_type) = return_type {
                if self.param_type_matches_return(param_type, return_type) {
                    // Windjammer `string` / `str` parameters: return type also being string-like
                    // does NOT mean the parameter is consumed into the return value.
                    // Example: find_translation(lang, key: string) -> string only compares `key`;
                    // inferring Owned for `key` breaks callers that pass the same String twice (E0382).
                    // Non-string types keep the broader rule (transform/migrate still get Owned).
                    let string_like = matches!(param_type, Type::String);
                    if self.is_returned(param_name, body)
                        || self.is_stored(param_name, body)
                    {
                        return Ok(OwnershipMode::Owned);
                    } else if !string_like
                        && self.param_is_consumed_into_return(param_name, body)
                    {
                        return Ok(OwnershipMode::Owned);
                    }
                }
            }
        }

        // Multi-pass registry-aware inference

        // 1. Check if parameter is mutated (uses registry for method call detection)
        if self.is_mutated(param_name, body, registry) {
            return Ok(OwnershipMode::MutBorrowed);
        }

        // 2. Check if parameter is returned (escapes function)
        if self.is_returned(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 2.3. WINDJAMMER FIX: Check if parameter is used in if/else expression
        if self.is_used_in_if_else_expression(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 3. Check if parameter is stored in a struct or collection
        if self.is_stored(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 4. Check if parameter is used in arithmetic binary operations (for Copy types)
        if self.is_used_in_arithmetic_op(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 5. Check if parameter is pattern matched with field extraction
        if self.is_pattern_matched_with_fields(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 6. TDD: Check if parameter is iterated over in a for loop
        if self.is_iterated_over(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 6b. TDD: Check if parameter calls a method that takes `self` by value (consuming).
        if self.calls_consuming_method(param_name, body, registry) {
            return Ok(OwnershipMode::Owned);
        }

        // 7. MULTI-PASS OWNERSHIP INFERENCE
        if let Some(pass_through_mode) = self.infer_passthrough_ownership(
            param_name,
            param_type,
            body,
            registry,
            current_func_name,
            func,
        ) {
            match pass_through_mode {
                OwnershipMode::Borrowed => return Ok(OwnershipMode::Borrowed),
                OwnershipMode::MutBorrowed => return Ok(OwnershipMode::MutBorrowed),
                OwnershipMode::Owned => {
                    return Ok(OwnershipMode::Owned);
                }
            }
        }

        Ok(OwnershipMode::Borrowed)
    }

    /// TDD: parameter ONLY used as &param or &mut param (never consumed directly).
    fn is_only_used_as_borrow(&self, param_name: &str, body: &[&'ast Statement<'ast>]) -> bool {
        for stmt in body {
            if !self.stmt_param_only_borrowed(param_name, stmt, false) {
                return false;
            }
        }
        true
    }

    fn stmt_param_only_borrowed(
        &self,
        param_name: &str,
        stmt: &Statement,
        _inside_ref: bool,
    ) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_param_only_borrowed(param_name, value, false),
            Statement::Return { value, .. } => value
                .as_ref()
                .is_none_or(|e| self.expr_param_only_borrowed(param_name, e, false)),
            Statement::Expression { expr, .. } => {
                self.expr_param_only_borrowed(param_name, expr, false)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_param_only_borrowed(param_name, condition, false)
                    && then_block
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
                    && else_block.as_ref().is_none_or(|b| {
                        b.iter()
                            .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_param_only_borrowed(param_name, condition, false)
                    && body
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
            }
            Statement::For { iterable, body, .. } => {
                self.expr_param_only_borrowed(param_name, iterable, false)
                    && body
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
            }
            _ => true,
        }
    }

    fn expr_param_only_borrowed(
        &self,
        param_name: &str,
        expr: &Expression,
        inside_ref: bool,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => inside_ref,
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => self.expr_param_only_borrowed(param_name, operand, true),
            Expression::Binary { left, right, .. } => {
                self.expr_param_only_borrowed(param_name, left, false)
                    && self.expr_param_only_borrowed(param_name, right, false)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_param_only_borrowed(param_name, function, false)
                    && arguments
                        .iter()
                        .all(|(_, a)| self.expr_param_only_borrowed(param_name, a, false))
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_param_only_borrowed(param_name, object, false)
                    && arguments
                        .iter()
                        .all(|(_, a)| self.expr_param_only_borrowed(param_name, a, false))
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_param_only_borrowed(param_name, object, false)
            }
            Expression::Index { object, index, .. } => {
                self.expr_param_only_borrowed(param_name, object, false)
                    && self.expr_param_only_borrowed(param_name, index, false)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .all(|s| self.stmt_param_only_borrowed(param_name, s, false)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .all(|e| self.expr_param_only_borrowed(param_name, e, false)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .all(|(_, v)| self.expr_param_only_borrowed(param_name, v, false)),
            Expression::TryOp { expr, .. } => {
                self.expr_param_only_borrowed(param_name, expr, false)
            }
            _ => true,
        }
    }
}
