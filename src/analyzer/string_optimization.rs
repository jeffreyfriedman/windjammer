/// String Parameter Optimization Analyzer
///
/// Phase 2 of the String Parameter Optimization plan:
/// Analyzes function bodies to determine if borrowed string parameters can safely
/// use `&str` instead of `&String`.
///
/// PROPER APPROACH (No String Matching!):
/// Instead of hard-coding method names, we analyze method signatures:
/// 1. Look up the method in the type registry
/// 2. Check if any parameter types are `&String` or `&T where T: Borrow<String>`
/// 3. If a parameter is passed to such a method → use &String (correctness)
/// 4. Otherwise → use &str (performance)
///
/// This is:
/// - Extensible: Works with custom methods automatically
/// - Maintainable: No hard-coded string lists
/// - Correct: Based on actual type system, not heuristics
///
/// PHASE 2 MVP: Conservative implementation - returns empty set (uses &String everywhere)
/// Full implementation requires:
/// 1. Method signature lookup in type registry
/// 2. AST traversal to find method calls
/// 3. Parameter flow analysis (which params are passed where)
use crate::analyzer::Analyzer;
use crate::parser::{Expression, FunctionDecl, Statement, Type};
use std::collections::HashSet;

impl<'ast> Analyzer<'ast> {
    /// Analyze all string parameters in a function and return the set that can use &str
    ///
    /// PHASE 3: Manual Override Support
    /// - Check for @str_ref decorator → force &str (developer promises it's safe)
    /// - Check for @string_ref decorator → force &String (developer wants conservative)
    ///
    /// THE PROPER WAY (Phase 2 full):
    /// - Traverse function body AST
    /// - For each method call, look up its signature in type registry
    /// - Check if any method parameter expects &String (not &str)
    /// - If parameter flows to such a method → must use &String
    /// - Otherwise → can safely use &str
    ///
    /// PHASE 2 FULL: Implement type-based analysis using signature registry
    /// Analyzes function body to determine which string parameters can safely use &str
    pub fn analyze_str_ref_optimizable_params(
        &self,
        func: &FunctionDecl,
        registry: &super::SignatureRegistry,
    ) -> HashSet<String> {
        // Extern functions use FFI types - never optimize their parameters
        if func.is_extern {
            return HashSet::new();
        }

        let mut optimizable = HashSet::new();

        for param in &func.parameters {
            // Only consider string parameters
            let is_string = matches!(param.type_, Type::String)
                || matches!(param.type_, Type::Custom(ref name) if name == "string");

            if !is_string {
                continue;
            }

            // Check for explicit decorators
            let has_str_ref = param.decorators.iter().any(|d| d.name == "str_ref");
            let has_string_ref = param.decorators.iter().any(|d| d.name == "string_ref");

            if has_str_ref {
                // PHASE 3: Developer explicitly requested &str
                // Trust the developer - they promise it's safe
                optimizable.insert(param.name.clone());
            } else if has_string_ref {
                // PHASE 3: Developer explicitly requested &String
                // Don't optimize this parameter
                continue;
            } else {
                // No decorator - use automatic analysis

                let needs_string_ref =
                    self.param_needs_string_ref(&param.name, &func.body, registry);

                if !needs_string_ref {
                    optimizable.insert(param.name.clone());
                }
            }
        }

        optimizable
    }

    /// Check if a parameter needs &String (passed to method that requires it)
    /// Recursively traverses the function body to find all usages
    pub(crate) fn param_needs_string_ref(
        &self,
        param_name: &str,
        body: &[&Statement],
        registry: &super::SignatureRegistry,
    ) -> bool {
        for stmt in body {
            if self.statement_uses_param_in_string_ref_context(param_name, stmt, registry) {
                return true;
            }
        }
        false
    }

    /// Check if a statement uses the parameter in a context requiring &String or String (owned)
    pub(crate) fn statement_uses_param_in_string_ref_context(
        &self,
        param_name: &str,
        stmt: &Statement,
        registry: &super::SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, expr, registry)
            }
            Statement::Let { value, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, value, registry)
            }
            // TDD FIX: Check for direct assignment to String fields
            // If `self.name = name` where self.name is String, parameter must be String (owned), not &str
            Statement::Assignment { target, value, .. } => {
                // Check if the value is our parameter (or & to our parameter)
                let value_is_param = self.expr_is_param_or_ref_to_param(param_name, value);

                if value_is_param {
                    // Check if target is a String field
                    // For simplicity, if assigning parameter directly to ANY field, be conservative
                    // and require &String (the codegen will handle owned String if needed)
                    // This prevents &str → String assignment errors
                    if matches!(target, Expression::FieldAccess { .. }) {
                        return true; // Assignment to field requires owned/&String, not &str
                    }
                }

                // Recursively check both target and value
                self.expr_uses_param_in_string_ref_context(param_name, target, registry)
                    || self.expr_uses_param_in_string_ref_context(param_name, value, registry)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_uses_param_in_string_ref_context(param_name, condition, registry)
                    || self.block_needs_string_ref(param_name, then_block, registry)
                    || else_block
                        .as_ref()
                        .map(|b| self.block_needs_string_ref(param_name, b, registry))
                        .unwrap_or(false)
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_uses_param_in_string_ref_context(param_name, condition, registry)
                    || self.block_needs_string_ref(param_name, body, registry)
            }
            Statement::For { body, .. } => self.block_needs_string_ref(param_name, body, registry),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_uses_param_in_string_ref_context(param_name, expr, registry),
            Statement::Match { value, arms, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, value, registry)
                    || arms.iter().any(|arm| {
                        self.expr_uses_param_in_string_ref_context(param_name, arm.body, registry)
                    })
            }
            _ => false,
        }
    }

    /// Check if a block needs &String (block is Vec<&Statement>)
    pub(crate) fn block_needs_string_ref(
        &self,
        param_name: &str,
        block: &Vec<&Statement>,
        registry: &super::SignatureRegistry,
    ) -> bool {
        for stmt in block {
            if self.statement_uses_param_in_string_ref_context(param_name, stmt, registry) {
                return true;
            }
        }
        false
    }

    /// Check if an expression uses the parameter in a context requiring &String
    pub(crate) fn expr_uses_param_in_string_ref_context(
        &self,
        param_name: &str,
        expr: &Expression,
        registry: &super::SignatureRegistry,
    ) -> bool {
        match expr {
            // Check method calls: param.method() or something.method(&param)
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // First check if any argument is our parameter (like items.contains(&id))
                for (idx, arg) in arguments.iter().enumerate() {
                    let arg_expr = &arg.1;

                    // Check if this argument is &param or param
                    if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                        // CONSERVATIVE HEURISTIC: If parameter is passed to a method on self (e.g., self.log(message)),
                        // and we don't have signature information, conservatively assume owned String is needed.
                        // This handles transitive dependencies like info(message) → log(message) → push(message).
                        // Known read-only methods are excluded from this heuristic.
                        let is_self_method = match &**object {
                            Expression::Identifier { name, .. } => name == "self",
                            // Also handle self.field (e.g., self.data.insert(...))
                            Expression::FieldAccess { object: inner, .. } => {
                                matches!(&**inner, Expression::Identifier { name, .. } if name == "self")
                            }
                            _ => false,
                        };

                        // Self-method calls (self.log(message), self.data.insert(key, val)):
                        // If ownership analyzer determined the param as Borrowed, then the
                        // downstream method that receives it will also have its string param
                        // analyzed. Known problematic stdlib methods (contains, push, insert)
                        // are handled by special cases below.
                        // No extra conservative block needed for self methods.
                        let _ = is_self_method;

                        if super::stdlib_method_traits::is_slice_search_method(method) && idx == 0 {
                            return true;
                        }

                        if super::stdlib_method_traits::is_storage_method(method) && idx == 0 {
                            return true;
                        }

                        if method == "insert" && idx == 1 {
                            return true;
                        }

                        // HashMap/BTreeMap key methods take `&K`; codegen passes `key` directly
                        // when the parameter already generates as `&str`/`&String` — no &String
                        // requirement here (that caused circular &&str bugs).

                        // Check if this method expects &String or String (owned) for this parameter position.
                        // Static/type calls (`Quest::new`) must use qualified keys — bare `new`
                        // hits unrelated constructors in the registry.
                        let method_sig = if let Expression::Identifier { name, .. } = &**object {
                            if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                                registry.get_signature(&format!("{}::{}", name, method))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                        .or_else(|| registry.lookup_method(method));

                        if let Some(sig) = method_sig {
                            if let Some(param_type) = sig.param_type_for_arg(idx) {
                                if self.type_is_string_ref_not_str(param_type) {
                                    return true;
                                }
                                if self.is_windjammer_string_param_type(param_type) {
                                    if self.callee_string_param_uses_rust_string_ref(
                                        sig, idx, param_type, method,
                                    ) {
                                        return true;
                                    }
                                    continue;
                                }
                                if self.type_is_owned_string(param_type) {
                                    return true;
                                }
                            }
                        }
                    }

                    // Recursively check argument expressions
                    if self.expr_uses_param_in_string_ref_context(param_name, arg_expr, registry) {
                        return true;
                    }
                }

                // param.method() — receiver (`self`) requirements are handled by
                // ownership inference, not string-ref analysis. Do not scan other
                // parameters on the callee (e.g. `inv.has(id)` must not mark `inv`
                // as &String because `id` needs &String).
                if let Expression::Identifier { name, .. } = &**object {
                    let _ = (name, method);
                }

                false
            }
            // Check function calls: function(&param)
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Type::method(...) constructor calls (parsed as Call(FieldAccess)).
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    let is_constructor = field.starts_with("new") || field.starts_with("from_");
                    if is_constructor {
                        let qualified = if let Expression::Identifier { name, .. } = &**object {
                            Some(format!("{}::{}", name, field))
                        } else {
                            None
                        };
                        for (i, arg) in arguments.iter().enumerate() {
                            let arg_expr = &arg.1;
                            if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                                if let Some(ref qname) = qualified {
                                    if let Some(sig) = registry.get_signature(qname) {
                                        if let Some(param_type) = sig.param_type_for_arg(i) {
                                            if self.type_is_string_ref_not_str(param_type) {
                                                return true;
                                            }
                                            if self.is_windjammer_string_param_type(param_type) {
                                                if self.callee_string_param_uses_rust_string_ref(
                                                    sig, i, param_type, field,
                                                ) {
                                                    return true;
                                                }
                                                continue;
                                            }
                                            if self.type_is_owned_string(param_type) {
                                                return true;
                                            }
                                            continue;
                                        }
                                    }
                                }
                                return true;
                            }
                        }
                    }
                }

                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    // Enum variants (Some, None, Ok, Err, MyEnum::Variant) consume
                    // their arguments. Detect enum variants vs module-qualified fn
                    // calls: enum variants have an uppercase final component.
                    let is_enum_variant =
                        matches!(fn_name.as_str(), "Some" | "None" | "Ok" | "Err")
                            || (fn_name.contains("::") && {
                                let last = fn_name.rsplit("::").next().unwrap_or("");
                                last.starts_with(|c: char| c.is_uppercase())
                            });

                    let is_constructor = fn_name.contains("::") && {
                        let last = fn_name.rsplit("::").next().unwrap_or("");
                        last.starts_with("new") || last.starts_with("from_")
                    };

                    if is_enum_variant {
                        for arg in arguments.iter() {
                            let arg_expr = &arg.1;
                            if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                                return true;
                            }
                        }
                    }

                    if is_constructor {
                        for (i, arg) in arguments.iter().enumerate() {
                            let arg_expr = &arg.1;
                            if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                                if let Some(sig) = registry.get_signature(fn_name) {
                                    if let Some(param_type) = sig.param_type_for_arg(i) {
                                        if self.type_is_string_ref_not_str(param_type) {
                                            return true;
                                        }
                                        if self.is_windjammer_string_param_type(param_type) {
                                            if self.callee_string_param_uses_rust_string_ref(
                                                sig, i, param_type, fn_name,
                                            ) {
                                                return true;
                                            }
                                            continue;
                                        }
                                        if self.type_is_owned_string(param_type) {
                                            return true;
                                        }
                                        continue;
                                    }
                                }
                                return true;
                            }
                        }
                    }

                    if let Some(sig) = registry.get_signature(fn_name) {
                        // Extern fns: codegen wraps string args in
                        // string_to_ffi(.to_string()), so &str is always safe
                        if !sig.is_extern {
                            for (i, arg) in arguments.iter().enumerate() {
                                let arg_expr = &arg.1;
                                if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                                    if let Some(param_type) = sig.param_type_for_arg(i) {
                                        if self.type_is_string_ref_not_str(param_type) {
                                            return true;
                                        }
                                        if self.is_windjammer_string_param_type(param_type) {
                                            if self.callee_string_param_uses_rust_string_ref(
                                                sig, i, param_type, fn_name,
                                            ) {
                                                return true;
                                            }
                                            continue;
                                        }
                                        if self.type_is_owned_string(param_type) {
                                            return true;
                                        }
                                    }
                                }
                                if self.expr_uses_param_in_string_ref_context(
                                    param_name, arg_expr, registry,
                                ) {
                                    return true;
                                }
                            }
                        }
                    } else {
                        // Signature not in registry (extern fns, other Windjammer fns).
                        // Safe to use &str because:
                        // - Extern fns: codegen wraps string args in string_to_ffi(.to_string()),
                        //   which works with both &str and &String
                        // - Other Windjammer fns: their borrowed string params will also be &str
                        // - Known problematic stdlib methods (contains, push, insert) are
                        //   handled by special cases above
                        // Still recursively check sub-expressions for other patterns.
                        for arg in arguments.iter() {
                            let arg_expr = &arg.1;
                            if self.expr_uses_param_in_string_ref_context(
                                param_name, arg_expr, registry,
                            ) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            // Check binary operations (comparisons, string concatenation, etc.)
            Expression::Binary {
                left, right, op, ..
            } => {
                // SPECIAL CASE: String concatenation `a + b` consumes the LHS (a must be String, not &str)
                // If parameter is the LHS of +, it must be String (owned)
                if matches!(op, crate::parser::BinaryOp::Add) {
                    if let Expression::Identifier { name, .. } = &**left {
                        if name == param_name {
                            return true; // LHS of + must be String (owned), not &str
                        }
                    }
                }

                // Recursively check both sides
                self.expr_uses_param_in_string_ref_context(param_name, left, registry)
                    || self.expr_uses_param_in_string_ref_context(param_name, right, registry)
            }
            // Check unary operations
            Expression::Unary { operand, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, operand, registry)
            }
            // Check field access
            Expression::FieldAccess { object, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, object, registry)
            }
            // Check blocks
            Expression::Block { statements, .. } => {
                self.param_needs_string_ref(param_name, statements, registry)
            }
            // Struct literal: `User { name }` into a `string` field coerces at codegen — still &str at API.
            Expression::StructLiteral { name, fields, .. } => {
                for (field_name, field_value) in fields {
                    if self.expr_is_param_or_ref_to_param(param_name, field_value) {
                        if self.struct_field_is_text_type(name, field_name) {
                            continue;
                        }
                        return true;
                    }
                    // Recursively check the field value
                    if self.expr_uses_param_in_string_ref_context(param_name, field_value, registry)
                    {
                        return true;
                    }
                }
                false
            }
            // Check tuple expressions: (name, value) where tuple might be stored
            // This handles cases like relationships.push((npc, delta)) where npc must be owned String
            Expression::Tuple { elements, .. } => {
                for element in elements {
                    // Check if any element is our parameter
                    if self.expr_is_param_or_ref_to_param(param_name, element) {
                        // Conservative: If parameter is used in tuple, assume String (owned) is needed
                        // Tuples used in push/assign contexts require owned values
                        return true;
                    }
                    // Recursively check each element
                    if self.expr_uses_param_in_string_ref_context(param_name, element, registry) {
                        return true;
                    }
                }
                false
            }
            // Macros that consume their arguments (vec![], assert_eq![], etc.)
            // need owned values. Formatting macros (format!, println!, etc.)
            // only borrow, so &str is fine for those.
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
                    for arg in args {
                        if self.expr_uses_param_in_string_ref_context(param_name, arg, registry) {
                            return true;
                        }
                    }
                    false
                } else {
                    for arg in args {
                        if self.expr_is_param_or_ref_to_param(param_name, arg) {
                            return true;
                        }
                        if self.expr_uses_param_in_string_ref_context(param_name, arg, registry) {
                            return true;
                        }
                    }
                    false
                }
            }
            // Array literals [param, ...] also consume elements as owned values
            Expression::Array { elements, .. } => {
                for element in elements {
                    if self.expr_is_param_or_ref_to_param(param_name, element) {
                        return true;
                    }
                    if self.expr_uses_param_in_string_ref_context(param_name, element, registry) {
                        return true;
                    }
                }
                false
            }
            // Identifiers by themselves don't require &String (only when passed to methods)
            Expression::Identifier { .. } => false,
            // Other expressions
            _ => false,
        }
    }

    /// Check if an expression is the parameter or &parameter
    pub(crate) fn expr_is_param_or_ref_to_param(
        &self,
        param_name: &str,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == param_name,
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref,
                operand,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**operand {
                    name == param_name
                } else {
                    false
                }
            }
            // TDD FIX: Detect param.clone() and param.method() patterns
            // When a parameter is used in a struct literal like `Asset { name: name.clone() }`,
            // we need to detect that `name` is being used even though it's wrapped in .clone()
            Expression::MethodCall { object, .. } => {
                // Check if the method is being called on our parameter
                self.expr_is_param_or_ref_to_param(param_name, object)
            }
            _ => false,
        }
    }

    /// Windjammer Phase-2 `&str` parameter (Reference(Custom("str"))).
    fn is_phase2_str_ref_param_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
        )
    }

    /// Callee string param uses &String (Borrowed + not Phase-2 &str).
    fn callee_string_param_uses_rust_string_ref(
        &self,
        sig: &super::FunctionSignature,
        arg_idx: usize,
        param_type: &Type,
        method: &str,
    ) -> bool {
        if super::stdlib_method_traits::is_storage_method(method) {
            return false;
        }
        self.is_windjammer_string_param_type(param_type)
            && !self.is_phase2_str_ref_param_type(param_type)
            && sig
                .param_ownership_for_arg(arg_idx)
                .is_some_and(|o| matches!(o, super::OwnershipMode::Borrowed))
    }

    /// Check if a type is &String (not &str)
    /// This is the key distinction for the optimization
    pub(crate) fn type_is_string_ref_not_str(&self, ty: &Type) -> bool {
        match ty {
            Type::Reference(inner) => match &**inner {
                Type::String => true,
                Type::Custom(name) if name == "string" => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Windjammer `string` parameters (including registry stubs before &str lowering).
    pub(crate) fn is_windjammer_string_param_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::String)
            || matches!(ty, Type::Custom(name) if name == "string")
            || matches!(
                ty,
                Type::Reference(inner)
                    if matches!(&**inner, Type::Custom(s) if s == "str")
            )
    }

    /// Check if a type is owned String (not &str, not &String)
    /// Used to detect when parameters are passed to functions expecting owned String
    pub(crate) fn type_is_owned_string(&self, ty: &Type) -> bool {
        matches!(ty, Type::String) || matches!(ty, Type::Custom(name) if name == "string")
    }
}
