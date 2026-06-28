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
                // clone(), to_owned(), etc. preserve the receiver type.
                if super::stdlib_method_traits::is_type_preserving(method) {
                    return self.infer_receiver_type_base(inner, func);
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
                    if let Some(simple) = func_name.rsplit("::").next() {
                        if simple != func_name {
                            match registry.get_signature(simple) {
                                Some(s) => s,
                                None => continue,
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            };
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
            _ => {}
        }
    }

    pub(super) fn expr_is_identifier(&self, expr: &Expression, name: &str) -> bool {
        matches!(expr, Expression::Identifier { name: id, .. } if id == name)
    }

    pub(crate) fn extract_function_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { field, .. } => Some(field.clone()),
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
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } if Self::is_hashmap_lookup_method(method) => {
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
            }
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
}
