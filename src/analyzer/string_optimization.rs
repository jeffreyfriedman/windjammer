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
        let mut optimizable = HashSet::new();

        // PHASE 3: Check for manual override decorators first
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
                // PHASE 2 FULL: Check if parameter is passed to methods needing &String
                let needs_string_ref = self.param_needs_string_ref(&param.name, &func.body, registry);
                
                if !needs_string_ref {
                    // Safe to use &str optimization
                    optimizable.insert(param.name.clone());
                }
            }
        }

        optimizable
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

    /// Check if a statement uses the parameter in a context requiring &String
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
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, condition, registry)
                    || self.block_needs_string_ref(param_name, then_block, registry)
                    || else_block.as_ref().map(|b| self.block_needs_string_ref(param_name, b, registry)).unwrap_or(false)
            }
            Statement::While { condition, body, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, condition, registry)
                    || self.block_needs_string_ref(param_name, body, registry)
            }
            Statement::For { body, .. } => {
                self.block_needs_string_ref(param_name, body, registry)
            }
            Statement::Return { value: Some(expr), .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, expr, registry)
            }
            Statement::Match { value, arms, .. } => {
                self.expr_uses_param_in_string_ref_context(param_name, value, registry)
                    || arms.iter().any(|arm| self.expr_uses_param_in_string_ref_context(param_name, &arm.body, registry))
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
            Expression::MethodCall { object, method, arguments, .. } => {
                // First check if any argument is our parameter (like items.contains(&id))
                for (idx, arg) in arguments.iter().enumerate() {
                    let arg_expr = &arg.1;
                    
                    // Check if this argument is &param or param
                    if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                        // SPECIAL CASE: Vec<String>::contains needs &String
                        // This is a Rust stdlib method, not in our signature registry
                        if method == "contains" && idx == 0 {
                            // Check if the object is a Vec<String>
                            // For now, assume any contains() call on a collection needs &String
                            // This is a conservative but correct heuristic
                            return true;
                        }
                        
                        // Check if this method expects &String for this parameter position
                        if let Some(sig) = registry.get_signature(method) {
                            // Get the parameter type at this position
                            // Note: idx is the argument index, which corresponds to parameter index
                            // (assuming no self receiver in the signature - methods store self separately)
                            if let Some(param_type) = sig.param_types.get(idx) {
                                if self.type_is_string_ref_not_str(param_type) {
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
                        if let Some(sig) = registry.get_signature(method) {
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
            Expression::Call { function, arguments, .. } => {
                // Check if param is passed to a function expecting &String
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    if let Some(sig) = registry.get_signature(fn_name) {
                        for (i, arg) in arguments.iter().enumerate() {
                            let arg_expr = &arg.1;
                            // Check if this argument is our parameter
                            if self.expr_is_param_or_ref_to_param(param_name, arg_expr) {
                                // Check if the corresponding parameter type in the signature is &String
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if self.type_is_string_ref_not_str(param_type) {
                                        return true;
                                    }
                                }
                            }
                            // Recursively check
                            if self.expr_uses_param_in_string_ref_context(param_name, arg_expr, registry) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            // Check binary operations (comparisons, etc.)
            Expression::Binary { left, right, .. } => {
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
            Expression::Unary { op: crate::parser::UnaryOp::Ref, operand, .. } => {
                if let Expression::Identifier { name, .. } = &**operand {
                    name == param_name
                } else {
                    false
                }
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
}
