//! Mutation detection methods for the analyzer.
//! Determines whether parameters or local variables are mutated,
//! enabling automatic &mut inference and mut binding inference.

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// THE WINDJAMMER WAY: Check if an expression contains a specific identifier
    /// Used to detect if a parameter is used in a method call chain (e.g., self.camera.move_to())
    /// 
    /// CRITICAL: For Index expressions, only check the object, NOT the index!
    /// When we see `arr[i].method()`, only `arr` is being used mutably, NOT `i`.
    /// The index `i` is just being READ to select which element to call the method on.
    fn expr_contains_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expr_contains_identifier(name, object),
            // THE FIX: Don't check the index part - it's only read, never mutated!
            // Before: self.expr_contains_identifier(name, object) || self.expr_contains_identifier(name, index)
            // After: Only check object
            Expression::Index { object, index: _, location: _ } => {
                self.expr_contains_identifier(name, object)
            }
            Expression::MethodCall { object, arguments, .. } => {
                if self.expr_contains_identifier(name, object) {
                    return true;
                }
                for (_label, arg) in arguments {
                    if self.expr_contains_identifier(name, arg) {
                        return true;
                    }
                }
                false
            }
            Expression::Call { arguments, .. } => {
                for (_label, arg) in arguments {
                    if self.expr_contains_identifier(name, arg) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn is_mutated(&self, name: &str, statements: &[&'ast Statement<'ast>], registry: &SignatureRegistry) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment { target, .. } => {
                    if let Expression::Identifier { name: id, .. } = target {
                        if id == name {
                            return true;
                        }
                    }

                    // THE WINDJAMMER WAY: Check if the assignment target is a field of the parameter
                    // e.g., p.x = ... or p.position.x = ...
                    // But NOT if the parameter is just used in an index expression!
                    // e.g., arr[entity.index] = x  <- entity is READ, not mutated
                    if self.is_direct_mutation_target(name, target) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.has_mutable_method_call(name, expr, registry) {
                        return true;
                    }
                }
                Statement::Let { value, .. } => {
                    if self.has_mutable_method_call(name, value, registry) {
                        return true;
                    }
                }
                Statement::Return { value: Some(expr), .. } => {
                    if self.has_mutable_method_call(name, expr, registry) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_mutated(name, then_block, registry) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_mutated(name, else_b, registry) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. }
                | Statement::While { body, .. }
                | Statement::For { body, .. } => {
                    if self.is_mutated(name, body, registry) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a parameter is the DIRECT target of mutation
    /// Returns true for: p = x, p.field = x, p.field.nested = x
    /// Returns false for: arr[p.index] = x, obj[p] = x  (p is only READ here)
    /// 
    /// THE WINDJAMMER WAY: Array indices are NEVER mutation targets!
    /// When we see `arr[i] = x`, only `arr` is mutated, NOT `i`.
    /// This is critical for Copy types like usize - they should stay owned (by value).
    fn is_direct_mutation_target(&self, name: &str, target: &Expression) -> bool {
        match target {
            Expression::Identifier { name: id, .. } => id == name,

            // Field access: p.x = ... or p.field.nested = ...
            Expression::FieldAccess { object, .. } => {
                self.is_direct_mutation_target(name, object)
            }

            // Index access: arr[i] = ...
            // CRITICAL: Only check the object (arr), NEVER the index (i)!
            // The index is only READ, not mutated, even if the indexed element is mutated.
            Expression::Index { object, index: _, location: _ } => {
                self.is_direct_mutation_target(name, object)
            }

            _ => false,
        }
    }

    fn has_mutable_method_call(&self, name: &str, expr: &Expression, registry: &SignatureRegistry) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                // THE WINDJAMMER WAY: Check if the parameter appears ANYWHERE in the object chain
                // This catches both direct calls (grid.set()) and field calls (self.camera.move_to())
                if self.expr_contains_identifier(name, object) {
                    // THE PROPER SOLUTION: Look up method signature in SignatureRegistry
                    if let Some(sig) = registry.get_signature(method) {
                        if sig.has_self_receiver && sig.param_ownership.first() == Some(&OwnershipMode::MutBorrowed) {
                            return true;
                        }
                    }

                    // FALLBACK HEURISTIC: stdlib methods not yet in registry
                    let is_mutating_by_name = method.starts_with("push")
                        || method.starts_with("insert")
                        || method.starts_with("remove")
                        || method.starts_with("clear")
                        || method.starts_with("set")  // VoxelGrid::set()
                        || method.ends_with("_mut")
                        || method == "smooth_follow"  // Camera methods
                        || method == "look_at";

                    return is_mutating_by_name;
                }
                false
            }
            Expression::TryOp { expr, .. } => self.has_mutable_method_call(name, expr, registry),
            Expression::Block { statements, .. } => {
                for s in statements {
                    match s {
                        Statement::Expression { expr, .. } => {
                            if self.has_mutable_method_call(name, expr, registry) {
                                return true;
                            }
                        }
                        Statement::Let { value, .. } => {
                            if self.has_mutable_method_call(name, value, registry) {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                false
            }
            Expression::Call { arguments, .. } => {
                for (_label, arg) in arguments {
                    if self.has_mutable_method_call(name, arg, registry) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Known read-only methods that always take &self (not &mut self).
    /// If a method call on a parameter is NOT in this list, it could potentially mutate.
    #[allow(dead_code)]
    pub(super) fn is_known_readonly_method(method: &str) -> bool {
        matches!(
            method,
            // Collection inspection
            "len"
                | "is_empty"
                | "contains"
                | "contains_key"
                | "get"
                | "first"
                | "last"
                | "capacity"
                | "keys"
                | "values"
                // Iterators (take &self)
                | "iter"
                | "windows"
                | "chunks"
                | "enumerate"
                // Cloning/conversion (take &self)
                | "clone"
                | "to_string"
                | "to_owned"
                | "as_str"
                | "as_ref"
                | "as_slice"
                | "as_bytes"
                | "as_deref"
                // String inspection
                | "trim"
                | "starts_with"
                | "ends_with"
                | "chars"
                | "bytes"
                | "split"
                | "lines"
                | "to_lowercase"
                | "to_uppercase"
                | "is_ascii"
                // Numeric (Copy types, but include for completeness)
                | "abs"
                | "ceil"
                | "floor"
                | "round"
                | "sqrt"
                | "powi"
                | "powf"
                | "sin"
                | "cos"
                | "tan"
                | "log"
                | "exp"
                | "min"
                | "max"
                | "clamp"
                // Display/formatting
                | "display"
                | "fmt"
                // Comparison
                | "cmp"
                | "partial_cmp"
                | "eq"
                | "ne"
                // Type checking
                | "is_some"
                | "is_none"
                | "is_ok"
                | "is_err"
                | "unwrap"
                | "unwrap_or"
                | "unwrap_or_else"
                | "unwrap_or_default"
                | "expect"
                | "map"
                | "and_then"
                | "or_else"
                | "ok_or"
                | "ok_or_else"
        )
    }

    /// Check if the parameter is the receiver of method calls that could potentially mutate.
    /// Returns true if there are method calls on the parameter that aren't known to be read-only.
    /// This catches patterns like `loader.load(...)` where .load() could require &mut self.
    #[allow(dead_code)]
    pub(super) fn has_potentially_mutating_method_call(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            if self.stmt_has_potentially_mutating_method_call(name, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_has_potentially_mutating_method_call(
        &self,
        name: &str,
        stmt: &Statement<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_has_potentially_mutating_method_call(name, expr)
            }
            Statement::Let { value, .. } => {
                self.expr_has_potentially_mutating_method_call(name, value)
            }
            Statement::Return { value: Some(v), .. } => {
                self.expr_has_potentially_mutating_method_call(name, v)
            }
            Statement::Assignment { value, .. } => {
                self.expr_has_potentially_mutating_method_call(name, value)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_has_potentially_mutating_method_call(name, condition)
                    || self.has_potentially_mutating_method_call(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.has_potentially_mutating_method_call(name, b))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_has_potentially_mutating_method_call(name, condition)
                    || self.has_potentially_mutating_method_call(name, body)
            }
            Statement::Loop { body, .. } | Statement::For { body, .. } => {
                self.has_potentially_mutating_method_call(name, body)
            }
            Statement::Match { value, arms, .. } => {
                self.expr_has_potentially_mutating_method_call(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expr_has_potentially_mutating_method_call(name, arm.body))
            }
            _ => false,
        }
    }

    fn expr_has_potentially_mutating_method_call(
        &self,
        name: &str,
        expr: &Expression<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: id, .. } = &**object {
                    if id == name && !Self::is_known_readonly_method(method) {
                        return true;
                    }
                }
                // Also check if the parameter is the receiver via field chain
                if let Expression::FieldAccess { object: inner, .. } = &**object {
                    if let Expression::Identifier { name: id, .. } = &**inner {
                        if id == name && !Self::is_known_readonly_method(method) {
                            return true;
                        }
                    }
                }
                self.expr_has_potentially_mutating_method_call(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_has_potentially_mutating_method_call(name, arg))
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_has_potentially_mutating_method_call(name, arg)),
            Expression::Binary { left, right, .. } => {
                self.expr_has_potentially_mutating_method_call(name, left)
                    || self.expr_has_potentially_mutating_method_call(name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expr_has_potentially_mutating_method_call(name, operand)
            }
            Expression::Block { statements, .. } => {
                self.has_potentially_mutating_method_call(name, statements)
            }
            Expression::Index { object, index, .. } => {
                self.expr_has_potentially_mutating_method_call(name, object)
                    || self.expr_has_potentially_mutating_method_call(name, index)
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_has_potentially_mutating_method_call(name, object)
            }
            // TDD FIX: TryOp wraps expressions with `?` (error propagation).
            Expression::TryOp { expr, .. } => {
                self.expr_has_potentially_mutating_method_call(name, expr)
            }
            _ => false,
        }
    }

    /// Track which local variables are mutated in a function body
    /// This enables automatic `mut` inference - users don't need to write `let mut x`
    pub fn track_mutations(&mut self, statements: &[&'ast Statement<'ast>]) {
        self.mutated_variables.clear();
        self.collect_mutations(statements);
    }

    /// Recursively collect all variable mutations
    fn collect_mutations(&mut self, statements: &[&'ast Statement<'ast>]) {
        for stmt in statements {
            match stmt {
                Statement::Assignment {
                    target: Expression::Identifier { name, .. },
                    ..
                } => {
                    self.mutated_variables.insert(name.clone());
                }
                Statement::Assignment { target, .. } => {
                    self.collect_mutation_target(target);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.collect_mutations(then_block);
                    if let Some(else_stmts) = else_block {
                        self.collect_mutations(else_stmts);
                    }
                }
                Statement::Match { arms, .. } => {
                    let _ = arms;
                }
                Statement::For { pattern, body, .. } => {
                    self.collect_mutations(body);

                    if let Pattern::Identifier(var_name) = pattern {
                        if self.is_variable_mutated_in_statements(var_name, body) {
                            self.mutated_variables
                                .insert(format!("__loop_var_{}", var_name));
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    self.collect_mutations(body);
                }
                Statement::Expression { expr, .. } => {
                    self.collect_mutations_in_expression(expr);
                }
                // DOGFOODING FIX #2B: Track mutations in let bindings
                Statement::Let { value, .. } => {
                    self.collect_mutations_in_expression(value);
                }
                _ => {}
            }
        }
    }

    /// Track mutations in expressions (method calls that mutate)
    fn collect_mutations_in_expression(&mut self, expr: &Expression) {
        if let Expression::MethodCall { object, method, .. } = expr {
            // DOGFOODING FIX #2: Check method signature to see if it takes &mut self
            let type_name = if let Expression::Identifier { name, .. } = &**object {
                self.variables.get(name).cloned()
            } else {
                None
            };
            
            let method_requires_mut = if let Some(_type_name_str) = type_name {
                if let Some(impl_functions) = &self.current_impl_functions {
                    if let Some(func) = impl_functions.get(method.as_str()) {
                        func.parameters.iter()
                            .find(|p| p.name == "self")
                            .map(|p| matches!(p.ownership, OwnershipHint::Mut))
                            .unwrap_or(false)
                    } else {
                        Self::is_heuristic_mutating_method(method)
                    }
                } else {
                    Self::is_heuristic_mutating_method(method)
                }
            } else {
                Self::is_heuristic_mutating_method(method)
            };
            
            if method_requires_mut {
                if let Expression::Identifier { name, .. } = &**object {
                    self.mutated_variables.insert(name.clone());
                }
            }
        }
    }
    
    /// Heuristic: Common stdlib methods that take &mut self
    fn is_heuristic_mutating_method(method: &str) -> bool {
        matches!(
            method,
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "clear"
                | "append"
                | "extend"
                | "truncate"
                | "resize"
                | "sort"
                | "reverse"
                | "dedup"
                | "retain"
                | "drain"
                | "split_off"
                | "swap_remove"
        )
    }

    /// Check if a variable is mutated within a specific set of statements
    pub(super) fn is_variable_mutated_in_statements(
        &self,
        var_name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment { target, .. } => {
                    if let Expression::Identifier { name, .. } = target {
                        if name == var_name {
                            return true;
                        }
                    }
                    if let Expression::FieldAccess { object, .. } = target {
                        if let Expression::Identifier { name, .. } = &**object {
                            if name == var_name {
                                return true;
                            }
                        }
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    if self.is_variable_mutated_in_statements(var_name, body) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_variable_mutated_in_statements(var_name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_variable_mutated_in_statements(var_name, else_stmts) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a variable is mutated (for automatic mut inference)
    pub fn is_variable_mutated(&self, var_name: &str) -> bool {
        self.mutated_variables.contains(var_name)
    }

    /// Track mutation target (left side of assignment)
    fn collect_mutation_target(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier { name, .. } => {
                self.mutated_variables.insert(name.clone());
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_mutation_target(object);
            }
            Expression::Index { object, .. } => {
                self.collect_mutation_target(object);
            }
            _ => {}
        }
    }
}
