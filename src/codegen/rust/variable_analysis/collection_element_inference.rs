use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::*;

use super::CodeGenerator;

fn sig_stores_element(sig: &FunctionSignature) -> bool {
    if !sig.has_self_receiver {
        return false;
    }
    if !matches!(
        sig.param_ownership.first(),
        Some(OwnershipMode::MutBorrowed)
    ) {
        return false;
    }
    let self_count = usize::from(sig.has_self_receiver);
    if sig.param_types.len() <= self_count {
        return false;
    }
    let last_idx = sig.param_types.len() - 1;
    match sig.formal_param_type(last_idx) {
        Some(Type::Custom(name)) if name == "usize" || name == "K" => false,
        Some(Type::Reference(inner) | Type::MutableReference(inner)) => !matches!(
            **inner,
            Type::Custom(ref n) if n == "K"
        ),
        _ => true,
    }
}

/// Element of `Vec<T>` / `&Vec<T>`. Prefer the AST formal when emission wrapped
/// a bare `Vec` and dropped `T` (demoted `&Vec<u64>` still has formal `Vec<u64>`).
fn concrete_vec_element_from_sig(sig: &FunctionSignature, arg_index: usize) -> Option<Type> {
    let shifted = sig.arg_param_index(arg_index);
    // A self-slot flag without a Self param (free fn `bakeoff_run_median`) would
    // skip the only `Vec<u64>` formal. Use the raw argument index then.
    let mut indexes = vec![shifted];
    if shifted != arg_index
        && sig.param_types.get(shifted).is_none()
        && sig.formal_param_type(shifted).is_none()
    {
        indexes.insert(0, arg_index);
    }
    let mut candidates = Vec::new();
    for pidx in indexes {
        if let Some(ty) = sig.formal_param_type(pidx) {
            candidates.push(ty);
        }
        if let Some(ty) = sig.param_types.get(pidx) {
            candidates.push(ty);
        }
    }
    let mut fallback = None;
    for ty in candidates {
        let Some(elem) = CodeGenerator::peeled_collection_element_type(ty) else {
            continue;
        };
        if matches!(elem, Type::Float) {
            continue;
        }
        let concrete = matches!(
            elem,
            Type::Custom(n)
                if matches!(
                    n.as_str(),
                    "u64" | "u32" | "u16" | "u8" | "i32" | "i16" | "i8" | "usize" | "f32"
                )
        ) || matches!(elem, Type::Int32 | Type::Uint);
        if concrete {
            return Some(elem.clone());
        }
        if fallback.is_none() {
            fallback = Some(elem.clone());
        }
    }
    fallback
}

fn method_stores_element_in_registry(method: &str, registry: &SignatureRegistry) -> bool {
    registry
        .signatures_for_method_name(method)
        .any(|(_key, sig)| sig_stores_element(sig))
}

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
        // Callee `Vec<u64>` / `&Vec<u64>` beats default `vec![10]` → i32 (WDB-127).
        let body: Vec<&Statement<'_>> = if !self.full_function_body_snapshot.is_empty() {
            self.full_function_body_snapshot.iter().copied().collect()
        } else {
            self.current_function_body.iter().copied().collect()
        };
        if let Some(from_call) = self.infer_vec_element_from_function_call(var_name, &body) {
            return Some(from_call);
        }
        let push_type =
            self.scan_statements_for_collection_usage(var_name, &self.current_function_body);
        // When push inference yields a generic Type::Float, also check function call context.
        // E.g. compare_rgba_buffers(pixels, ...) where param type is Vec<f32> → use f32, not f64.
        if matches!(push_type, Some(Type::Float)) {
            // No concrete type from function context. Bare float literals default to f32
            // (see literals.rs), so Vec element type must also be f32 for consistency.
            return Some(Type::Custom("f32".into()));
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

    /// Defining-module signature and the local stub. An importer stub that dropped
    /// `Vec<u64>` params must not hide the callee element (WDB-127).
    fn vec_elem_signature_candidates(&self, fn_name: &str) -> Vec<&FunctionSignature> {
        let lookup = self.signature_lookup_callee_name(fn_name);
        let mut out: Vec<&FunctionSignature> = Vec::new();
        if let Some(g) = self.global_signature_registry.as_ref() {
            if let Some(sig) = g
                .get_signature(lookup.as_ref())
                .or_else(|| g.get_signature(fn_name))
            {
                out.push(sig);
            }
        }
        if let Some(sig) = self
            .signature_registry
            .get_signature(lookup.as_ref())
            .or_else(|| self.signature_registry.get_signature(fn_name))
        {
            if !out.iter().any(|s| std::ptr::eq(*s, sig)) {
                out.push(sig);
            }
        }
        if let Some(sig) = self.get_signature_with_global(fn_name) {
            if !out.iter().any(|s| std::ptr::eq(*s, sig)) {
                out.push(sig);
            }
        }
        out
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
                    for sig in self.vec_elem_signature_candidates(fn_name) {
                        for (i, (_label, arg)) in arguments.iter().enumerate() {
                            let arg_name = match arg {
                                Expression::Identifier { name, .. } => Some(name.as_str()),
                                Expression::Unary { operand, .. } => match &**operand {
                                    Expression::Identifier { name, .. } => Some(name.as_str()),
                                    _ => None,
                                },
                                _ => None,
                            };
                            if arg_name == Some(var_name) {
                                if let Some(elem) = concrete_vec_element_from_sig(sig, i) {
                                    return Some(elem);
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
                if let Some(sig) = self.get_signature_with_global(method) {
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == var_name) {
                            if let Some(elem) = concrete_vec_element_from_sig(sig, i) {
                                return Some(elem);
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

            let registry = self
                .global_signature_registry
                .as_deref()
                .unwrap_or(&self.signature_registry);
            if !method_stores_element_in_registry(method, registry) {
                return None;
            }
            if arguments.is_empty() {
                return None;
            }

            let arg_expr = &arguments[arguments.len() - 1].1;
            return self.infer_expression_type(arg_expr);
        }
        None
    }
}
