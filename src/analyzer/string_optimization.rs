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

                // If the function returns String and this param is returned directly,
                // it needs to be owned String (not &str)
                let returns_string = func
                    .return_type
                    .as_ref()
                    .map(|rt| {
                        matches!(rt, Type::String)
                            || matches!(rt, Type::Custom(ref n) if n == "string")
                    })
                    .unwrap_or(false);

                let param_is_returned = returns_string
                    && self.param_is_returned_directly(&param.name, &func.body);

                if param_is_returned {
                    continue;
                }

                let needs_string_ref =
                    self.param_needs_string_ref(&param.name, &func.body, registry);

                if !needs_string_ref {
                    optimizable.insert(param.name.clone());
                }
            }
        }

        optimizable
    }

    /// Check if a string parameter is returned directly (explicit return or implicit last expr)
    fn param_is_returned_directly(&self, param_name: &str, body: &[&Statement]) -> bool {
        for stmt in body {
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if let Expression::Identifier { name, .. } = &**expr {
                        if name == param_name {
                            return true;
                        }
                    }
                }
                Statement::Expression { expr, .. } => {
                    if let Expression::Identifier { name, .. } = &**expr {
                        if name == param_name {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a parameter needs &String (passed to method that requires it)
    /// Recursively traverses the function body to find all usages
    fn param_needs_string_ref(
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
    fn statement_uses_param_in_string_ref_context(
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
    fn block_needs_string_ref(
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
    fn expr_uses_param_in_string_ref_context(
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

                        // SPECIAL CASE: Vec<String>::contains needs &String
                        // This is a Rust stdlib method, not in our signature registry
                        if method == "contains" && idx == 0 {
                            // Check if the object is a Vec<String>
                            // For now, assume any contains() call on a collection needs &String
                            // This is a conservative but correct heuristic
                            return true;
                        }

                        // SPECIAL CASE: Vec<String>::push needs String (owned), not &str
                        // If parameter is passed to push(), it must be String
                        if method == "push" && idx == 0 {
                            return true; // Require String (owned), not &str
                        }

                        // SPECIAL CASE: HashMap::insert consumes both key and value
                        // Both args must be owned String, not &str
                        if method == "insert" && (idx == 0 || idx == 1) {
                            return true;
                        }

                        // Check if this method expects &String or String (owned) for this parameter position
                        if let Some(sig) = registry
                            .get_signature(method)
                            .or_else(|| registry.find_signature_ending_with(method))
                        {
                            let sig_idx = if sig.has_self_receiver { idx + 1 } else { idx };
                            if let Some(param_type) = sig.param_types.get(sig_idx) {
                                if self.type_is_string_ref_not_str(param_type) {
                                    return true;
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

                // Also check if the method is called ON the parameter (param.method())
                if let Expression::Identifier { name, .. } = &**object {
                    if name == param_name {
                        // param.method() - check if method needs &String receiver
                        if let Some(sig) = registry
                            .get_signature(method)
                            .or_else(|| registry.find_signature_ending_with(method))
                        {
                            // Check all parameter types in the signature
                            for param_type in &sig.param_types {
                                if self.type_is_string_ref_not_str(param_type) {
                                    return true;
                                }
                            }
                        }
                    }
                }

                false
            }
            // Check function calls: function(&param)
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    // Enum variants (Some, None, Ok, Err, MyEnum::Variant) consume
                    // their arguments. Detect enum variants vs module-qualified fn
                    // calls: enum variants have an uppercase final component.
                    let is_enum_variant = matches!(fn_name.as_str(), "Some" | "None" | "Ok" | "Err")
                        || (fn_name.contains("::") && {
                            let last = fn_name.rsplit("::").next().unwrap_or("");
                            last.starts_with(|c: char| c.is_uppercase())
                        });

                    // Enum variants and constructors (Type::new, Type::from_*) consume
                    // their arguments, so string params passed to them need owned String.
                    let is_constructor = fn_name.contains("::") && {
                        let last = fn_name.rsplit("::").next().unwrap_or("");
                        last.starts_with("new") || last.starts_with("from_")
                    };

                    if is_enum_variant || is_constructor {
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
                                    let sig_idx =
                                        if sig.has_self_receiver { i + 1 } else { i };
                                    if let Some(param_type) = sig.param_types.get(sig_idx) {
                                        if self.type_is_string_ref_not_str(param_type) {
                                            return true;
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
            // Check struct literals: Item { name: name } where name is a String field
            Expression::StructLiteral { fields, .. } => {
                for (_field_name, field_value) in fields {
                    // Check if this field value is our parameter
                    if self.expr_is_param_or_ref_to_param(param_name, field_value) {
                        // Conservative: If parameter is assigned to any field, assume String (owned) is needed
                        // This prevents &str → String assignment errors
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
            // Identifiers by themselves don't require &String (only when passed to methods)
            Expression::Identifier { .. } => false,
            // Other expressions
            _ => false,
        }
    }

    /// Check if an expression is the parameter or &parameter
    fn expr_is_param_or_ref_to_param(&self, param_name: &str, expr: &Expression) -> bool {
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

    /// Check if a type is &String (not &str)
    /// This is the key distinction for the optimization
    fn type_is_string_ref_not_str(&self, ty: &Type) -> bool {
        match ty {
            Type::Reference(inner) => match &**inner {
                Type::String => true,
                Type::Custom(name) if name == "string" => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Check if a type is owned String (not &str, not &String)
    /// Used to detect when parameters are passed to functions expecting owned String
    fn type_is_owned_string(&self, ty: &Type) -> bool {
        matches!(ty, Type::String) || matches!(ty, Type::Custom(name) if name == "string")
    }
}
