//! Self-analysis methods for the analyzer.
//! Determines whether impl methods need &self, &mut self, or owned self
//! by analyzing field access patterns, mutations, and return types.

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// Check if a function modifies self fields (for impl methods)
    #[allow(dead_code)]
    pub(super) fn function_modifies_self_fields(&self, func: &FunctionDecl) -> bool {
        self.function_modifies_self_fields_with_registry(func, None)
    }

    /// Check if a function modifies self fields, with optional registry for cross-type resolution
    pub(super) fn function_modifies_self_fields_with_registry(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        // THE WINDJAMMER WAY: Check ALL cases that require &mut self

        // Case 1: Return type is &mut T (requires &mut self)
        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        // Case 2: Function calls other methods on self that need &mut self
        if self.function_calls_mutating_self_methods_with_registry(func, registry) {
            return true;
        }

        // Case 3: Function modifies self fields directly
        for stmt in &func.body {
            if self.statement_modifies_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a function modifies self fields WITHOUT checking for method calls
    /// (prevents infinite recursion when analyzing cross-method dependencies)
    fn function_modifies_self_fields_recursive(&self, func: &FunctionDecl) -> bool {
        // Case 1: Return type is &mut T (requires &mut self)
        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        // Case 2: Function modifies self fields directly
        for stmt in &func.body {
            if self.statement_modifies_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a type contains a mutable reference (&mut T)
    /// This includes Option<&mut T>, Result<&mut T, E>, Vec<&mut T>, etc.
    fn type_is_mut_ref(&self, ty: &Type) -> bool {
        match ty {
            Type::MutableReference(_) => true,
            Type::Option(inner) | Type::Vec(inner) | Type::Reference(inner) => {
                self.type_is_mut_ref(inner)
            }
            Type::Result(ok, err) => self.type_is_mut_ref(ok) || self.type_is_mut_ref(err),
            Type::Tuple(types) => types.iter().any(|t| self.type_is_mut_ref(t)),
            Type::Parameterized(_, args) => args.iter().any(|t| self.type_is_mut_ref(t)),
            _ => false,
        }
    }

    /// Check if function calls methods on self that require &mut self
    #[allow(dead_code)]
    fn function_calls_mutating_self_methods(&self, func: &FunctionDecl) -> bool {
        self.function_calls_mutating_self_methods_with_registry(func, None)
    }

    /// Check if function calls methods on self that require &mut self (with registry)
    fn function_calls_mutating_self_methods_with_registry(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        for stmt in &func.body {
            if self.statement_calls_mutating_self_methods(stmt, registry) {
                return true;
            }
        }
        false
    }

    /// Check if statement calls methods on self that require &mut self
    fn statement_calls_mutating_self_methods(
        &self,
        stmt: &Statement,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_calls_mutating_self_methods(expr, registry)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_calls_mutating_self_methods(s, registry))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_calls_mutating_self_methods(s, registry))
                    })
            }
            Statement::While { body, .. } => body
                .iter()
                .any(|s| self.statement_calls_mutating_self_methods(s, registry)),
            Statement::For { iterable, body, .. } => {
                self.expression_calls_mutating_self_methods(iterable, registry)
                    || body
                        .iter()
                        .any(|s| self.statement_calls_mutating_self_methods(s, registry))
            }
            Statement::Let { value, .. } => {
                self.expression_calls_mutating_self_methods(value, registry)
            }
            _ => false,
        }
    }

    /// Check if expression calls methods on self that require &mut self
    fn expression_calls_mutating_self_methods(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Check if calling a method on self (not self.field, just self)
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        // THE WINDJAMMER WAY: Check if this method requires &mut self
                        // 1. Check hardcoded stdlib mutating methods
                        if self.is_mutating_method(method) {
                            return true;
                        }

                        // 2. Check methods in current impl block
                        if let Some(impl_functions) = &self.current_impl_functions {
                            if let Some(called_func) = impl_functions.get(method) {
                                if self.function_modifies_self_fields_recursive(called_func) {
                                    return true;
                                }
                            }
                        }
                    }
                }

                // Cross-type mutation propagation via self.field.method()
                if self.expression_is_self_field_mutating_method_call(object, method, registry) {
                    return true;
                }

                // Recurse into arguments to find nested mutation patterns
                for (_, arg) in arguments {
                    if self.expression_calls_mutating_self_methods(arg, registry) {
                        return true;
                    }
                }

                false
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_calls_mutating_self_methods(s, registry)),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_calls_mutating_self_methods(arg, registry)),
            _ => false,
        }
    }

    /// Check if object.method() is a self.field[.subfield...].method() pattern
    /// where method requires &mut self. Checks both stdlib list and signature registry.
    fn expression_is_self_field_mutating_method_call(
        &self,
        object: &Expression,
        method: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        if !self.expression_traces_to_self(object) {
            return false;
        }

        if self.is_mutating_method(method) {
            return true;
        }

        // Check methods in current impl block (same-file, different struct)
        if let Some(impl_functions) = &self.current_impl_functions {
            if let Some(called_func) = impl_functions.get(method) {
                if self.function_modifies_self_fields_recursive(called_func) {
                    return true;
                }
            }
        }

        // Cross-type registry lookup: if the method exists in the registry
        // and takes &mut self, it's a mutating call
        if let Some(reg) = registry {
            if let Some(sig) = reg.get_signature(method) {
                if sig.has_self_receiver {
                    if let Some(&ownership) = sig.param_ownership.first() {
                        if matches!(ownership, super::OwnershipMode::MutBorrowed) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if an expression traces back to `self` through a chain of field accesses.
    /// Returns true for: self.field, self.field.subfield, etc.
    fn expression_traces_to_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    name == "self"
                } else {
                    self.expression_traces_to_self(object)
                }
            }
            _ => false,
        }
    }

    /// Check if a statement modifies self fields
    fn statement_modifies_self_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                self.expression_is_self_field_access(target)
                    || self.expression_is_self_field_index_access(target)
            }
            Statement::Expression { expr, .. } => self.expression_mutates_self_fields(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // THE WINDJAMMER WAY: Check condition for mutations!
                self.expression_mutates_self_fields(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_modifies_self_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_modifies_self_fields(s))
                    })
            }
            Statement::While { body, .. } => {
                body.iter().any(|s| self.statement_modifies_self_fields(s))
            }
            Statement::For { iterable, body, .. } => {
                // THE WINDJAMMER WAY: Check BOTH the iterable AND the body!
                self.expression_mutates_self_fields(iterable)
                    || body.iter().any(|s| self.statement_modifies_self_fields(s))
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Check match value for mutations!
                self.expression_mutates_self_fields(value)
                    || arms
                        .iter()
                        .any(|arm| self.expression_contains_self_field_mutations(arm.body))
            }
            Statement::Return { value, .. } => value
                .as_ref()
                .is_some_and(|expr| self.expression_mutates_self_fields(expr)),
            Statement::Let { value, .. } => self.expression_mutates_self_fields(value),
            _ => false,
        }
    }

    /// Check if an expression contains self field mutations (for match arms and blocks)
    fn expression_contains_self_field_mutations(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_modifies_self_fields(s)),
            Expression::MethodCall { object, method, .. } => {
                // TDD FIX: Check for mutations on both direct AND indexed self fields
                (self.expression_is_self_field_access(object)
                    || self.expression_is_self_field_index_access(object))
                    && self.is_mutating_method(method)
            }
            _ => false,
        }
    }

    /// Check if expression is a self field access (self.field)
    pub(super) fn expression_is_self_field_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                match &**object {
                    Expression::Identifier { name, .. } if name == "self" => true,
                    Expression::FieldAccess { .. } => self.expression_is_self_field_access(object),
                    // CRITICAL FIX: Handle index expressions like self.children[i].field
                    Expression::Index { .. } => self.expression_is_self_field_index_access(object),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Check if expression is an index access on a self field (self.field[index] or self.field[i][j])
    fn expression_is_self_field_index_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Index { object, .. } => {
                self.expression_is_self_field_access(object)
                    || self.expression_is_self_field_index_access(object)
            }
            _ => false,
        }
    }

    /// Check if expression mutates self fields
    fn expression_mutates_self_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                // TDD FIX: Check for mutations on INDEXED self fields!
                if self.expression_is_self_field_access(object) && self.is_mutating_method(method) {
                    return true;
                }

                if self.expression_is_self_field_index_access(object)
                    && self.is_mutating_method(method)
                {
                    return true;
                }

                false
            }
            // TDD FIX: Detect `&mut self.field` as a mutation of self
            Expression::Unary {
                op: crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => self.expression_is_self_field_access(operand),
            _ => false,
        }
    }

    /// Check if a function returns Self (for builder pattern detection)
    pub(super) fn function_returns_self(&self, func: &FunctionDecl) -> bool {
        use crate::parser::{Statement, Type};

        let return_type_name = match &func.return_type {
            Some(Type::Custom(name)) => name,
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

    /// Check if a function returns a non-Copy field from self (e.g., `self.content` where content is String).
    /// Moving a field out of `self` requires owned self, not `&self`.
    pub(super) fn function_returns_non_copy_self_field(&self, func: &FunctionDecl) -> bool {
        use crate::parser::Statement;

        let return_type = match &func.return_type {
            Some(t) => t,
            None => return false,
        };

        if self.is_copy_type(return_type) {
            return false;
        }

        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => self.expression_is_self_field_access(expr),
                Statement::Expression { expr, .. } => self.expression_is_self_field_access(expr),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check if an expression returns the Self type (either `self` or a struct literal of the same type)
    fn expression_returns_self_type(&self, expr: &Expression, type_name: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { name, .. } if name == type_name => true,
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
    fn statement_uses_identifier(&self, name: &str, stmt: &Statement) -> bool {
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

    /// Check if a function accesses self fields (for impl methods)
    #[allow(dead_code)]
    pub(super) fn function_accesses_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_accesses_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement accesses self fields
    fn statement_accesses_self_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_accesses_self_fields(expr),
            Statement::Let { value, .. } => self.expression_accesses_self_fields(value),
            Statement::Assignment { target, value, .. } => {
                self.expression_accesses_self_fields(target)
                    || self.expression_accesses_self_fields(value)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_accesses_self_fields(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_accesses_self_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_accesses_self_fields(s))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_accesses_self_fields(condition)
                    || body.iter().any(|s| self.statement_accesses_self_fields(s))
            }
            Statement::For { iterable, body, .. } => {
                self.expression_accesses_self_fields(iterable)
                    || body.iter().any(|s| self.statement_accesses_self_fields(s))
            }
            _ => false,
        }
    }

    /// Check if expression accesses self fields
    #[allow(clippy::only_used_in_recursion)]
    fn expression_accesses_self_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_accesses_self_fields(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_accesses_self_fields(arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expression_accesses_self_fields(left)
                    || self.expression_accesses_self_fields(right)
            }
            Expression::Unary { operand, .. } => self.expression_accesses_self_fields(operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_accesses_self_fields(arg)),
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|arg| self.expression_accesses_self_fields(arg)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_accesses_self_fields(elem)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_accesses_self_fields(elem)),
            _ => false,
        }
    }

    /// Check if an expression references a variable
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn expression_references_variable(&self, var_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var_name,
            Expression::Binary { left, right, .. } => {
                self.expression_references_variable(var_name, left)
                    || self.expression_references_variable(var_name, right)
            }
            Expression::MethodCall { object, .. } | Expression::FieldAccess { object, .. } => {
                self.expression_references_variable(var_name, object)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expression_references_variable(var_name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_references_variable(var_name, arg))
            }
            _ => false,
        }
    }
}
