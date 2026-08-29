//! Return paths and “consumed into return” tracking for identifiers in function bodies.

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// True when every return path that mentions `name` returns only a field of `name`
    /// (e.g. `key.bytes`), not the whole binding — callers may reuse the binding afterward.
    pub(crate) fn is_field_extract_returned(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        if !self.is_returned(name, statements) {
            return false;
        }
        !self.is_whole_binding_returned(name, statements)
    }

    fn is_whole_binding_returned(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        let len = statements.len();
        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == len - 1;
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expression_returns_whole_binding(name, expr) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } if is_last => {
                    let is_void_call = if let Expression::Call { function, .. } = expr {
                        if let Expression::Identifier { name: fn_name, .. } = &**function {
                            matches!(
                                fn_name.as_str(),
                                "println" | "print" | "eprintln" | "eprint" | "assert" | "panic"
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if !is_void_call && self.expression_returns_whole_binding(name, expr) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_whole_binding_returned(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_whole_binding_returned(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if self.expression_returns_whole_binding(name, arm.body) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expression_returns_whole_binding(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } if id == name => true,
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    if crate::type_classification::is_language_level_payload_call_name(fn_name) {
                        for (_label, arg) in arguments {
                            if self.expression_returns_whole_binding(name, arg) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_returns_whole_binding(name, elem)),
            Expression::FieldAccess { .. } => false,
            _ => false,
        }
    }

    pub(crate) fn is_returned(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        let len = statements.len();
        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == len - 1;
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Check if parameter is returned directly or wrapped in Some/Ok/Err/tuple
                    if self.expression_uses_identifier_for_return(name, expr) {
                        return true;
                    }
                }
                // CRITICAL: Handle implicit returns (last expression without semicolon)
                // In Windjammer/Rust, the last expression in a block is the return value
                Statement::Expression { expr, .. } if is_last => {
                    // Skip ONLY void-returning function calls (like println)
                    // Wrapper calls (Some, Ok, Err) DO return their arguments!
                    let is_void_call = if let Expression::Call { function, .. } = expr {
                        if let Expression::Identifier { name: fn_name, .. } = &**function {
                            matches!(
                                fn_name.as_str(),
                                "println" | "print" | "eprintln" | "eprint" | "assert" | "panic"
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !is_void_call && self.expression_uses_identifier_for_return(name, expr) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_returned(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_returned(name, else_b) {
                            return true;
                        }
                    }
                }
                // CRITICAL: Handle match expressions where parameter is returned in arms
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if self.expression_uses_identifier_for_return(name, arm.body) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if an expression uses a parameter in a way that requires ownership for return.
    /// This includes direct use, wrapping in Some/Ok/Err, tuples, etc.
    pub(crate) fn expression_uses_identifier_for_return(
        &self,
        name: &str,
        expr: &Expression,
    ) -> bool {
        match expr {
            // Direct identifier use
            Expression::Identifier { name: id, .. } if id == name => true,

            // Wrapped in constructors: Some(param), Ok(param), Err(param), Enum::Variant(param)
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    if crate::type_classification::is_language_level_payload_call_name(fn_name) {
                        for (_label, arg) in arguments {
                            if self.expression_uses_identifier(name, arg) {
                                return true;
                            }
                        }
                    }
                }
                false
            }

            // Tuple expression: (a, b, c)
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    if self.expression_uses_identifier(name, elem) {
                        return true;
                    }
                }
                false
            }

            // WJ `"…${param}…"` / `format!(…)` — only a *direct* identifier arg is
            // moved into the owned String. Nested uses (`data.len()`, `obj.field`) are reads.
            Expression::MacroInvocation {
                name: macro_name,
                args,
                ..
            } if crate::type_classification::is_language_level_owned_string_macro(macro_name) => {
                args.iter()
                    .any(|arg| matches!(arg, Expression::Identifier { name: id, .. } if id == name))
            }

            // Returning `param.field` moves the field out of an owned parameter (consumes `param`)
            // when the field type is non-Copy. Copy fields are read through `&param` with implicit copy.
            Expression::FieldAccess { object, .. } => {
                if !self.expression_uses_identifier(name, object) {
                    return false;
                }
                if matches!(&**object, Expression::Identifier { name: id, .. } if id == name) {
                    if name == "self" {
                        if let Some(chain_type) = self.resolve_self_field_chain_type(expr) {
                            if self.is_copy_type(&chain_type) {
                                return false;
                            }
                        }
                    }
                }
                self.expression_uses_identifier_for_return(name, object)
            }

            // CRITICAL FIX: Binary expressions (comparisons, arithmetic) return the RESULT, not the parameter
            // Example: `id == "test"` returns bool, NOT id
            // Example: `id + 1` returns the sum, NOT id
            // The parameter is only being READ, not returned
            Expression::Binary { .. } => false,

            // Unary expressions also return the result, not the operand
            Expression::Unary { .. } => false,

            // Default: reject (conservative - only allow explicit cases above)
            _ => false,
        }
    }

    pub(crate) fn param_is_consumed_into_return(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in body {
            match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(var_name),
                    value,
                    ..
                } => {
                    if self.expression_uses_identifier(param_name, value) {
                        if self.is_returned(var_name, body) {
                            if matches!(
                                value,
                                Expression::Identifier { name, .. } if name == param_name
                            ) {
                                return true;
                            }
                            // `"${param.trim()}"` → return derived text; param stays borrowable.
                            if self.expression_is_readonly_text_derivation(param_name, value) {
                                continue;
                            }
                            return true;
                        }
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expression_uses_identifier(param_name, value) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.param_is_consumed_into_return(param_name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.param_is_consumed_into_return(param_name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            if self.param_is_consumed_into_return(param_name, statements) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// `let s = "${param.trim()}"; return s` — param is read, not moved into the return.
    fn expression_is_readonly_text_derivation(
        &self,
        param_name: &str,
        expr: &Expression,
    ) -> bool {
        if !self.expression_uses_identifier(param_name, expr) {
            return false;
        }
        if matches!(
            expr,
            Expression::Identifier { name, .. } if name == param_name
        ) {
            return false;
        }
        !self.expression_has_consuming_use_of_param(param_name, expr)
    }

    fn expression_has_consuming_use_of_param(
        &self,
        param_name: &str,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => true,
            Expression::MethodCall { object, .. } => {
                if matches!(
                    &**object,
                    Expression::Identifier { name, .. } if name == param_name
                ) {
                    false
                } else {
                    self.expression_has_consuming_use_of_param(param_name, object)
                }
            }
            Expression::Call { arguments, function, .. } => {
                self.expression_has_consuming_use_of_param(param_name, function)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_consuming_use_of_param(param_name, arg)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_consuming_use_of_param(param_name, left)
                    || self.expression_has_consuming_use_of_param(param_name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expression_has_consuming_use_of_param(param_name, operand)
            }
            Expression::Block { statements, .. } => self
                .function_uses_identifier(param_name, statements)
                && statements.iter().any(|s| match s {
                    Statement::Expression { expr, .. } => {
                        self.expression_has_consuming_use_of_param(param_name, expr)
                    }
                    _ => false,
                }),
            Expression::Array { elements, .. } | Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_has_consuming_use_of_param(param_name, el)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_has_consuming_use_of_param(param_name, v)),
            _ => false,
        }
    }
}
