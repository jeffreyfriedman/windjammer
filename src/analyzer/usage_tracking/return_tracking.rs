//! Return paths and “consumed into return” tracking for identifiers in function bodies.

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
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
    pub(crate) fn expression_uses_identifier_for_return(&self, name: &str, expr: &Expression) -> bool {
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
                    let is_known_wrapper = matches!(fn_name.as_str(), "Some" | "Ok" | "Err");
                    let is_enum_constructor = Self::looks_like_enum_variant_constructor(fn_name);

                    if is_known_wrapper || is_enum_constructor {
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
}
