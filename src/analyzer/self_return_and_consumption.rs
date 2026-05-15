//! Returning `self`, `match self`, moves, and general identifier scans.
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

    fn expression_consumes_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self")),
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
        for stmt in &func.body {
            if self.statement_matches_on_self_consuming(stmt) {
                return true;
            }
        }
        false
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
        for stmt in &func.body {
            if self.statement_consumes_self_field_elements(stmt, registry) {
                return true;
            }
        }
        false
    }

    fn statement_consumes_self_field_elements(
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
                // Check if iterable is self.field (e.g., self.conditions)
                if !self.expression_is_self_field(iterable) {
                    return false;
                }

                // Get the loop variable name from pattern
                let loop_var = match pattern {
                    Pattern::Identifier(name) => name.clone(),
                    _ => return false,
                };

                // Check if loop body calls consuming methods on loop_var
                body.iter()
                    .any(|s| self.statement_calls_consuming_method_on_var(s, &loop_var, registry))
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

    fn expression_is_self_field(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    fn statement_calls_consuming_method_on_var(
        &self,
        stmt: &Statement,
        var_name: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Let { value: expr, .. }
            | Statement::Assignment { value: expr, .. } => {
                self.expression_calls_consuming_method_on_var(expr, var_name, registry)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_calls_consuming_method_on_var(condition, var_name, registry)
                    || then_block.iter().any(|s| {
                        self.statement_calls_consuming_method_on_var(s, var_name, registry)
                    })
                    || else_block.as_ref().is_some_and(|body| {
                        body.iter().any(|s| {
                            self.statement_calls_consuming_method_on_var(s, var_name, registry)
                        })
                    })
            }
            _ => false,
        }
    }

    fn expression_calls_consuming_method_on_var(
        &self,
        expr: &Expression,
        var_name: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        if let Some(reg) = registry {
                            if let Some(sig) = reg
                                .get_signature(method)
                                .or_else(|| reg.find_signature_ending_with(method))
                            {
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
                self.expression_calls_consuming_method_on_var(left, var_name, registry)
                    || self.expression_calls_consuming_method_on_var(right, var_name, registry)
            }
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expression_calls_consuming_method_on_var(arg, var_name, registry)
            }),
            Expression::Unary { operand, .. } => {
                self.expression_calls_consuming_method_on_var(operand, var_name, registry)
            }
            _ => false,
        }
    }

    fn statement_matches_on_self_consuming(&self, stmt: &Statement) -> bool {
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
    fn match_arms_consume_bound_values(&self, arms: &[crate::parser::MatchArm]) -> bool {
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

    fn pattern_bound_variables(&self, pattern: &Pattern) -> Vec<String> {
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

    fn expression_uses_variables_consuming(&self, expr: &Expression, vars: &[String]) -> bool {
        use crate::parser::Expression;

        match expr {
            // If the expression is just an identifier from our bound vars, it's consumed
            Expression::Identifier { name, .. } if vars.contains(name) => true,

            // Casts consume the value
            Expression::Cast { expr: inner, .. } => {
                self.expression_uses_variables_consuming(inner, vars)
            }

            // Binary operations might consume (depends on the variable being used)
            Expression::Binary { left, right, .. } => {
                self.expression_uses_variables_consuming(left, vars)
                    || self.expression_uses_variables_consuming(right, vars)
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

    fn statement_uses_variables_consuming(&self, stmt: &Statement, vars: &[String]) -> bool {
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

    fn expression_contains_match_on_self_consuming(&self, expr: &Expression) -> bool {
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

    fn expression_is_self_or_ref_self(&self, expr: &Expression) -> bool {
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

    fn statement_moves_non_copy_self_field(&self, stmt: &Statement) -> bool {
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

    fn expression_moves_non_copy_self_field(&self, expr: &Expression) -> bool {
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
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_moves_non_copy_self_field(v)),
            // Block expression
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            _ => false,
        }
    }

    fn expression_is_self(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Identifier { name, .. } if name == "self")
    }

    /// Resolve the final type of a self field chain like `self.a.b.c`.
    /// Returns `Some(type_of_c)` if the entire chain can be resolved, `None` otherwise.
    fn resolve_self_field_chain_type(&self, expr: &Expression) -> Option<Type> {
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
    fn lookup_field_type_on_struct(&self, ty: &Type, field: &str) -> Option<Type> {
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
        for (key, fields) in &self.global_struct_field_types {
            if key.ends_with(&suffix) || key == type_name {
                if let Some(field_type) = fields.get(field) {
                    return Some(field_type.clone());
                }
            }
        }

        None
    }

    /// Look up the type of a field on `self` using the struct definition from the program.
    fn lookup_field_type_for_self(&self, field: &str) -> Option<Type> {
        let ctx = self.self_impl_context.as_ref()?;
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
        None
    }

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
}
