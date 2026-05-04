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
    #[allow(dead_code)] // Reserved for future mutation analysis
    fn expr_contains_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expr_contains_identifier(name, object),
            // THE FIX: Don't check the index part - it's only read, never mutated!
            // Before: self.expr_contains_identifier(name, object) || self.expr_contains_identifier(name, index)
            // After: Only check object
            Expression::Index {
                object,
                index: _,
                location: _,
            } => self.expr_contains_identifier(name, object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
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

    pub(super) fn is_mutated(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> bool {
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
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.has_mutable_method_call(name, expr, registry) {
                        return true;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.has_mutable_method_call(name, condition, registry) {
                        return true;
                    }
                    if self.is_mutated(name, then_block, registry) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_mutated(name, else_b, registry) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. } => {
                    if self.is_mutated(name, body, registry) {
                        return true;
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if self.has_mutable_method_call(name, condition, registry) {
                        return true;
                    }
                    if self.is_mutated(name, body, registry) {
                        return true;
                    }
                }
                Statement::For { iterable, body, .. } => {
                    if self.has_mutable_method_call(name, iterable, registry) {
                        return true;
                    }
                    if self.is_mutated(name, body, registry) {
                        return true;
                    }
                }
                Statement::Match { value, arms, .. } => {
                    if self.has_mutable_method_call(name, value, registry) {
                        return true;
                    }
                    for arm in arms {
                        if let Some(guard) = arm.guard {
                            if self.has_mutable_method_call(name, guard, registry) {
                                return true;
                            }
                        }
                        if self.is_mutated_in_match_arm_body(name, value, arm, registry) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Match arm bodies are expressions; blocks contain real statement lists.
    fn is_mutated_in_match_arm_body(
        &self,
        name: &str,
        scrutinee: &Expression<'ast>,
        arm: &MatchArm<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        if self.if_let_some_mutates_indexed_binding_of_param(name, scrutinee, arm, registry) {
            return true;
        }
        match &arm.body {
            Expression::Block { statements, .. } => self.is_mutated(name, statements, registry),
            _ => self.has_mutable_method_call(name, arm.body, registry),
        }
    }

    /// `if let Some(x) = param[i]` with `Option` inner `Copy`: mutating `x` must update `param`'s
    /// slot, so treat `param` as mut-borrowed. Plain `is_mutated` misses this because assignments
    /// target `x`, not `param`.
    fn if_let_some_mutates_indexed_binding_of_param(
        &self,
        param: &str,
        scrutinee: &Expression<'ast>,
        arm: &MatchArm<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        if matches!(arm.pattern, Pattern::Wildcard) {
            return false;
        }
        let Some(inner_binding) = Self::enum_some_single_binding(&arm.pattern) else {
            return false;
        };
        if Self::receiver_root_local_identifier(scrutinee) != Some(param) {
            return false;
        }
        if !Self::expr_has_indexed_access(scrutinee) {
            return false;
        }
        self.match_arm_body_mutates_binding(inner_binding, arm.body, registry)
    }

    fn enum_some_single_binding<'p>(pattern: &'p Pattern<'p>) -> Option<&'p str> {
        match pattern {
            Pattern::EnumVariant(v, EnumPatternBinding::Single(name))
                if v == "Some" || v.ends_with("::Some") =>
            {
                Some(name.as_str())
            }
            _ => None,
        }
    }

    /// True if `expr` is or contains an index operation (`vec[i]`, `a.b[i]`).
    fn expr_has_indexed_access(expr: &Expression<'_>) -> bool {
        match expr {
            Expression::Index { .. } => true,
            Expression::FieldAccess { object, .. } => Self::expr_has_indexed_access(object),
            _ => false,
        }
    }

    fn match_binding_is_assignment_target(expr: &Expression, var: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var,
            Expression::FieldAccess { object, .. } => {
                Self::match_binding_is_assignment_target(object, var)
            }
            Expression::Index { object, .. } => {
                Self::match_binding_is_assignment_target(object, var)
            }
            Expression::Unary {
                op: UnaryOp::Deref,
                operand,
                ..
            } => Self::match_binding_is_assignment_target(operand, var),
            _ => false,
        }
    }

    fn match_arm_body_mutates_binding(
        &self,
        binding: &str,
        body: &Expression<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        match body {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_mutates_binding_in_tree(s, binding, registry)),
            _ => self.expr_may_mutate_if_let_some_binding(binding, body, registry),
        }
    }

    /// Like [Self::has_mutable_method_call], plus: unknown methods on `binding` (not known &-self
    /// std APIs) count as mutations. Used only for `if let Some(x) = vec[i]` bodies so `.add()` on
    /// a Copy `Option` payload is not mistaken for a read (see mutability_complete_test).
    fn expr_may_mutate_if_let_some_binding(
        &self,
        binding: &str,
        expr: &Expression<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        if self.has_mutable_method_call(binding, expr, registry) {
            return true;
        }
        if let Expression::MethodCall { object, method, .. } = expr {
            if self.is_in_receiver_chain(binding, object) {
                return !crate::method_registry::is_known_readonly_method(method);
            }
        }
        false
    }

    fn stmt_mutates_binding_in_tree(
        &self,
        stmt: &Statement<'ast>,
        binding: &str,
        registry: &SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                Self::match_binding_is_assignment_target(target, binding)
            }
            Statement::Expression { expr, .. } => {
                self.expr_may_mutate_if_let_some_binding(binding, expr, registry)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.stmt_mutates_binding_in_tree(s, binding, registry))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_mutates_binding_in_tree(s, binding, registry))
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.stmt_mutates_binding_in_tree(s, binding, registry)),
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.stmt_mutates_binding_in_tree(s, binding, registry)),
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Some(g) = arm.guard {
                    if self.has_mutable_method_call(binding, g, registry) {
                        return true;
                    }
                }
                self.match_arm_body_mutates_binding(binding, arm.body, registry)
            }),
            Statement::Let { value, .. } | Statement::Const { value, .. } => {
                self.expr_may_mutate_if_let_some_binding(binding, value, registry)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_may_mutate_if_let_some_binding(binding, expr, registry),
            _ => false,
        }
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
            Expression::FieldAccess { object, .. } => self.is_direct_mutation_target(name, object),

            // Index access: arr[i] = ...
            // CRITICAL: Only check the object (arr), NEVER the index (i)!
            // The index is only READ, not mutated, even if the indexed element is mutated.
            Expression::Index {
                object,
                index: _,
                location: _,
            } => self.is_direct_mutation_target(name, object),

            _ => false,
        }
    }

    /// Check if a parameter is in the direct receiver chain of a method call.
    /// Only follows the object path: param.field.method() -> true
    /// Does NOT match arguments of nested calls: f.method(param).other() -> false
    ///
    /// This prevents false mutation detection for parameters that are merely
    /// passed as arguments to intermediate methods in a chain.
    /// Example: f.cross(up).normalize() - up is an argument to cross, NOT
    /// a receiver of normalize, so normalize's mutability doesn't apply to up.
    fn is_in_receiver_chain(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.is_in_receiver_chain(name, object),
            Expression::MethodCall { object, .. } => self.is_in_receiver_chain(name, object),
            Expression::Index { object, .. } => self.is_in_receiver_chain(name, object),
            _ => false,
        }
    }

    fn has_mutable_method_call(
        &self,
        name: &str,
        expr: &Expression,
        registry: &SignatureRegistry,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                // Check if the parameter is in the DIRECT receiver chain (not just any argument).
                // This catches: param.set(), param.field.push(), self.camera.move_to()
                // But NOT: f.cross(param).normalize() (param is an argument, not a receiver)
                if self.is_in_receiver_chain(name, object) {
                    // THE PROPER SOLUTION: Look up method signature in SignatureRegistry
                    if let Some(sig) = registry.get_signature(method) {
                        if sig.has_self_receiver {
                            if let Some(mode) = sig.param_ownership.first() {
                                // Only &mut self methods constitute mutation.
                                // Owned self CONSUMES the receiver (move), it doesn't mutate it.
                                // Treating Owned as mutation incorrectly forces callers to pass
                                // &mut when they should pass an owned value (e.g., for HashMap::insert).
                                if matches!(mode, OwnershipMode::MutBorrowed) {
                                    return true;
                                }
                            }
                        }
                    }

                    if crate::method_registry::mutates_receiver(method) {
                        return true;
                    }
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
            Expression::Unary { operand, .. } => {
                self.has_mutable_method_call(name, operand, registry)
            }
            Expression::Binary { left, right, .. } => {
                self.has_mutable_method_call(name, left, registry)
                    || self.has_mutable_method_call(name, right, registry)
            }
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    if self.has_mutable_method_call(name, e, registry) {
                        return true;
                    }
                }
                false
            }
            Expression::Index { object, index, .. } => {
                self.has_mutable_method_call(name, object, registry)
                    || self.has_mutable_method_call(name, index, registry)
            }
            Expression::FieldAccess { object, .. } => {
                self.has_mutable_method_call(name, object, registry)
            }
            _ => false,
        }
    }

    /// Known read-only methods that always take &self (not &mut self).
    /// Delegates to the centralized method_registry — single source of truth.
    pub(super) fn is_known_readonly_method(method: &str) -> bool {
        crate::method_registry::is_known_readonly_method(method)
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
    pub fn track_mutations(
        &mut self,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) {
        self.mutated_variables.clear();
        self.collect_mutations(statements, registry);
    }

    /// Root local binding for `a.b.c` / `a[i]` receiver chains (not `self`).
    fn receiver_root_local_identifier<'e>(expr: &'e Expression<'e>) -> Option<&'e str> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                Self::receiver_root_local_identifier(object)
            }
            _ => None,
        }
    }

    /// Recursively collect all variable mutations
    fn collect_mutations(
        &mut self,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) {
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
                    self.collect_mutations(then_block, registry);
                    if let Some(else_stmts) = else_block {
                        self.collect_mutations(else_stmts, registry);
                    }
                }
                Statement::Match { arms, .. } => {
                    let _ = arms;
                }
                Statement::For { pattern, body, .. } => {
                    self.collect_mutations(body, registry);

                    if let Pattern::Identifier(var_name) = pattern {
                        if self.is_variable_mutated_in_statements(var_name, body) {
                            self.mutated_variables
                                .insert(format!("__loop_var_{}", var_name));
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    self.collect_mutations(body, registry);
                }
                Statement::Expression { expr, .. } => {
                    self.collect_mutations_in_expression(expr, registry);
                }
                // DOGFOODING FIX #2B: Track mutations in let bindings
                Statement::Let { value, .. } => {
                    self.collect_mutations_in_expression(value, registry);
                }
                _ => {}
            }
        }
    }

    /// Track mutations in expressions (method calls that mutate)
    ///
    /// Aligns with [`Self::has_mutable_method_call`]: `local.field.mut_method()` marks `local`
    /// when the method's analyzed signature uses `&mut self`.
    fn collect_mutations_in_expression(&mut self, expr: &Expression, registry: &SignatureRegistry) {
        if let Expression::MethodCall { object, .. } = expr {
            if let Some(root) = Self::receiver_root_local_identifier(object) {
                if root != "self" && self.has_mutable_method_call(root, expr, registry) {
                    self.mutated_variables.insert(root.to_string());
                }
            }
        }
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
