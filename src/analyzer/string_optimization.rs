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

                // Module helpers: skip &str optimization when the param is returned,
                // forwarded as owned, or is a HashMap key helper (owned String API).
                if func.parent_type.is_none()
                    && !needs_string_ref
                    && self.module_param_blocks_str_ref_optimization(
                        &param.name,
                        &func.body,
                        registry,
                        func,
                    )
                {
                    continue;
                }

                // Runtime AsRef<&str> modules (`db.connect`, `env::var`, …): keep owned
                // WJ `string` + borrow at the call site — do not demote to `&str`.
                if !needs_string_ref
                    && self.param_only_forwarded_to_asref_str_runtime_modules(
                        &param.name,
                        &func.body,
                    )
                {
                    continue;
                }

                // Qualified HashMap get-key-only params: prefer &str (Borrow) rather than
                // forcing owned String — avoids `&&str` when call sites already hold refs.
                if self.param_only_used_as_qualified_map_get_key(&param.name, &func.body, func) {
                    if !needs_string_ref && !self.is_stored(&param.name, &func.body, registry) {
                        optimizable.insert(param.name.clone());
                    }
                    continue;
                }

                if self.is_stored(&param.name, &func.body, registry) {
                    continue;
                }

                // LHS of string `+` consumes the param (owned String), not &str.
                if self.param_is_string_concat_lhs(&param.name, &func.body) {
                    continue;
                }

                if !needs_string_ref {
                    optimizable.insert(param.name.clone());
                }
            }
        }

        optimizable
    }

    /// True when `param_name` appears as the LHS of a `+` binary expression
    /// (string concatenation consumes the LHS — needs owned String, not &str).
    pub(crate) fn param_is_string_concat_lhs(
        &self,
        param_name: &str,
        body: &[&Statement],
    ) -> bool {
        body.iter().any(|stmt| self.stmt_has_param_as_concat_lhs(param_name, stmt))
    }

    fn stmt_has_param_as_concat_lhs(&self, param_name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expr_has_param_as_concat_lhs(param_name, expr)
            }
            Statement::Let { value, .. } => self.expr_has_param_as_concat_lhs(param_name, value),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr_has_param_as_concat_lhs(param_name, condition)
                    || then_block.iter().any(|s| self.stmt_has_param_as_concat_lhs(param_name, s))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| self.stmt_has_param_as_concat_lhs(param_name, s))
                    })
            }
            _ => false,
        }
    }

    fn expr_has_param_as_concat_lhs(&self, param_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary { left, right, op, .. } => {
                if matches!(op, crate::parser::BinaryOp::Add) {
                    if let Expression::Identifier { name, .. } = &**left {
                        if name == param_name {
                            return true;
                        }
                    }
                }
                self.expr_has_param_as_concat_lhs(param_name, left)
                    || self.expr_has_param_as_concat_lhs(param_name, right)
            }
            _ => false,
        }
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

                        if super::stdlib_method_traits::is_storage_method(method)
                            && registry.lookup_method(method).is_some_and(|sig| {
                                sig.param_type_for_arg(idx).is_some_and(|t| {
                                    self.is_windjammer_string_param_type(t)
                                        || self.type_is_owned_string(t)
                                })
                            })
                        {
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
                // Type::method(...) calls (parsed as Call(FieldAccess)) — signature-driven.
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    let qualified = if let Expression::Identifier { name, .. } = &**object {
                        Some(format!("{}::{}", name, field))
                    } else {
                        None
                    };
                    if let Some(ref qname) = qualified {
                        if let Some(sig) = registry.get_signature(qname) {
                            for (i, arg) in arguments.iter().enumerate() {
                                let arg_expr = &arg.1;
                                if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
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
                                    }
                                }
                                if self.expr_uses_param_in_string_ref_context(
                                    param_name, arg_expr, registry,
                                ) {
                                    return true;
                                }
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

                    if is_enum_variant {
                        for arg in arguments.iter() {
                            let arg_expr = &arg.1;
                            if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
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
                // NOTE: LHS of string `+` needs owned String, not &String.
                // That is NOT a "&String context" — it is an owned context.
                // Handled by `param_is_string_concat_lhs` which blocks
                // str_ref optimization AND keeps ownership Owned.
                let _ = op;

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

    /// Module-level params that must stay owned `String` (not &str) block str_ref optimization.
    fn module_param_blocks_str_ref_optimization(
        &self,
        param_name: &str,
        body: &[&Statement],
        registry: &super::SignatureRegistry,
        func: &FunctionDecl,
    ) -> bool {
        for stmt in body {
            if self.statement_forwards_param_as_owned_pass_through(param_name, stmt, registry, func)
            {
                return true;
            }
        }
        false
    }

    /// `db.connect(url)` / `conn.query(sql, …)` — runtime AsRef<&str> APIs. Keep owned
    /// WJ `string` formals so call sites emit `&url` (std_db_call_site_borrow_test).
    fn param_only_forwarded_to_asref_str_runtime_modules(
        &self,
        param_name: &str,
        body: &[&Statement],
    ) -> bool {
        let mut saw = false;
        for stmt in body {
            if !self.statement_asref_runtime_forward(param_name, stmt, &mut saw) {
                return false;
            }
        }
        saw
    }

    fn statement_asref_runtime_forward(
        &self,
        param_name: &str,
        stmt: &Statement,
        saw: &mut bool,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expr_asref_runtime_forward(param_name, expr, saw),
            Statement::Let { value, else_block, .. } => {
                self.expr_asref_runtime_forward(param_name, value, saw)
                    && else_block.as_ref().map_or(true, |b| {
                        b.iter()
                            .all(|s| self.statement_asref_runtime_forward(param_name, s, saw))
                    })
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_asref_runtime_forward(param_name, condition, saw)
                    && then_block
                        .iter()
                        .all(|s| self.statement_asref_runtime_forward(param_name, s, saw))
                    && else_block.as_ref().map_or(true, |b| {
                        b.iter()
                            .all(|s| self.statement_asref_runtime_forward(param_name, s, saw))
                    })
            }
            Statement::Match { value, arms, .. } => {
                self.expr_asref_runtime_forward(param_name, value, saw)
                    && arms
                        .iter()
                        .all(|arm| self.expr_asref_runtime_forward(param_name, &arm.body, saw))
            }
            Statement::While { body, condition, .. } => {
                self.expr_asref_runtime_forward(param_name, condition, saw)
                    && body
                        .iter()
                        .all(|s| self.statement_asref_runtime_forward(param_name, s, saw))
            }
            Statement::For { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .all(|s| self.statement_asref_runtime_forward(param_name, s, saw)),
            _ => true,
        }
    }

    fn expr_asref_runtime_forward(
        &self,
        param_name: &str,
        expr: &Expression,
        saw: &mut bool,
    ) -> bool {
        let is_asref_module = |name: &str| {
            matches!(
                name,
                "db" | "env" | "strings" | "json" | "jwt" | "regex" | "csv" | "mime" | "http"
            ) || name.ends_with("::db")
                || name.contains("db::")
        };
        let arg_is_param = |arg: &Expression| {
            matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                || matches!(
                    arg,
                    Expression::Unary {
                        op: crate::parser::UnaryOp::Ref,
                        operand,
                        ..
                    } if matches!(&**operand, Expression::Identifier { name, .. } if name == param_name)
                )
        };
        match expr {
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                for (_, arg) in arguments {
                    if arg_is_param(arg) {
                        *saw = true;
                        let recv_ok = match &**object {
                            Expression::Identifier { name, .. } => {
                                is_asref_module(name) || name == "conn"
                            }
                            _ => {
                                // `conn.query(sql, …)` — Connection methods take &str.
                                true
                            }
                        };
                        if !recv_ok {
                            return false;
                        }
                    } else if !self.expr_asref_runtime_forward(param_name, arg, saw) {
                        return false;
                    }
                }
                self.expr_asref_runtime_forward(param_name, object, saw)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                for (_, arg) in arguments {
                    if arg_is_param(arg) {
                        *saw = true;
                        let ok = match &**function {
                            Expression::FieldAccess { object, .. } => matches!(
                                &**object,
                                Expression::Identifier { name, .. }
                                    if is_asref_module(name) || name == "conn"
                            ),
                            Expression::Identifier { name, .. } => {
                                is_asref_module(name) || name.contains("db::")
                            }
                            _ => false,
                        };
                        if !ok {
                            return false;
                        }
                    } else if !self.expr_asref_runtime_forward(param_name, arg, saw) {
                        return false;
                    }
                }
                self.expr_asref_runtime_forward(param_name, function, saw)
            }
            Expression::Identifier { name, .. } if name == param_name => false,
            Expression::Binary { left, right, .. } => {
                self.expr_asref_runtime_forward(param_name, left, saw)
                    && self.expr_asref_runtime_forward(param_name, right, saw)
            }
            Expression::Unary { operand, .. }
            | Expression::FieldAccess { object: operand, .. }
            | Expression::TryOp { expr: operand, .. }
            | Expression::Await { expr: operand, .. }
            | Expression::Cast { expr: operand, .. } => {
                self.expr_asref_runtime_forward(param_name, operand, saw)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .all(|s| self.statement_asref_runtime_forward(param_name, s, saw)),
            Expression::StructLiteral { fields, .. } => {
                // Constructing owned values from the param is not an AsRef forward.
                !fields
                    .iter()
                    .any(|(_, e)| self.expr_mentions_param(param_name, e))
            }
            Expression::MacroInvocation { args, .. } => {
                // `vec![tenant]` needs owned String — not an AsRef-only forward.
                !args
                    .iter()
                    .any(|a| self.expr_mentions_param(param_name, a))
            }
            _ => true,
        }
    }

    fn expr_mentions_param(&self, param_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == param_name,
            Expression::Unary { operand, .. }
            | Expression::FieldAccess { object: operand, .. }
            | Expression::TryOp { expr: operand, .. }
            | Expression::Await { expr: operand, .. }
            | Expression::Cast { expr: operand, .. } => {
                self.expr_mentions_param(param_name, operand)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_param(param_name, left)
                    || self.expr_mentions_param(param_name, right)
            }
            Expression::Call { function, arguments, .. } => {
                self.expr_mentions_param(param_name, function)
                    || arguments
                        .iter()
                        .any(|(_, a)| self.expr_mentions_param(param_name, a))
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_mentions_param(param_name, object)
                    || arguments
                        .iter()
                        .any(|(_, a)| self.expr_mentions_param(param_name, a))
            }
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|a| self.expr_mentions_param(param_name, a)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, e)| self.expr_mentions_param(param_name, e)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|e| self.expr_mentions_param(param_name, e)),
            _ => false,
        }
    }

    fn statement_forwards_param_as_owned_pass_through(
        &self,
        param_name: &str,
        stmt: &Statement,
        registry: &super::SignatureRegistry,
        func: &FunctionDecl,
    ) -> bool {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_is_param_or_ref_to_param(param_name, expr),
            Statement::Expression { expr, .. } => {
                self.expr_forwards_param_as_owned_pass_through(param_name, expr, registry, func)
            }
            Statement::Let { value, .. } => {
                self.expr_forwards_param_as_owned_pass_through(param_name, value, registry, func)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| {
                    self.statement_forwards_param_as_owned_pass_through(
                        param_name, s, registry, func,
                    )
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.statement_forwards_param_as_owned_pass_through(
                            param_name, s, registry, func,
                        )
                    })
                })
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                self.expr_forwards_param_as_owned_pass_through(
                    param_name, arm.body, registry, func,
                )
            }),
            _ => false,
        }
    }

    fn expr_forwards_param_as_owned_pass_through(
        &self,
        param_name: &str,
        expr: &Expression,
        registry: &super::SignatureRegistry,
        func: &FunctionDecl,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if !self.expr_is_param_or_ref_to_param(param_name, arg) {
                        continue;
                    }
                    let method_key =
                        self.qualified_method_registry_key(object, method, func);
                    let sig = registry
                        .lookup_method(&method_key)
                        .or_else(|| registry.get_signature(&method_key));
                    if let Some(sig) = sig {
                        let pidx = if sig.has_self_receiver { i + 1 } else { i };
                        if matches!(
                            sig.param_ownership.get(pidx),
                            Some(super::OwnershipMode::Owned)
                        ) {
                            return true;
                        }
                    }
                }
                false
            }
            Expression::Call { function, arguments, .. } => arguments.iter().any(|(_, arg)| {
                if !self.expr_is_param_or_ref_to_param(param_name, arg) {
                    return false;
                }
                if self.call_expr_is_string_runtime(function) {
                    return false;
                }
                if let Expression::Identifier { name, .. } = &**function {
                    if matches!(name.as_str(), "println" | "print" | "eprintln" | "eprint") {
                        return false;
                    }
                    if let Some(sig) = registry.get_signature(name) {
                        return sig.param_ownership.iter().any(|o| {
                            matches!(o, super::OwnershipMode::Owned)
                        });
                    }
                }
                false
            }),
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.statement_forwards_param_as_owned_pass_through(param_name, s, registry, func)
            }),
            _ => false,
        }
    }

    /// Module-level plain `string` params stay owned when only forwarded to
    /// non-string-runtime callees (collections, domain helpers, etc.).
    #[allow(dead_code)]
    fn module_param_keeps_owned_string_api(
        &self,
        param_name: &str,
        body: &[&Statement],
        _registry: &super::SignatureRegistry,
    ) -> bool {
        for stmt in body {
            if self.statement_uses_param_in_string_runtime_module(param_name, stmt) {
                return false;
            }
        }
        true
    }

    fn block_uses_param_in_string_runtime_module(
        &self,
        param_name: &str,
        block: &[&Statement],
    ) -> bool {
        block
            .iter()
            .any(|s| self.statement_uses_param_in_string_runtime_module(param_name, s))
    }

    fn statement_uses_param_in_string_runtime_module(
        &self,
        param_name: &str,
        stmt: &Statement,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_uses_param_in_string_runtime_module(param_name, expr)
            }
            Statement::Let { value, .. } => {
                self.expr_uses_param_in_string_runtime_module(param_name, value)
            }
            Statement::Assignment { target, value, .. } => {
                self.expr_uses_param_in_string_runtime_module(param_name, target)
                    || self.expr_uses_param_in_string_runtime_module(param_name, value)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_uses_param_in_string_runtime_module(param_name, condition)
                    || self.block_uses_param_in_string_runtime_module(param_name, then_block)
                    || else_block.as_ref().is_some_and(|b| {
                        self.block_uses_param_in_string_runtime_module(param_name, b)
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_uses_param_in_string_runtime_module(param_name, condition)
                    || self.block_uses_param_in_string_runtime_module(param_name, body)
            }
            Statement::For { body, .. } => {
                self.block_uses_param_in_string_runtime_module(param_name, body)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_uses_param_in_string_runtime_module(param_name, expr),
            Statement::Match { value, arms, .. } => {
                self.expr_uses_param_in_string_runtime_module(param_name, value)
                    || arms.iter().any(|arm| {
                        self.expr_uses_param_in_string_runtime_module(param_name, arm.body)
                    })
            }
            _ => false,
        }
    }

    fn expr_uses_param_in_string_runtime_module(
        &self,
        param_name: &str,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::Call { function, arguments, .. } => {
                let is_strings = self.call_expr_is_string_runtime(function);
                for (_, arg) in arguments {
                    if self.expr_is_param_or_ref_to_param(param_name, arg) && is_strings {
                        return true;
                    }
                    if self.expr_uses_param_in_string_runtime_module(param_name, arg) {
                        return true;
                    }
                }
                false
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if self.expr_is_param_or_ref_to_param(param_name, object)
                    && self.method_name_is_string_runtime(method)
                {
                    return true;
                }
                for (_, arg) in arguments {
                    if self.expr_is_param_or_ref_to_param(param_name, arg)
                        && self.method_name_is_string_runtime(method)
                    {
                        return true;
                    }
                    if self.expr_uses_param_in_string_runtime_module(param_name, arg) {
                        return true;
                    }
                }
                false
            }
            Expression::Binary { left, right, .. } => {
                self.expr_uses_param_in_string_runtime_module(param_name, left)
                    || self.expr_uses_param_in_string_runtime_module(param_name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expr_uses_param_in_string_runtime_module(param_name, operand)
            }
            Expression::Block { statements, .. } => self
                .block_uses_param_in_string_runtime_module(param_name, statements),
            _ => false,
        }
    }

    fn call_expr_is_string_runtime(&self, function: &Expression) -> bool {
        match function {
            Expression::Identifier { name, .. } => Self::qualified_name_is_string_runtime(name),
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    Self::qualified_name_is_string_runtime(&format!("{name}.{field}"))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn method_name_is_string_runtime(&self, method: &str) -> bool {
        matches!(
            method,
            "starts_with"
                | "ends_with"
                | "contains"
                | "substring"
                | "len"
                | "trim"
                | "to_lowercase"
                | "to_uppercase"
                | "split"
                | "replace"
        )
    }

    fn qualified_name_is_string_runtime(name: &str) -> bool {
        name.starts_with("strings::")
            || name.starts_with("std::strings::")
            || name.split("::").next() == Some("strings")
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

    /// Params whose only direct use is a map/set key lookup keep owned `String` formals.
    fn param_only_used_as_map_get_key(
        &self,
        param_name: &str,
        body: &[&Statement],
        func: &FunctionDecl,
    ) -> bool {
        let mut key_uses = 0usize;
        let mut other_uses = 0usize;
        self.collect_map_get_key_uses(body, param_name, func, &mut key_uses, &mut other_uses);
        key_uses > 0 && other_uses == 0
    }

    /// FMP/wdb route helpers: owned `String` keys for qualified `std::collections::HashMap` lookups.
    fn param_only_used_as_qualified_map_get_key(
        &self,
        param_name: &str,
        body: &[&Statement],
        func: &FunctionDecl,
    ) -> bool {
        let mut key_uses = 0usize;
        let mut other_uses = 0usize;
        self.collect_qualified_map_get_key_uses(body, param_name, func, &mut key_uses, &mut other_uses);
        key_uses > 0 && other_uses == 0
    }

    fn collect_qualified_map_get_key_uses(
        &self,
        body: &[&Statement],
        param_name: &str,
        func: &FunctionDecl,
        key_uses: &mut usize,
        other_uses: &mut usize,
    ) {
        for stmt in body {
            self.statement_collect_qualified_map_get_key_uses(
                stmt, param_name, func, key_uses, other_uses,
            );
        }
    }

    fn statement_collect_qualified_map_get_key_uses(
        &self,
        stmt: &Statement,
        param_name: &str,
        func: &FunctionDecl,
        key_uses: &mut usize,
        other_uses: &mut usize,
    ) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    expr, param_name, func, key_uses, other_uses,
                );
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    value, param_name, func, key_uses, other_uses,
                );
                if let Some(b) = else_block {
                    self.collect_qualified_map_get_key_uses(b, param_name, func, key_uses, other_uses);
                }
            }
            Statement::Return { value: Some(expr), .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    expr, param_name, func, key_uses, other_uses,
                );
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_collect_qualified_map_get_key_uses(
                    condition, param_name, func, key_uses, other_uses,
                );
                self.collect_qualified_map_get_key_uses(then_block, param_name, func, key_uses, other_uses);
                if let Some(b) = else_block {
                    self.collect_qualified_map_get_key_uses(b, param_name, func, key_uses, other_uses);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    value, param_name, func, key_uses, other_uses,
                );
                for arm in arms {
                    self.expression_collect_qualified_map_get_key_uses(
                        &arm.body, param_name, func, key_uses, other_uses,
                    );
                }
            }
            Statement::While { body, condition, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    condition, param_name, func, key_uses, other_uses,
                );
                self.collect_qualified_map_get_key_uses(body, param_name, func, key_uses, other_uses);
            }
            Statement::For { body, iterable, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    iterable, param_name, func, key_uses, other_uses,
                );
                self.collect_qualified_map_get_key_uses(body, param_name, func, key_uses, other_uses);
            }
            _ => {}
        }
    }

    fn expression_collect_qualified_map_get_key_uses(
        &self,
        expr: &Expression,
        param_name: &str,
        func: &FunctionDecl,
        key_uses: &mut usize,
        other_uses: &mut usize,
    ) {
        if let Some((object, _method, arguments)) =
            super::stdlib_method_traits::decompose_collection_key_lookup(expr)
        {
            let qualified_receiver = self
                .receiver_param_type_from_expr(object, func)
                .is_some_and(super::stdlib_method_traits::is_qualified_map_type);
            for (i, (_, arg)) in arguments.iter().enumerate() {
                if self.expr_is_identifier(*arg, param_name) {
                    if i == 0 && qualified_receiver {
                        *key_uses += 1;
                    } else {
                        *other_uses += 1;
                    }
                }
            }
            self.expression_collect_qualified_map_get_key_uses(
                object, param_name, func, key_uses, other_uses,
            );
            for (_, arg) in arguments {
                self.expression_collect_qualified_map_get_key_uses(*arg, param_name, func, key_uses, other_uses);
            }
            return;
        }
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let qualified_receiver = self
                    .receiver_param_type_from_expr(object, func)
                    .is_some_and(super::stdlib_method_traits::is_qualified_map_type);
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(*arg, param_name) {
                        if super::stdlib_method_traits::is_map_key_method(method)
                            && i == 0
                            && qualified_receiver
                        {
                            *key_uses += 1;
                        } else {
                            *other_uses += 1;
                        }
                    }
                }
                self.expression_collect_qualified_map_get_key_uses(
                    object, param_name, func, key_uses, other_uses,
                );
                for (_, arg) in arguments {
                    self.expression_collect_qualified_map_get_key_uses(*arg, param_name, func, key_uses, other_uses);
                }
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    function, param_name, func, key_uses, other_uses,
                );
                for (_, arg) in arguments {
                    self.expression_collect_qualified_map_get_key_uses(*arg, param_name, func, key_uses, other_uses);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    left, param_name, func, key_uses, other_uses,
                );
                self.expression_collect_qualified_map_get_key_uses(
                    right, param_name, func, key_uses, other_uses,
                );
            }
            Expression::Unary { operand, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    operand, param_name, func, key_uses, other_uses,
                );
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    object, param_name, func, key_uses, other_uses,
                );
            }
            Expression::Index { object, index, .. } => {
                self.expression_collect_qualified_map_get_key_uses(
                    object, param_name, func, key_uses, other_uses,
                );
                self.expression_collect_qualified_map_get_key_uses(
                    index, param_name, func, key_uses, other_uses,
                );
            }
            Expression::Block { statements, .. } => {
                self.collect_qualified_map_get_key_uses(statements, param_name, func, key_uses, other_uses);
            }
            _ => {}
        }
    }

    fn receiver_param_type_from_expr<'a>(
        &self,
        object: &'a Expression,
        func: &'a FunctionDecl,
    ) -> Option<&'a Type> {
        if let Expression::Identifier { name, .. } = object {
            func.parameters.iter().find(|p| p.name == *name).map(|p| &p.type_)
        } else {
            None
        }
    }

    fn collect_map_get_key_uses(
        &self,
        body: &[&Statement],
        param_name: &str,
        func: &FunctionDecl,
        key_uses: &mut usize,
        other_uses: &mut usize,
    ) {
        for stmt in body {
            self.statement_collect_map_get_key_uses(
                stmt, param_name, func, key_uses, other_uses,
            );
        }
    }

    fn statement_collect_map_get_key_uses(
        &self,
        stmt: &Statement,
        param_name: &str,
        func: &FunctionDecl,
        key_uses: &mut usize,
        other_uses: &mut usize,
    ) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_collect_map_get_key_uses(
                    expr, param_name, func, key_uses, other_uses,
                );
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_collect_map_get_key_uses(
                    value, param_name, func, key_uses, other_uses,
                );
                if let Some(b) = else_block {
                    self.collect_map_get_key_uses(b, param_name, func, key_uses, other_uses);
                }
            }
            Statement::Return { value: Some(expr), .. } => {
                self.expression_collect_map_get_key_uses(
                    expr, param_name, func, key_uses, other_uses,
                );
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_collect_map_get_key_uses(
                    condition, param_name, func, key_uses, other_uses,
                );
                self.collect_map_get_key_uses(then_block, param_name, func, key_uses, other_uses);
                if let Some(b) = else_block {
                    self.collect_map_get_key_uses(b, param_name, func, key_uses, other_uses);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expression_collect_map_get_key_uses(
                    value, param_name, func, key_uses, other_uses,
                );
                for arm in arms {
                    self.expression_collect_map_get_key_uses(
                        &arm.body, param_name, func, key_uses, other_uses,
                    );
                }
            }
            Statement::While { body, condition, .. } => {
                self.expression_collect_map_get_key_uses(
                    condition, param_name, func, key_uses, other_uses,
                );
                self.collect_map_get_key_uses(body, param_name, func, key_uses, other_uses);
            }
            Statement::For { body, iterable, .. } => {
                self.expression_collect_map_get_key_uses(
                    iterable, param_name, func, key_uses, other_uses,
                );
                self.collect_map_get_key_uses(body, param_name, func, key_uses, other_uses);
            }
            _ => {}
        }
    }

    fn expression_collect_map_get_key_uses(
        &self,
        expr: &Expression,
        param_name: &str,
        func: &FunctionDecl,
        key_uses: &mut usize,
        other_uses: &mut usize,
    ) {
        if let Some((object, _method, arguments)) =
            super::stdlib_method_traits::decompose_collection_key_lookup(expr)
        {
            for (i, (_, arg)) in arguments.iter().enumerate() {
                if self.expr_is_identifier(*arg, param_name) {
                    let receiver = self.receiver_type_name_from_expr(object, func);
                    if i == 0
                        && super::stdlib_method_traits::is_map_receiver(receiver.as_deref())
                    {
                        *key_uses += 1;
                    } else {
                        *other_uses += 1;
                    }
                }
            }
            self.expression_collect_map_get_key_uses(
                object, param_name, func, key_uses, other_uses,
            );
            for (_, arg) in arguments {
                self.expression_collect_map_get_key_uses(*arg, param_name, func, key_uses, other_uses);
            }
            return;
        }
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(*arg, param_name) {
                        let receiver = self.receiver_type_name_from_expr(object, func);
                        if super::stdlib_method_traits::is_map_key_method(method)
                            && i == 0
                            && super::stdlib_method_traits::is_map_receiver(receiver.as_deref())
                        {
                            *key_uses += 1;
                        } else {
                            *other_uses += 1;
                        }
                    }
                }
                self.expression_collect_map_get_key_uses(
                    object, param_name, func, key_uses, other_uses,
                );
                for (_, arg) in arguments {
                    self.expression_collect_map_get_key_uses(
                        arg, param_name, func, key_uses, other_uses,
                    );
                }
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_collect_map_get_key_uses(
                    function, param_name, func, key_uses, other_uses,
                );
                for (_, arg) in arguments {
                    self.expression_collect_map_get_key_uses(
                        arg, param_name, func, key_uses, other_uses,
                    );
                }
            }
            Expression::Binary { left, right, .. } => {
                self.expression_collect_map_get_key_uses(
                    left, param_name, func, key_uses, other_uses,
                );
                self.expression_collect_map_get_key_uses(
                    right, param_name, func, key_uses, other_uses,
                );
            }
            Expression::Unary { operand, .. } => {
                self.expression_collect_map_get_key_uses(
                    operand, param_name, func, key_uses, other_uses,
                );
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_collect_map_get_key_uses(
                    object, param_name, func, key_uses, other_uses,
                );
            }
            Expression::Index { object, index, .. } => {
                self.expression_collect_map_get_key_uses(
                    object, param_name, func, key_uses, other_uses,
                );
                self.expression_collect_map_get_key_uses(
                    index, param_name, func, key_uses, other_uses,
                );
            }
            Expression::Block { statements, .. } => {
                self.collect_map_get_key_uses(statements, param_name, func, key_uses, other_uses);
            }
            _ => {}
        }
    }

    fn receiver_type_name_from_expr(
        &self,
        object: &Expression,
        func: &FunctionDecl,
    ) -> Option<String> {
        if let Expression::Identifier { name, .. } = object {
            func.parameters.iter().find(|p| p.name == *name).and_then(|p| {
                match &p.type_ {
                    Type::Custom(n) => Some(n.clone()),
                    Type::Parameterized(base, _) => Some(base.clone()),
                    Type::Reference(inner) | Type::MutableReference(inner) => {
                        match inner.as_ref() {
                            Type::Custom(n) => Some(n.clone()),
                            Type::Parameterized(base, _) => Some(base.clone()),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            })
        } else {
            None
        }
    }
}
