//! Passthrough ownership inference for the analyzer.
//! Multi-pass inference that matches parameter ownership to callee signatures
//! when a parameter is simply passed through to another function.

use crate::parser::*;

use std::collections::HashMap;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    pub(crate) fn strip_type_generics(name: &str) -> String {
        name.split('<').next().unwrap_or(name).to_string()
    }

    pub(crate) fn is_windjammer_text_param_type(t: &Type) -> bool {
        matches!(t, Type::String)
            || matches!(
                t,
                Type::Custom(name) if matches!(name.as_str(), "string" | "String" | "str")
            )
            || matches!(
                t,
                Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
            )
    }

    pub(crate) fn is_text_element_vec_param_type(t: &Type) -> bool {
        matches!(t, Type::Vec(inner) if Self::is_windjammer_text_param_type(inner))
    }

    /// Resolve struct field map using module-qualified keys (`dialogue::tree::DialogueNodeTree`).
    pub(crate) fn lookup_struct_fields_for_type(
        &self,
        type_name: &str,
    ) -> Option<&HashMap<String, Type>> {
        crate::type_inference::struct_field_registry::lookup_struct_field_map(
            &self.global_struct_field_types,
            type_name,
            &HashMap::new(),
            &self.struct_defining_module_paths,
        )
    }

    /// Structural type name used as `SignatureRegistry` keys (`Inventory`, `Merchant`, …).
    pub(crate) fn type_to_struct_base(ty: &Type) -> Option<String> {
        match ty {
            Type::Custom(name) => Some(Self::strip_type_generics(name)),
            Type::Parameterized(base, _) => Some(Self::strip_type_generics(base)),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::type_to_struct_base(inner)
            }
            _ => None,
        }
    }

    /// Resolve the static type backing a method-call receiver (`self`, param, `self.field`, …).
    pub(crate) fn infer_receiver_type_base(
        &self,
        object: &Expression,
        func: &FunctionDecl<'ast>,
    ) -> Option<String> {
        match object {
            Expression::Identifier { name, .. } if name == "self" => func
                .parent_type
                .as_ref()
                .map(|p| Self::strip_type_generics(p)),
            Expression::Identifier { name, .. } => func
                .parameters
                .iter()
                .find(|p| &p.name == name)
                .and_then(|p| Self::type_to_struct_base(&p.type_)),
            Expression::FieldAccess {
                object: inner,
                field,
                ..
            } => {
                let inner_base = self.infer_receiver_type_base(inner, func)?;
                self.lookup_struct_fields_for_type(&inner_base)
                    .and_then(|m| m.get(field.as_str()))
                    .and_then(Self::type_to_struct_base)
            }
            Expression::MethodCall {
                object: inner,
                method,
                ..
            } => {
                // Type-preserving methods (registry return `Self`) keep the receiver type.
                let recv = self.infer_receiver_type_base(inner, func);
                let registry = SignatureRegistry::stdlib();
                if super::stdlib_method_traits::method_is_type_preserving_qualified(
                    method,
                    recv.as_deref(),
                    registry,
                ) {
                    return recv.or_else(|| self.infer_receiver_type_base(inner, func));
                }
                None
            }
            Expression::Index { object: inner, .. } => {
                let collection_type = self.infer_receiver_type_base(inner, func)?;
                self.lookup_struct_fields_for_type(&collection_type)
                    .and(None)
                    .or_else(|| {
                        // Vec<T>, array, etc.: strip Vec wrapper to get element type.
                        // Look up the collection type as a generic (e.g. Vec<DialogueConsequence>).
                        // For now, check the struct fields registry for the inner type's
                        // generic parameter.
                        let inner_base = self.infer_receiver_type_base(inner, func)?;
                        // Try resolving through field types: if inner is self.field,
                        // look up the field type and extract the generic parameter.
                        self.resolve_index_element_type(&inner_base, inner, func)
                    })
            }
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => self.infer_receiver_type_base(operand, func),
            _ => None,
        }
    }

    fn resolve_index_element_type(
        &self,
        _collection_type_name: &str,
        object: &Expression,
        func: &FunctionDecl<'ast>,
    ) -> Option<String> {
        // Resolve the actual Type of the collection (e.g. Vec<DialogueConsequence>)
        // by tracing through the expression to find the declaring field/param type.
        let full_type = match object {
            Expression::FieldAccess {
                object: inner,
                field,
                ..
            } => {
                let inner_base = self.infer_receiver_type_base(inner, func)?;
                self.lookup_struct_fields_for_type(&inner_base)
                    .and_then(|m| m.get(field.as_str()))
                    .cloned()
            }
            Expression::Identifier { name, .. } => func
                .parameters
                .iter()
                .find(|p| &p.name == name)
                .map(|p| p.type_.clone()),
            _ => None,
        }?;
        // Extract the element type from Vec<T>, Array<T>, etc.
        match &full_type {
            Type::Array(inner, _) => Self::type_to_struct_base(inner),
            Type::Parameterized(_, params) if !params.is_empty() => {
                Self::type_to_struct_base(&params[0])
            }
            _ => None,
        }
    }

    /// Static type for `self...` receivers inside an `impl`, for `Type::method` registry keys.
    pub(crate) fn static_value_type_of_self_rooted_expr(
        &self,
        _program: &Program<'ast>,
        impl_type_base: &str,
        expr: &Expression<'ast>,
    ) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => {
                Some(Type::Custom(impl_type_base.to_string()))
            }
            Expression::FieldAccess { object, field, .. } => {
                let inner_ty =
                    self.static_value_type_of_self_rooted_expr(_program, impl_type_base, object)?;
                let inner_base = Self::type_to_struct_base(&inner_ty)?;
                self.lookup_struct_fields_for_type(&inner_base)
                    .and_then(|m| m.get(field.as_str()))
                    .cloned()
            }
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => self.static_value_type_of_self_rooted_expr(_program, impl_type_base, operand),
            _ => None,
        }
    }

    pub(crate) fn type_base_for_qualified_sig_lookup(ty: &Type) -> Option<String> {
        Self::type_to_struct_base(ty)
    }

    /// Registry lookup key matching [`SignatureRegistry`] (`Type::method`), not ambiguous `method` alone.
    pub(crate) fn qualified_method_registry_key(
        &self,
        object: &Expression,
        method: &str,
        func: &FunctionDecl<'ast>,
    ) -> String {
        self.infer_receiver_type_base(object, func)
            .map(|base| format!("{}::{}", base, method))
            .unwrap_or_else(|| method.to_string())
    }

    /// MULTI-PASS: Infer ownership from pass-through calls using signature registry
    /// If param is ONLY passed to functions whose signatures are known, match their ownership
    pub(super) fn infer_passthrough_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
        current_func_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> Option<OwnershipMode> {
        // TDD: Check for METHOD CALLS ON the parameter first (e.g., grid.set(42))
        // This determines if parameter needs &mut based on method's self type
        //
        // THE WINDJAMMER WAY: Multi-pass compilation makes this work
        // - Pass 1: Grid::set isn't registered yet, fallback to other inference
        // - Pass 2: Grid::set is registered, we look it up and see it needs &mut self
        // - Result: fill_grid(grid: &mut Grid) correctly inferred!
        if let Some(method_self_mode) =
            self.infer_from_method_calls_on_param(param_name, body, registry, Some(param_type))
        {
            return Some(method_self_mode);
        }

        // Then check for pass-through calls (parameter passed AS argument)
        // (func_name, arg_position, is_self_field_call, is_bare_fn_call)
        let mut passthrough_calls: Vec<(String, usize, bool, bool)> = Vec::new();
        self.collect_passthrough_calls(param_name, body, func, &mut passthrough_calls);

        if std::env::var("WJ_DEBUG_PASSTHROUGH").is_ok() {
            eprintln!("[PASSTHROUGH] fn={} param={} calls={:?}", current_func_name, param_name, passthrough_calls);
        }

        // Skip recursive calls to the current function to break circular ownership inference.
        // Without this, recursive functions like `traverse(bvh, ray)` calling `traverse(bvh, ray)`
        // would see their own Owned signature and keep inferring Owned, preventing convergence.
        // BUT: self.field.method() calls are on a DIFFERENT type even if the method name matches,
        // so don't filter those (e.g., Merchant::add_item calling self.inventory.add_item).
        passthrough_calls
            .retain(|(func_name, _, is_field, _)| *is_field || func_name != current_func_name);

        if passthrough_calls.is_empty() {
            return None;
        }

        let mut inferred_mode: Option<OwnershipMode> = None;

        for (func_name, arg_position, _is_field, is_bare_fn_call) in &passthrough_calls {
            // Method names (`get`, `clear`, …) collide across thousands of engine metadata
            // entries — require type-qualified keys (`HashMap::get`) for method passthrough.
            // But bare function calls like `set_if(grid)` use unqualified names that are
            // unique in the registry, so allow them through.
            // For unqualified method calls, try a suffix lookup — if there's a unique
            // `Type::method` entry, it's unambiguous and safe to use.
            if !func_name.contains("::") && !is_bare_fn_call {
                let suffix_pattern = format!("::{}", func_name);
                let suffix_matches: Vec<_> = registry
                    .all_signatures()
                    .filter(|(k, _)| k.ends_with(&suffix_pattern))
                    .collect();
                if suffix_matches.len() != 1 {
                    continue;
                }
            }
            // Look up the callee signature with multiple fallback strategies:
            // 1. Exact name (e.g., "place_marker" or "StationBuilder::place_marker")
            // 2. Suffix match (e.g., find "Type::method" from "method")
            // 3. Simple name from qualified (e.g., "place_marker" from "station_builder::place_marker")
            //    This handles cross-crate calls where metadata stores the simple name
            //    but the call site uses the module-qualified name.
            let sig = match registry.lookup_method(func_name) {
                Some(s) => s,
                None => {
                    if let Some(s) = registry.get_signature(func_name) {
                        s
                    } else if let Some(simple) = func_name.rsplit("::").next() {
                        if simple != func_name {
                            match registry.get_signature(simple) {
                                Some(s) => s,
                                None => {
                                    if std::env::var("WJ_DEBUG_PASSTHROUGH").is_ok() {
                                        let matching: Vec<_> = registry.all_signatures()
                                            .filter(|(k, _)| k.contains("clear") || k.contains("place") || k.contains("Cache"))
                                            .map(|(k, _)| k.clone())
                                            .collect();
                                        eprintln!("[PASSTHROUGH] fn={} callee={} NOT FOUND in registry (tried simple={}). Related keys: {:?}", current_func_name, func_name, simple, matching);
                                    }
                                    continue;
                                },
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            };
            if std::env::var("WJ_DEBUG_PASSTHROUGH").is_ok() {
                eprintln!("[PASSTHROUGH] fn={} callee={} found sig ownership={:?} has_self={}", current_func_name, func_name, sig.param_ownership, sig.has_self_receiver);
            }
            let adjusted_position = if sig.has_self_receiver {
                *arg_position + 1
            } else {
                *arg_position
            };
            if adjusted_position >= sig.param_ownership.len()
                && !(sig.is_extern && Self::is_windjammer_text_param_type(param_type))
            {
                continue;
            }
            if let Some(expected_ty) = sig.param_types.get(adjusted_position) {
                if !self.passthrough_types_compatible(expected_ty, param_type) {
                    continue;
                }
            }
            // Extern FFI callees take owned `String`, but Windjammer wrappers with `string`
            // formals keep Borrowed — codegen converts at the FFI boundary (string_to_ffi).
            let ownership = if sig.is_extern && Self::is_windjammer_text_param_type(param_type) {
                OwnershipMode::Borrowed
            } else if let Some(&own) = sig.param_ownership.get(adjusted_position) {
                own
            } else {
                continue;
            };
            // TDD FIX: Use the STRONGEST ownership mode, not Owned on conflict.
            // In Rust, &mut T can always be reborrowed as &T, so:
            //   MutBorrowed + Borrowed → MutBorrowed (caller provides &mut, callees reborrow as needed)
            //   MutBorrowed + Owned → Owned (one callee consumes it)
            //   Borrowed + Owned → Owned (one callee consumes it)
            // The old code returned Owned whenever any two modes disagreed, which broke
            // the common pattern of passing a &mut parameter to both mutating and read-only functions.
            inferred_mode = Some(match (inferred_mode, ownership) {
                (None, mode) => mode,
                (Some(OwnershipMode::Owned), _) | (_, OwnershipMode::Owned) => OwnershipMode::Owned,
                (Some(OwnershipMode::MutBorrowed), _) | (_, OwnershipMode::MutBorrowed) => {
                    OwnershipMode::MutBorrowed
                }
                _ => OwnershipMode::Borrowed,
            });
        }

        inferred_mode
    }

    /// TDD: Infer ownership from method calls made ON the parameter
    /// E.g., `grid.set(42)` where `set(&mut self, ...)` → grid needs `&mut Grid`
    /// E.g., `grid.get(0)` where `get(&self, ...)` → grid needs `&Grid`
    pub(crate) fn infer_from_method_calls_on_param(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
        param_type: Option<&Type>,
    ) -> Option<OwnershipMode> {
        let mut method_calls = Vec::new();
        self.collect_method_calls_on_param(param_name, body, &mut method_calls);

        if method_calls.is_empty() {
            return None;
        }

        let type_base = param_type.and_then(Self::type_to_struct_base);
        let mut max_mode: Option<OwnershipMode> = None;

        for method_name in &method_calls {
            // PRIORITY: Type-qualified lookup (e.g. MannequinCache::clear)
            // prevents collision with Vec::clear, HashMap::clear, etc.
            let sig = type_base
                .as_ref()
                .and_then(|base| registry.get_signature(&format!("{}::{}", base, method_name)))
                .or_else(|| {
                    if !registry.has_collision(method_name) {
                        registry.get_signature(method_name)
                    } else {
                        None
                    }
                });
            if let Some(sig) = sig {
                if let Some(&self_ownership) = sig.param_ownership.first() {
                    max_mode = Some(match max_mode {
                        None => self_ownership,
                        Some(current) => match (current, self_ownership) {
                            (OwnershipMode::Owned, _) | (_, OwnershipMode::Owned) => {
                                OwnershipMode::Owned
                            }
                            (OwnershipMode::MutBorrowed, _) | (_, OwnershipMode::MutBorrowed) => {
                                OwnershipMode::MutBorrowed
                            }
                            _ => OwnershipMode::Borrowed,
                        },
                    });
                }
            }
        }

        max_mode
    }

    /// Collect method calls made ON the parameter (param is the receiver)
    /// E.g., `grid.set(42)` → collect "set"
    pub(crate) fn collect_method_calls_on_param(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        results: &mut Vec<String>,
    ) {
        for stmt in body {
            self.collect_method_calls_from_stmt(param_name, stmt, results);
        }
    }

    pub(crate) fn collect_method_calls_from_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        results: &mut Vec<String>,
    ) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.collect_method_calls_from_expr(param_name, expr, results);
            }
            Statement::Let { value, .. } => {
                self.collect_method_calls_from_expr(param_name, value, results);
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_method_calls_from_expr(param_name, expr, results);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, condition, results);
                for stmt in then_block {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_method_calls_from_stmt(param_name, stmt, results);
                    }
                }
            }
            Statement::While {
                condition,
                body: while_body,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, condition, results);
                for stmt in while_body {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
            }
            Statement::For {
                iterable,
                body: for_body,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, iterable, results);
                for stmt in for_body {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_method_calls_from_expr(param_name, value, results);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.collect_method_calls_from_expr(param_name, guard, results);
                    }
                    self.collect_method_calls_from_expr(param_name, arm.body, results);
                    if let Expression::Block { statements, .. } = arm.body {
                        for stmt in statements {
                            self.collect_method_calls_from_stmt(param_name, stmt, results);
                        }
                    }
                }
            }
            Statement::Loop { body: loop_body, .. } => {
                for stmt in loop_body {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_method_calls_from_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        results: &mut Vec<String>,
    ) {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Check if method is called ON the parameter
                if self.expr_is_identifier(object, param_name) {
                    results.push(method.clone());
                }
                // Recurse into nested expressions
                self.collect_method_calls_from_expr(param_name, object, results);
                for (_, arg) in arguments {
                    self.collect_method_calls_from_expr(param_name, arg, results);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // UFCS / field-call style: `query.get(k)` may parse as Call(FieldAccess(query, get), …)
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if self.expr_is_identifier(object, param_name) {
                        results.push(field.clone());
                    }
                }
                self.collect_method_calls_from_expr(param_name, function, results);
                for (_, arg) in arguments {
                    self.collect_method_calls_from_expr(param_name, arg, results);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_method_calls_from_expr(param_name, object, results);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_method_calls_from_expr(param_name, left, results);
                self.collect_method_calls_from_expr(param_name, right, results);
            }
            Expression::Unary { operand, .. } => {
                self.collect_method_calls_from_expr(param_name, operand, results);
            }
            // TDD FIX: Recurse into TryOp (?) expressions
            // Example: loader.load(...)? wraps the method call in TryOp
            Expression::TryOp { expr, .. } => {
                self.collect_method_calls_from_expr(param_name, expr, results);
            }
            Expression::Cast { expr, .. } => {
                self.collect_method_calls_from_expr(param_name, expr, results);
            }
            // Match/if expressions lower to `Block { statements: [Statement::Match/If] }`.
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
            }
            _ => {}
        }
    }

    /// Helper: Collect all function calls where param is passed as an argument
    /// Returns (function_name, argument_position, is_self_field_call)
    pub(crate) fn collect_passthrough_calls(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        func: &FunctionDecl<'ast>,
        results: &mut Vec<(String, usize, bool, bool)>,
    ) {
        for stmt in body {
            self.collect_passthrough_from_stmt(param_name, stmt, func, results);
        }
    }

    pub(crate) fn collect_passthrough_from_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        func: &FunctionDecl<'ast>,
        results: &mut Vec<(String, usize, bool, bool)>,
    ) {
        match stmt {
            Statement::Expression {
                expr: expression, ..
            } => {
                self.collect_passthrough_from_expr(param_name, expression, func, results);
            }
            Statement::Let { value, .. } => {
                self.collect_passthrough_from_expr(param_name, value, func, results);
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_passthrough_from_expr(param_name, expr, func, results);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, condition, func, results);
                for stmt in then_block {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                    }
                }
            }
            Statement::While {
                condition,
                body: while_body,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, condition, func, results);
                for stmt in while_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Statement::For {
                iterable,
                body: for_body,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, iterable, func, results);
                for stmt in for_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_passthrough_from_expr(param_name, value, func, results);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.collect_passthrough_from_expr(param_name, guard, func, results);
                    }
                    self.collect_passthrough_from_expr(param_name, arm.body, func, results);
                }
            }
            Statement::Assignment { value, .. } => {
                self.collect_passthrough_from_expr(param_name, value, func, results);
            }
            _ => {}
        }
    }

    pub(crate) fn collect_passthrough_from_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        func: &FunctionDecl<'ast>,
        results: &mut Vec<(String, usize, bool, bool)>,
    ) {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let is_bare = matches!(&**function, Expression::Identifier { .. });
                for (i, (_name, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(arg, param_name) {
                        if let Some(func_name) = self.extract_function_name(function) {
                            results.push((func_name, i, false, is_bare));
                        }
                    }
                }
                self.collect_passthrough_from_expr(param_name, function, func, results);
                for (_name, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, func, results);
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let is_self_field_call = matches!(&**object, Expression::FieldAccess { object: inner, .. }
                    if matches!(&**inner, Expression::Identifier { name, .. } if name == "self"));
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(arg, param_name) {
                        let method_key = self.qualified_method_registry_key(object, method, func);
                        results.push((method_key, i, is_self_field_call, false));
                    }
                }
                self.collect_passthrough_from_expr(param_name, object, func, results);
                for (_, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, func, results);
                }
            }
            Expression::TryOp { expr, .. } => {
                self.collect_passthrough_from_expr(param_name, expr, func, results);
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Expression::Unary { operand, .. } => {
                self.collect_passthrough_from_expr(param_name, operand, func, results);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_passthrough_from_expr(param_name, left, func, results);
                self.collect_passthrough_from_expr(param_name, right, func, results);
            }
            Expression::Index { object, index, .. } => {
                self.collect_passthrough_from_expr(param_name, object, func, results);
                self.collect_passthrough_from_expr(param_name, index, func, results);
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_passthrough_from_expr(param_name, object, func, results);
            }
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    self.collect_passthrough_from_expr(param_name, e, func, results);
                }
            }
            Expression::MacroInvocation { name, args, .. } => {
                let borrows_only = matches!(
                    name.as_str(),
                    "format"
                        | "println"
                        | "print"
                        | "eprintln"
                        | "eprint"
                        | "write"
                        | "writeln"
                        | "panic"
                        | "debug"
                        | "info"
                        | "warn"
                        | "error"
                        | "trace"
                        | "log"
                );
                if borrows_only {
                    for (i, arg) in args.iter().enumerate() {
                        if self.expr_is_identifier(arg, param_name) {
                            results.push((name.clone(), i, false, true));
                        }
                        self.collect_passthrough_from_expr(param_name, arg, func, results);
                    }
                } else {
                    for arg in args {
                        if self.expr_is_identifier(arg, param_name) {
                            results.push((name.clone(), 0, false, true));
                        }
                        self.collect_passthrough_from_expr(param_name, arg, func, results);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn expr_is_identifier(&self, expr: &Expression, name: &str) -> bool {
        matches!(expr, Expression::Identifier { name: id, .. } if id == name)
    }

    /// True when `expr` is `param.field` or a nested field chain rooted at `param`.
    fn expr_is_field_move_from_param(param_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(**object, Expression::Identifier { ref name, .. } if name == param_name)
                    || Self::expr_is_field_move_from_param(param_name, object)
            }
            _ => false,
        }
    }

    /// True when `expr` is `param.field`, `param[i]`, or a nested chain rooted at `param`.
    fn expr_is_field_or_index_rooted_at_param(param_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                matches!(**object, Expression::Identifier { ref name, .. } if name == param_name)
                    || Self::expr_is_field_or_index_rooted_at_param(param_name, object)
            }
            _ => false,
        }
    }

    pub(crate) fn extract_function_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                if let Some(prefix) = self.extract_function_name(object) {
                    Some(format!("{}::{}", prefix, field))
                } else {
                    Some(field.clone())
                }
            }
            _ => None,
        }
    }

    /// True when `param` is only passed as the key to HashMap-style lookup methods.
    pub(crate) fn is_only_hashmap_lookup_key_param(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        func: &FunctionDecl<'ast>,
    ) -> bool {
        let mut lookups = Vec::new();
        self.collect_hashmap_lookup_key_uses(param_name, body, func, &mut lookups);
        if lookups.is_empty() {
            return false;
        }
        let mut other_uses = false;
        self.collect_non_lookup_param_uses(param_name, body, &mut other_uses);
        !other_uses
    }

    /// True when `param` is only read via field/index chains (e.g. `key.bytes == …`),
    /// never as a bare value, method receiver, or call argument.
    pub(crate) fn is_field_access_only_param_usage(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        let mut any_use = false;
        let mut bad_use = false;
        for stmt in body {
            self.check_field_only_param_use_stmt(param_name, stmt, false, &mut any_use, &mut bad_use);
            if bad_use {
                return false;
            }
        }
        any_use && !bad_use
    }

    /// True when a `let` binding moves a field/index off `param` (`let bytes = key.bytes`).
    pub(crate) fn param_has_field_or_index_move_binding(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        body.iter()
            .any(|stmt| self.stmt_has_field_or_index_move_binding(param_name, stmt))
    }

    /// True when a non-Copy `param.field` is passed as a call/method argument
    /// (`return_f64(buf.scores)`). That is a partial move — keep the param Owned (WDB-096).
    pub(crate) fn param_projects_non_copy_field_into_call_arg(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        body.iter().any(|stmt| {
            self.stmt_projects_non_copy_field_into_call_arg(param_name, param_type, stmt)
        })
    }

    fn stmt_projects_non_copy_field_into_call_arg(
        &self,
        param_name: &str,
        param_type: &Type,
        stmt: &Statement,
    ) -> bool {
        match stmt {
            Statement::Let { value, .. }
            | Statement::Assignment { value, .. }
            | Statement::Expression { expr: value, .. }
            | Statement::Return {
                value: Some(value), ..
            } => self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, value),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, condition)
                    || then_block.iter().any(|s| {
                        self.stmt_projects_non_copy_field_into_call_arg(param_name, param_type, s)
                    })
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.stmt_projects_non_copy_field_into_call_arg(
                                param_name, param_type, s,
                            )
                        })
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, condition)
                    || body.iter().any(|s| {
                        self.stmt_projects_non_copy_field_into_call_arg(param_name, param_type, s)
                    })
            }
            Statement::For {
                iterable, body, ..
            } => {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, iterable)
                    || body.iter().any(|s| {
                        self.stmt_projects_non_copy_field_into_call_arg(param_name, param_type, s)
                    })
            }
            Statement::Match { value, arms, .. } => {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, value)
                    || arms.iter().any(|arm| {
                        arm.guard.is_some_and(|g| {
                            self.expr_projects_non_copy_field_into_call_arg(
                                param_name, param_type, g,
                            )
                        }) || self.expr_projects_non_copy_field_into_call_arg(
                            param_name,
                            param_type,
                            arm.body,
                        )
                    })
            }
            _ => false,
        }
    }

    fn expr_projects_non_copy_field_into_call_arg(
        &self,
        param_name: &str,
        param_type: &Type,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                arguments.iter().any(|(_, arg)| {
                    self.expr_is_non_copy_field_projection_from_param(param_name, param_type, arg)
                        || self.expr_projects_non_copy_field_into_call_arg(
                            param_name, param_type, arg,
                        )
                })
            }
            Expression::FieldAccess { object, .. }
            | Expression::Index { object, .. }
            | Expression::Unary { operand: object, .. }
            | Expression::TryOp { expr: object, .. } => {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, object)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, left)
                    || self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, right)
            }
            Expression::Tuple { elements, .. } => elements.iter().any(|e| {
                self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, e)
            }),
            Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, v)| {
                self.expr_is_non_copy_field_projection_from_param(param_name, param_type, v)
                    || self.expr_projects_non_copy_field_into_call_arg(param_name, param_type, v)
            }),
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.stmt_projects_non_copy_field_into_call_arg(param_name, param_type, s)
            }),
            _ => false,
        }
    }

    fn expr_is_non_copy_field_projection_from_param(
        &self,
        param_name: &str,
        param_type: &Type,
        expr: &Expression,
    ) -> bool {
        let Expression::FieldAccess { object, field, .. } = expr else {
            return false;
        };
        if !matches!(&**object, Expression::Identifier { name, .. } if name == param_name) {
            // Nested `param.a.b` — treat as projection if the root is the param and
            // the final field type is non-Copy.
            let Some(root) = Self::root_identifier_name(expr) else {
                return false;
            };
            if root != param_name {
                return false;
            }
            return self
                .resolve_field_chain_type_from_param(param_type, expr)
                .is_some_and(|ty| !self.is_copy_type(&ty));
        }
        let type_name = match param_type {
            Type::Custom(n) => n.as_str(),
            Type::Reference(inner) | Type::MutableReference(inner) => match &**inner {
                Type::Custom(n) => n.as_str(),
                _ => return !self.is_copy_type(param_type),
            },
            _ => return false,
        };
        let Some(fields) = self.lookup_struct_fields_for_type(type_name) else {
            // Unknown struct layout — be conservative: non-Copy custom params projecting
            // fields into calls keep Owned (safe; shared formals still auto-ref).
            return !self.is_copy_type(param_type);
        };
        fields
            .get(field.as_str())
            .is_some_and(|ft| !self.is_copy_type(ft))
    }

    fn root_identifier_name(expr: &Expression<'_>) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                Self::root_identifier_name(object)
            }
            _ => None,
        }
    }

    fn resolve_field_chain_type_from_param(
        &self,
        param_type: &Type,
        expr: &Expression,
    ) -> Option<Type> {
        match expr {
            Expression::Identifier { .. } => Some(param_type.clone()),
            Expression::FieldAccess { object, field, .. } => {
                let parent_ty = self.resolve_field_chain_type_from_param(param_type, object)?;
                let type_name = match parent_ty {
                    Type::Custom(n) => n,
                    Type::Reference(inner) | Type::MutableReference(inner) => match *inner {
                        Type::Custom(n) => n,
                        _ => return None,
                    },
                    _ => return None,
                };
                self.lookup_struct_fields_for_type(&type_name)?
                    .get(field.as_str())
                    .cloned()
            }
            _ => None,
        }
    }

    fn stmt_has_field_or_index_move_binding(&self, param_name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let { value, .. } => {
                Self::expr_is_field_move_from_param(param_name, value)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.stmt_has_field_or_index_move_binding(param_name, s))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_has_field_or_index_move_binding(param_name, s))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } | Statement::Loop { body, .. } => {
                body.iter()
                    .any(|s| self.stmt_has_field_or_index_move_binding(param_name, s))
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                Self::expr_is_field_move_from_param(param_name, &arm.body)
            }),
            _ => false,
        }
    }

    fn check_field_only_param_use_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        in_field_chain: bool,
        any_use: &mut bool,
        bad_use: &mut bool,
    ) {
        if *bad_use {
            return;
        }
        match stmt {
            Statement::Let { value: expr, .. } => {
                // `let x = param.field` moves the field — not readonly projection.
                if Self::expr_is_field_move_from_param(param_name, expr) {
                    *any_use = true;
                    *bad_use = true;
                    return;
                }
                self.check_field_only_param_use_expr(
                    param_name,
                    expr,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
            }
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => {
                self.check_field_only_param_use_expr(
                    param_name,
                    expr,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.check_field_only_param_use_expr(
                    param_name,
                    condition,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                for s in then_block {
                    self.check_field_only_param_use_stmt(param_name, s, in_field_chain, any_use, bad_use);
                }
                if let Some(else_block) = else_block {
                    for s in else_block {
                        self.check_field_only_param_use_stmt(param_name, s, in_field_chain, any_use, bad_use);
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                self.check_field_only_param_use_expr(
                    param_name,
                    value,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.check_field_only_param_use_expr(
                            param_name,
                            guard,
                            in_field_chain,
                            any_use,
                            bad_use,
                        );
                    }
                    self.check_field_only_param_use_expr(
                        param_name,
                        &arm.body,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.check_field_only_param_use_expr(
                    param_name,
                    condition,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                for s in body {
                    self.check_field_only_param_use_stmt(param_name, s, in_field_chain, any_use, bad_use);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.check_field_only_param_use_expr(
                    param_name,
                    iterable,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                for s in body {
                    self.check_field_only_param_use_stmt(param_name, s, in_field_chain, any_use, bad_use);
                }
            }
            _ => {}
        }
    }

    fn check_field_only_param_use_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        in_field_chain: bool,
        any_use: &mut bool,
        bad_use: &mut bool,
    ) {
        if *bad_use {
            return;
        }
        match expr {
            Expression::Identifier { name, .. } if name == param_name => {
                *any_use = true;
                if !in_field_chain {
                    *bad_use = true;
                }
            }
            Expression::FieldAccess { object, .. } => {
                if self.expr_is_identifier(object, param_name) {
                    *any_use = true;
                } else {
                    self.check_field_only_param_use_expr(
                        param_name,
                        object,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            Expression::Index { object, index, .. } => {
                let object_is_param = self.expr_is_identifier(object, param_name);
                self.check_field_only_param_use_expr(
                    param_name,
                    object,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                self.check_field_only_param_use_expr(
                    param_name,
                    index,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                if object_is_param {
                    *bad_use = true;
                }
            }
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                if self.expr_is_identifier(object, param_name) {
                    *any_use = true;
                    *bad_use = true;
                    return;
                }
                self.check_field_only_param_use_expr(
                    param_name,
                    object,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                for (_, arg) in arguments {
                    if self.expr_is_identifier(arg, param_name) {
                        *any_use = true;
                        *bad_use = true;
                        return;
                    }
                    self.check_field_only_param_use_expr(
                        param_name,
                        arg,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                for (_, arg) in arguments {
                    if self.expr_is_identifier(arg, param_name) {
                        *any_use = true;
                        *bad_use = true;
                        return;
                    }
                }
                self.check_field_only_param_use_expr(
                    param_name,
                    function,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
                for (_, arg) in arguments {
                    self.check_field_only_param_use_expr(
                        param_name,
                        arg,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            Expression::Unary { operand, .. } | Expression::TryOp { expr: operand, .. } => {
                self.check_field_only_param_use_expr(
                    param_name,
                    operand,
                    in_field_chain,
                    any_use,
                    bad_use,
                );
            }
            Expression::Binary { left, right, op, .. } => {
                // String concat (`+`) of `param.field` consumes projected non-Copy data
                // (`deps.tag + ":"` → keep owned AppDeps). Numeric ops on Copy fields
                // (`a.x - b.x`) stay readonly field-chain usage → borrowed Body.
                use crate::parser::ast::operators::BinaryOp;
                if matches!(op, BinaryOp::Add) {
                    self.check_field_only_binary_operand(
                        param_name,
                        left,
                        any_use,
                        bad_use,
                    );
                    self.check_field_only_binary_operand(
                        param_name,
                        right,
                        any_use,
                        bad_use,
                    );
                } else {
                    self.check_field_only_param_use_expr(
                        param_name,
                        left,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                    self.check_field_only_param_use_expr(
                        param_name,
                        right,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    self.check_field_only_param_use_expr(
                        param_name,
                        e,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.check_field_only_param_use_stmt(
                        param_name,
                        stmt,
                        in_field_chain,
                        any_use,
                        bad_use,
                    );
                }
            }
            _ => {}
        }
    }

    /// Binary operands that read `param.field` (not `param.field.method()`) consume the
    /// projected value — composition helpers like `deps.tag + ":"` must keep owned formals.
    fn check_field_only_binary_operand(
        &self,
        param_name: &str,
        expr: &Expression,
        any_use: &mut bool,
        bad_use: &mut bool,
    ) {
        if *bad_use {
            return;
        }
        match expr {
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. }
                if self.expr_is_identifier(object, param_name)
                    || Self::field_chain_mentions_param(object, param_name) =>
            {
                *any_use = true;
                *bad_use = true;
            }
            _ => {
                self.check_field_only_param_use_expr(param_name, expr, false, any_use, bad_use);
            }
        }
    }

    fn field_chain_mentions_param(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier { name: n, .. } => n == name,
            Expression::FieldAccess { object, .. }
            | Expression::Index { object, .. }
            | Expression::Unary { operand: object, .. }
            | Expression::TryOp { expr: object, .. }
            | Expression::Await { expr: object, .. }
            | Expression::Cast { expr: object, .. } => {
                Self::field_chain_mentions_param(object, name)
            }
            _ => false,
        }
    }

    fn collect_hashmap_lookup_key_uses(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        func: &FunctionDecl<'ast>,
        results: &mut Vec<()>,
    ) {
        for stmt in body {
            self.collect_hashmap_lookup_key_uses_stmt(param_name, stmt, func, results);
        }
    }

    fn collect_hashmap_lookup_key_uses_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        func: &FunctionDecl<'ast>,
        results: &mut Vec<()>,
    ) {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Let { value: expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, expr, func, results);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, condition, func, results);
                for s in then_block {
                    self.collect_hashmap_lookup_key_uses_stmt(param_name, s, func, results);
                }
                if let Some(else_block) = else_block {
                    for s in else_block {
                        self.collect_hashmap_lookup_key_uses_stmt(param_name, s, func, results);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, condition, func, results);
                for s in body {
                    self.collect_hashmap_lookup_key_uses_stmt(param_name, s, func, results);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, iterable, func, results);
                for s in body {
                    self.collect_hashmap_lookup_key_uses_stmt(param_name, s, func, results);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, value, func, results);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.collect_hashmap_lookup_key_uses_expr(param_name, guard, func, results);
                    }
                    self.collect_hashmap_lookup_key_uses_expr(param_name, arm.body, func, results);
                }
            }
            _ => {}
        }
    }

    fn collect_hashmap_lookup_key_uses_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        func: &FunctionDecl<'ast>,
        results: &mut Vec<()>,
    ) {
        if let Some((object, _method, arguments)) =
            super::stdlib_method_traits::decompose_collection_key_lookup(expr)
        {
            if arguments
                .first()
                .is_some_and(|(_, arg)| self.expr_is_identifier(arg, param_name))
            {
                results.push(());
            }
            self.collect_hashmap_lookup_key_uses_expr(param_name, object, func, results);
            for (_, arg) in arguments {
                self.collect_hashmap_lookup_key_uses_expr(param_name, arg, func, results);
            }
            return;
        }
        match expr {
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, object, func, results);
                for (_, arg) in arguments {
                    self.collect_hashmap_lookup_key_uses_expr(param_name, arg, func, results);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, function, func, results);
                for (_, arg) in arguments {
                    self.collect_hashmap_lookup_key_uses_expr(param_name, arg, func, results);
                }
            }
            Expression::FieldAccess { object, .. }
            | Expression::Unary {
                operand: object, ..
            }
            | Expression::TryOp { expr: object, .. } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, object, func, results);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_hashmap_lookup_key_uses_expr(param_name, left, func, results);
                self.collect_hashmap_lookup_key_uses_expr(param_name, right, func, results);
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_hashmap_lookup_key_uses_stmt(param_name, stmt, func, results);
                }
            }
            _ => {}
        }
    }

    fn is_hashmap_lookup_method(method: &str) -> bool {
        super::stdlib_method_traits::is_map_key_method(method)
    }

    fn collect_non_lookup_param_uses(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        found: &mut bool,
    ) {
        if *found {
            return;
        }
        for stmt in body {
            self.collect_non_lookup_param_uses_stmt(param_name, stmt, found);
        }
    }

    fn collect_non_lookup_param_uses_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        found: &mut bool,
    ) {
        if *found {
            return;
        }
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Let { value: expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_non_lookup_param_uses_expr(param_name, expr, found);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_non_lookup_param_uses_expr(param_name, condition, found);
                for s in then_block {
                    self.collect_non_lookup_param_uses_stmt(param_name, s, found);
                }
                if let Some(else_block) = else_block {
                    for s in else_block {
                        self.collect_non_lookup_param_uses_stmt(param_name, s, found);
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_non_lookup_param_uses_expr(param_name, value, found);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.collect_non_lookup_param_uses_expr(param_name, guard, found);
                    }
                    self.collect_non_lookup_param_uses_expr(param_name, arm.body, found);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.collect_non_lookup_param_uses_expr(param_name, condition, found);
                for s in body {
                    self.collect_non_lookup_param_uses_stmt(param_name, s, found);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.collect_non_lookup_param_uses_expr(param_name, iterable, found);
                for s in body {
                    self.collect_non_lookup_param_uses_stmt(param_name, s, found);
                }
            }
            _ => {}
        }
    }

    fn collect_non_lookup_param_uses_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        found: &mut bool,
    ) {
        if *found {
            return;
        }
        match expr {
            Expression::Identifier { name, .. } if name == param_name => {
                *found = true;
            }
            _ if super::stdlib_method_traits::decompose_collection_key_lookup(expr)
                .is_some_and(|(_, _method, args)| {
                    args.first()
                        .is_some_and(|(_, arg)| self.expr_is_identifier(arg, param_name))
                }) =>
            {
                if let Some((object, _method, arguments)) =
                    super::stdlib_method_traits::decompose_collection_key_lookup(expr)
                {
                    self.collect_non_lookup_param_uses_expr(param_name, object, found);
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if i == 0 {
                            continue;
                        }
                        self.collect_non_lookup_param_uses_expr(param_name, arg, found);
                    }
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let is_lookup = Self::is_hashmap_lookup_method(method)
                    && arguments
                        .first()
                        .is_some_and(|(_, arg)| self.expr_is_identifier(arg, param_name));
                if !is_lookup {
                    for (_, arg) in arguments {
                        if self.expr_is_identifier(arg, param_name) {
                            *found = true;
                            return;
                        }
                    }
                }
                self.collect_non_lookup_param_uses_expr(param_name, object, found);
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    // HashMap lookup keys are reads — do not count the key argument as a
                    // separate non-lookup use (fixes `match map.get(id)` false positive).
                    if is_lookup && i == 0 {
                        continue;
                    }
                    self.collect_non_lookup_param_uses_expr(param_name, arg, found);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                for (_, arg) in arguments {
                    if self.expr_is_identifier(arg, param_name) {
                        *found = true;
                        return;
                    }
                }
                self.collect_non_lookup_param_uses_expr(param_name, function, found);
                for (_, arg) in arguments {
                    self.collect_non_lookup_param_uses_expr(param_name, arg, found);
                }
            }
            Expression::FieldAccess { object, .. }
            | Expression::Unary {
                operand: object, ..
            }
            | Expression::TryOp { expr: object, .. } => {
                self.collect_non_lookup_param_uses_expr(param_name, object, found);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_non_lookup_param_uses_expr(param_name, left, found);
                self.collect_non_lookup_param_uses_expr(param_name, right, found);
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_non_lookup_param_uses_stmt(param_name, stmt, found);
                }
            }
            _ => {}
        }
    }

    /// Module-level `string` params that only forward to a peer function which forwards back
    /// (e.g. `foo(x)` ↔ `bar(y)`) must keep owned `String` at call sites — stale
    /// `Borrowed`/`Reference(str)` from incomplete multipass must not win.
    pub(crate) fn module_string_in_mutual_recursion_owned_contract(
        &self,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        let Some(ref tops) = self.top_level_functions else {
            return false;
        };

        let mut passthrough_calls: Vec<(String, usize, bool, bool)> = Vec::new();
        self.collect_passthrough_calls(param_name, &func.body, func, &mut passthrough_calls);

        for (callee_name, arg_pos, _is_field, is_bare) in passthrough_calls {
            if !is_bare {
                continue;
            }
            if callee_name == func.name {
                return true;
            }
            let Some(callee_decl) = tops.get(&callee_name) else {
                continue;
            };
            let Some(callee_param) = callee_decl.parameters.get(arg_pos) else {
                continue;
            };
            if !Self::is_windjammer_text_param_type(&callee_param.type_) {
                continue;
            }
            let mut back_calls: Vec<(String, usize, bool, bool)> = Vec::new();
            self.collect_passthrough_calls(
                &callee_param.name,
                &callee_decl.body,
                callee_decl,
                &mut back_calls,
            );
            if back_calls
                .iter()
                .any(|(name, _, _, bare)| *bare && name == &func.name)
            {
                return true;
            }
            // Registry-backed one-hop: callee already converged to forward to us.
            let _ = registry;
        }
        false
    }
}
