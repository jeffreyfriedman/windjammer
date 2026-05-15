use crate::parser::*;

use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Forward-scan the current function body for `.push()` / `.insert()` calls on a variable
    /// to infer the collection element type for `Vec::new()` / `HashSet::new()` declarations.
    /// Returns the inferred element `Type` if found.
    pub(crate) fn infer_collection_element_type_from_usage(&self, var_name: &str) -> Option<Type> {
        if let Some(ty) = self
            .scan_statements_for_struct_literal_vec_binding(var_name, &self.current_function_body)
        {
            return Some(ty);
        }
        let push_type =
            self.scan_statements_for_collection_usage(var_name, &self.current_function_body);
        // When push inference yields a generic Type::Float, also check function call context.
        // E.g. compare_rgba_buffers(pixels, ...) where param type is Vec<f32> → use f32, not f64.
        if matches!(push_type, Some(Type::Float)) {
            if let Some(concrete) =
                self.infer_vec_element_from_function_call(var_name, &self.current_function_body)
            {
                return Some(concrete);
            }
        }
        push_type
    }

    /// Scan function calls where `var_name` is passed as an argument.
    /// If the callee parameter type is `Vec<T>` with a concrete `T`, return `T`.
    fn infer_vec_element_from_function_call(
        &self,
        var_name: &str,
        stmts: &[&Statement<'_>],
    ) -> Option<Type> {
        for stmt in stmts {
            if let Some(ty) = self.check_stmt_for_vec_param_type(var_name, stmt) {
                return Some(ty);
            }
        }
        None
    }

    fn check_stmt_for_vec_param_type(&self, var_name: &str, stmt: &Statement<'_>) -> Option<Type> {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Let { value: expr, .. } => {
                self.check_expr_for_vec_param_type(var_name, expr)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.check_expr_for_vec_param_type(var_name, expr),
            Statement::If {
                then_block,
                else_block,
                ..
            } => self
                .infer_vec_element_from_function_call(var_name, then_block)
                .or_else(|| {
                    else_block
                        .as_ref()
                        .and_then(|b| self.infer_vec_element_from_function_call(var_name, b))
                }),
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::For { body, .. } => {
                self.infer_vec_element_from_function_call(var_name, body)
            }
            _ => None,
        }
    }

    fn check_expr_for_vec_param_type(&self, var_name: &str, expr: &Expression<'_>) -> Option<Type> {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let mut names_to_try: Vec<String> = Vec::new();
                match &**function {
                    Expression::Identifier { name, .. } => {
                        names_to_try.push(name.to_string());
                    }
                    Expression::FieldAccess { object, field, .. } => {
                        names_to_try.push(field.to_string());
                        if let Expression::Identifier { name, .. } = &**object {
                            names_to_try.push(format!("{}::{}", name, field));
                        }
                    }
                    _ => {}
                };
                for fn_name in &names_to_try {
                    if let Some(sig) = self.signature_registry.get_signature(fn_name) {
                        let param_offset = if sig.has_self_receiver { 1 } else { 0 };
                        for (i, (_label, arg)) in arguments.iter().enumerate() {
                            if matches!(arg, Expression::Identifier { name, .. } if name == var_name)
                            {
                                let param_idx = i + param_offset;
                                if let Some(param_type) = sig.param_types.get(param_idx) {
                                    if let Type::Vec(inner) = param_type {
                                        if !matches!(**inner, Type::Float) {
                                            return Some((**inner).clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for (_label, arg) in arguments {
                    if let Some(ty) = self.check_expr_for_vec_param_type(var_name, arg) {
                        return Some(ty);
                    }
                }
                None
            }
            Expression::MethodCall {
                method,
                arguments,
                object,
                ..
            } => {
                if let Some(sig) = self.signature_registry.get_signature(method) {
                    let param_offset = if sig.has_self_receiver { 1 } else { 0 };
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == var_name) {
                            let param_idx = i + param_offset;
                            if let Some(param_type) = sig.param_types.get(param_idx) {
                                if let Type::Vec(inner) = param_type {
                                    if !matches!(**inner, Type::Float) {
                                        return Some((**inner).clone());
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(ty) = self.check_expr_for_vec_param_type(var_name, object) {
                    return Some(ty);
                }
                for (_label, arg) in arguments {
                    if let Some(ty) = self.check_expr_for_vec_param_type(var_name, arg) {
                        return Some(ty);
                    }
                }
                None
            }
            _ => None,
        }
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
