//! Mutation detection methods for the analyzer.
//! Determines whether parameters or local variables are mutated,
//! enabling automatic &mut inference and mut binding inference.

use crate::parser::*;

use super::{Analyzer, FunctionSignature, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// THE WINDJAMMER WAY: Check if an expression contains a specific identifier
    /// Used to detect if a parameter is used in a method call chain (e.g., self.camera.move_to())
    ///
    /// CRITICAL: For Index expressions, only check the object, NOT the index!
    /// When we see `arr[i].method()`, only `arr` is being used mutably, NOT `i`.
    /// The index `i` is just being READ to select which element to call the method on.
    #[allow(dead_code)] // Reserved for future mutation analysis
    pub(crate) fn expr_contains_identifier(&self, name: &str, expr: &Expression) -> bool {
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
        param_type_hint: Option<&Type>,
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
                    if self.has_mutable_method_call(name, expr, registry, param_type_hint) {
                        return true;
                    }
                }
                Statement::Let { value, .. } => {
                    if self.has_mutable_method_call(name, value, registry, param_type_hint) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.has_mutable_method_call(name, expr, registry, param_type_hint) {
                        return true;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.has_mutable_method_call(name, condition, registry, param_type_hint) {
                        return true;
                    }
                    if self.is_mutated(name, then_block, registry, param_type_hint) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_mutated(name, else_b, registry, param_type_hint) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. } => {
                    if self.is_mutated(name, body, registry, param_type_hint) {
                        return true;
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if self.has_mutable_method_call(name, condition, registry, param_type_hint) {
                        return true;
                    }
                    if self.is_mutated(name, body, registry, param_type_hint) {
                        return true;
                    }
                }
                Statement::For { iterable, body, .. } => {
                    if self.has_mutable_method_call(name, iterable, registry, param_type_hint) {
                        return true;
                    }
                    if self.is_mutated(name, body, registry, param_type_hint) {
                        return true;
                    }
                }
                Statement::Match { value, arms, .. } => {
                    if self.has_mutable_method_call(name, value, registry, param_type_hint) {
                        return true;
                    }
                    for arm in arms {
                        if let Some(guard) = arm.guard {
                            if self.has_mutable_method_call(name, guard, registry, param_type_hint)
                            {
                                return true;
                            }
                        }
                        if self.is_mutated_in_match_arm_body(
                            name,
                            value,
                            arm,
                            registry,
                            param_type_hint,
                        ) {
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
    pub(crate) fn is_mutated_in_match_arm_body(
        &self,
        name: &str,
        scrutinee: &Expression<'ast>,
        arm: &MatchArm<'ast>,
        registry: &SignatureRegistry,
        param_type_hint: Option<&Type>,
    ) -> bool {
        if self.if_let_some_mutates_indexed_binding_of_param(name, scrutinee, arm, registry) {
            return true;
        }
        match &arm.body {
            Expression::Block { statements, .. } => {
                self.is_mutated(name, statements, registry, param_type_hint)
            }
            _ => self.has_mutable_method_call(name, arm.body, registry, param_type_hint),
        }
    }

    /// `if let Some(x) = param[i]` with `Option` inner `Copy`: mutating `x` must update `param`'s
    /// slot, so treat `param` as mut-borrowed. Plain `is_mutated` misses this because assignments
    /// target `x`, not `param`.
    pub(crate) fn if_let_some_mutates_indexed_binding_of_param(
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

    pub(crate) fn enum_some_single_binding<'p>(pattern: &'p Pattern<'p>) -> Option<&'p str> {
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
    pub(crate) fn expr_has_indexed_access(expr: &Expression<'_>) -> bool {
        match expr {
            Expression::Index { .. } => true,
            Expression::FieldAccess { object, .. } => Self::expr_has_indexed_access(object),
            _ => false,
        }
    }

    pub(crate) fn match_binding_is_assignment_target(expr: &Expression, var: &str) -> bool {
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

    pub(crate) fn match_arm_body_mutates_binding(
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
    pub(crate) fn expr_may_mutate_if_let_some_binding(
        &self,
        binding: &str,
        expr: &Expression<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        if self.has_mutable_method_call(binding, expr, registry, None) {
            return true;
        }
        if let Expression::MethodCall { object, method, .. } = expr {
            if self.is_in_receiver_chain(binding, object) {
                let receiver_base =
                    self.receiver_type_base_for_param_method_call(binding, object, None);
                return super::stdlib_method_traits::method_call_mutates_receiver(
                    method,
                    receiver_base.as_deref(),
                    registry,
                );
            }
        }
        false
    }

    pub(crate) fn stmt_mutates_binding_in_tree(
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
                    if self.has_mutable_method_call(binding, g, registry, None) {
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
    pub(crate) fn is_direct_mutation_target(&self, name: &str, target: &Expression) -> bool {
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

    /// True when `name` is the direct assignment target (e.g. `p.x = …`, `p = …`).
    /// Excludes method-call-only mutation such as `map.remove(key)`.
    pub(crate) fn is_field_mutated(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment { target, .. } => {
                    if self.is_direct_mutation_target(name, target) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_field_mutated(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_field_mutated(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. } | Statement::While { body, .. } => {
                    if self.is_field_mutated(name, body) {
                        return true;
                    }
                }
                Statement::For { body, .. } => {
                    if self.is_field_mutated(name, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a parameter is in the direct receiver chain of a method call.
    /// Only follows the object path: param.field.method() -> true
    /// Does NOT match arguments of nested calls: f.method(param).other() -> false
    ///
    /// This prevents false mutation detection for parameters that are merely
    /// passed as arguments to intermediate methods in a chain.
    /// Example: f.cross(up).normalize() - up is an argument to cross, NOT
    /// a receiver of normalize, so normalize's mutability doesn't apply to up.
    pub(crate) fn is_in_receiver_chain(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.is_in_receiver_chain(name, object),
            Expression::MethodCall { object, .. } => self.is_in_receiver_chain(name, object),
            Expression::Index { object, .. } => self.is_in_receiver_chain(name, object),
            _ => false,
        }
    }

    /// Resolve the type at the end of a field-access chain rooted at `param_name`.
    fn resolve_field_chain_type_for_param(
        &self,
        param_name: &str,
        expr: &Expression,
        param_type_hint: Option<&Type>,
    ) -> Option<Type> {
        match expr {
            Expression::FieldAccess { object, field, .. } => {
                let base =
                    self.resolve_field_chain_type_for_param(param_name, object, param_type_hint)?;
                self.lookup_field_type_on_struct(&base, field)
            }
            Expression::Index { object, .. } => {
                let base =
                    self.resolve_field_chain_type_for_param(param_name, object, param_type_hint)?;
                match &base {
                    Type::Vec(inner) | Type::Array(inner, _) => Some((**inner).clone()),
                    Type::Parameterized(name, params) if name == "Vec" && !params.is_empty() => {
                        Some(params[0].clone())
                    }
                    _ => None,
                }
            }
            Expression::Identifier { name, .. } if name == param_name => param_type_hint.cloned(),
            _ => None,
        }
    }

    /// Look up a free-function signature from a call expression, mirroring passthrough inference.
    fn lookup_call_signature<'a>(
        registry: &'a SignatureRegistry,
        func_name: &str,
    ) -> Option<&'a FunctionSignature> {
        registry.lookup_method(func_name).or_else(|| {
            func_name
                .rsplit("::")
                .next()
                .filter(|simple| *simple != func_name)
                .and_then(|simple| registry.get_signature(simple))
        })
    }

    /// `module::func` or `Type::func` style call target for registry lookup.
    fn call_expr_qualified_name(function: &Expression) -> Option<String> {
        match function {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier { name: obj, .. } = &**object {
                    Some(format!("{}::{}", obj, field))
                } else {
                    Some(field.clone())
                }
            }
            _ => None,
        }
    }

    fn is_external_module_call(function: &Expression) -> bool {
        match function {
            Expression::FieldAccess { object, .. } => matches!(
                &**object,
                Expression::Identifier { name, .. }
                    if name.chars().next().is_some_and(|c| c.is_lowercase())
            ),
            Expression::Identifier { name, .. } => {
                name.contains("::") && name.chars().next().is_some_and(|c| c.is_lowercase())
            }
            _ => false,
        }
    }

    /// True when `name` is passed to a call whose callee expects `&mut` at that argument index.
    fn param_passed_to_mut_borrowed_call_arg(
        &self,
        name: &str,
        function: &Expression,
        arguments: &[(Option<String>, &Expression)],
        registry: &SignatureRegistry,
        param_type_hint: Option<&Type>,
    ) -> bool {
        for (i, (_, arg)) in arguments.iter().enumerate() {
            if !matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                continue;
            }
            let func_name = Self::call_expr_qualified_name(function)
                .or_else(|| self.extract_function_name(function));
            let Some(func_name) = func_name else {
                continue;
            };
            if let Some(sig) = Self::lookup_call_signature(registry, &func_name) {
                if sig
                    .param_ownership_for_arg(i)
                    .is_some_and(|m| matches!(m, OwnershipMode::MutBorrowed))
                {
                    return true;
                }
            } else if Self::is_external_module_call(function)
                && i == 0
                && param_type_hint.is_some_and(|ty| !self.is_copy_type(ty))
            {
                // Cross-crate module calls often mutate the first non-Copy argument (e.g.
                // `station_builder::set_if(grid, ...)`). When engine metadata omits the callee,
                // infer mutation from the call pattern instead of cloning at every callsite.
                return true;
            }
        }
        false
    }

    /// Resolve the registry type base for `param_name.method()` or `param_name.field...method()`.
    fn receiver_type_base_for_param_method_call(
        &self,
        param_name: &str,
        object: &Expression,
        param_type_hint: Option<&Type>,
    ) -> Option<String> {
        if let Expression::Identifier { name: recv, .. } = object {
            if recv == param_name {
                if let Some(ty) = param_type_hint {
                    return type_base_for_registry_lookup(ty);
                }
            }
        }
        self.resolve_field_chain_type_for_param(param_name, object, param_type_hint)
            .and_then(|ty| type_base_for_registry_lookup(&ty))
    }

    pub(crate) fn has_mutable_method_call(
        &self,
        name: &str,
        expr: &Expression,
        registry: &SignatureRegistry,
        param_type_hint: Option<&Type>,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if self.is_in_receiver_chain(name, object) {
                    let receiver_base = self.receiver_type_base_for_param_method_call(
                        name,
                        object,
                        param_type_hint,
                    );
                    if super::stdlib_method_traits::method_call_mutates_receiver(
                        method,
                        receiver_base.as_deref(),
                        registry,
                    ) {
                        return true;
                    }
                    if receiver_base.is_some() {
                        return false;
                    }
                    if param_type_hint.is_none() {
                        return true;
                    }
                    return false;
                }

                // HashMap/HashSet key args are Borrowed (`&Q`), never MutBorrowed.
                // Signature loop below decides MutBorrowed from param_ownership — do not
                // short-circuit on method-name lists (Vec::remove shares "remove").
                if let Expression::MethodCall { arguments, .. } = expr {
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                            if let Some(sig) = registry.lookup_method(method) {
                                if sig
                                    .param_ownership_for_arg(i)
                                    .is_some_and(|m| matches!(m, OwnershipMode::MutBorrowed))
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            Expression::TryOp { expr, .. } => {
                self.has_mutable_method_call(name, expr, registry, param_type_hint)
            }
            Expression::Block { statements, .. } => {
                for s in statements {
                    match s {
                        Statement::Expression { expr, .. } => {
                            if self.has_mutable_method_call(name, expr, registry, param_type_hint) {
                                return true;
                            }
                        }
                        Statement::Let { value, .. } => {
                            if self.has_mutable_method_call(name, value, registry, param_type_hint)
                            {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                false
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if self.param_passed_to_mut_borrowed_call_arg(
                    name,
                    function,
                    arguments,
                    registry,
                    param_type_hint,
                ) {
                    return true;
                }
                for (_label, arg) in arguments {
                    if self.has_mutable_method_call(name, arg, registry, param_type_hint) {
                        return true;
                    }
                }
                false
            }
            Expression::Unary { operand, .. } => {
                self.has_mutable_method_call(name, operand, registry, param_type_hint)
            }
            Expression::Binary { left, right, .. } => {
                self.has_mutable_method_call(name, left, registry, param_type_hint)
                    || self.has_mutable_method_call(name, right, registry, param_type_hint)
            }
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    if self.has_mutable_method_call(name, e, registry, param_type_hint) {
                        return true;
                    }
                }
                false
            }
            Expression::Index { object, index, .. } => {
                self.has_mutable_method_call(name, object, registry, param_type_hint)
                    || self.has_mutable_method_call(name, index, registry, param_type_hint)
            }
            Expression::FieldAccess { object, .. } => {
                self.has_mutable_method_call(name, object, registry, param_type_hint)
            }
            _ => false,
        }
    }

    /// Check if the parameter is the receiver of method calls that could potentially mutate.
    /// Returns true when registry lookup shows mutation or the method is not consensus-read-only.
    pub(super) fn has_potentially_mutating_method_call_in_tryop(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
        param_type: &Type,
    ) -> bool {
        for stmt in statements {
            if self.stmt_has_potentially_mutating_method_call_in_tryop(
                name, stmt, registry, param_type,
            ) {
                return true;
            }
        }
        false
    }

    fn stmt_has_potentially_mutating_method_call_in_tryop(
        &self,
        name: &str,
        stmt: &Statement<'ast>,
        registry: &SignatureRegistry,
        param_type: &Type,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self
                .expr_has_potentially_mutating_method_call_in_tryop(
                    name, expr, registry, param_type,
                ),
            Statement::Let { value, .. } => self
                .expr_has_potentially_mutating_method_call_in_tryop(
                    name, value, registry, param_type,
                ),
            Statement::Return { value: Some(v), .. } => self
                .expr_has_potentially_mutating_method_call_in_tryop(name, v, registry, param_type),
            Statement::Assignment { value, .. } => self
                .expr_has_potentially_mutating_method_call_in_tryop(
                    name, value, registry, param_type,
                ),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_has_potentially_mutating_method_call_in_tryop(
                    name, condition, registry, param_type,
                ) || self.has_potentially_mutating_method_call_in_tryop(
                    name, then_block, registry, param_type,
                ) || else_block.as_ref().is_some_and(|b| {
                    self.has_potentially_mutating_method_call_in_tryop(
                        name, b, registry, param_type,
                    )
                })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_has_potentially_mutating_method_call_in_tryop(
                    name, condition, registry, param_type,
                ) || self
                    .has_potentially_mutating_method_call_in_tryop(name, body, registry, param_type)
            }
            Statement::Loop { body, .. } | Statement::For { body, .. } => {
                self.has_potentially_mutating_method_call_in_tryop(name, body, registry, param_type)
            }
            Statement::Match { value, arms, .. } => {
                self.expr_has_potentially_mutating_method_call_in_tryop(
                    name, value, registry, param_type,
                ) || arms.iter().any(|arm| {
                    self.expr_has_potentially_mutating_method_call_in_tryop(
                        name, arm.body, registry, param_type,
                    )
                })
            }
            _ => false,
        }
    }

    /// True when `param.method()` (or `param.field…method()`) under `?` may need `&mut self`.
    fn param_receiver_method_might_mutate(
        &self,
        param_name: &str,
        param_type: &Type,
        object: &Expression<'ast>,
        method: &str,
        registry: &SignatureRegistry,
    ) -> bool {
        if !self.is_in_receiver_chain(param_name, object) {
            return false;
        }
        let receiver_base =
            self.receiver_type_base_for_param_method_call(param_name, object, Some(param_type));
        if super::stdlib_method_traits::method_call_mutates_receiver(
            method,
            receiver_base.as_deref(),
            registry,
        ) {
            return true;
        }
        if super::stdlib_method_traits::method_call_consumes_receiver(
            method,
            receiver_base.as_deref(),
            registry,
        ) {
            return true;
        }
        // Type-qualified readonly wins — do not require unqualified stdlib consensus
        // (user `AssetLoader::load` must not be poisoned by missing `::{load}` consensus).
        if super::stdlib_method_traits::is_known_readonly_qualified(
            method,
            receiver_base.as_deref(),
            registry,
        ) {
            return false;
        }
        false
    }

    fn expr_has_potentially_mutating_method_call_in_tryop(
        &self,
        name: &str,
        expr: &Expression<'ast>,
        registry: &SignatureRegistry,
        param_type: &Type,
    ) -> bool {
        match expr {
            Expression::TryOp { expr: inner, .. } => {
                self.expr_under_tryop_might_mutate_param(name, inner, registry, param_type)
            }
            Expression::Block { statements, .. } => self
                .has_potentially_mutating_method_call_in_tryop(
                    name, statements, registry, param_type,
                ),
            Expression::Binary { left, right, .. } => {
                self.expr_has_potentially_mutating_method_call_in_tryop(
                    name, left, registry, param_type,
                ) || self.expr_has_potentially_mutating_method_call_in_tryop(
                    name, right, registry, param_type,
                )
            }
            Expression::Unary { operand, .. } => self
                .expr_has_potentially_mutating_method_call_in_tryop(
                    name, operand, registry, param_type,
                ),
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expr_has_potentially_mutating_method_call_in_tryop(
                    name, arg, registry, param_type,
                )
            }),
            _ => false,
        }
    }

    /// Recurse into the scrutinee of `?` — method calls here may need `&mut self`.
    fn expr_under_tryop_might_mutate_param(
        &self,
        name: &str,
        expr: &Expression<'ast>,
        registry: &SignatureRegistry,
        param_type: &Type,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if self
                    .param_receiver_method_might_mutate(name, param_type, object, method, registry)
                {
                    return true;
                }
                self.expr_under_tryop_might_mutate_param(name, object, registry, param_type)
                    || arguments.iter().any(|(_, arg)| {
                        self.expr_under_tryop_might_mutate_param(name, arg, registry, param_type)
                    })
            }
            Expression::TryOp { expr: inner, .. } => {
                self.expr_under_tryop_might_mutate_param(name, inner, registry, param_type)
            }
            Expression::Block { statements, .. } => self
                .has_potentially_mutating_method_call_in_tryop(
                    name, statements, registry, param_type,
                ),
            Expression::Binary { left, right, .. } => {
                self.expr_under_tryop_might_mutate_param(name, left, registry, param_type)
                    || self.expr_under_tryop_might_mutate_param(name, right, registry, param_type)
            }
            Expression::Unary { operand, .. } => {
                self.expr_under_tryop_might_mutate_param(name, operand, registry, param_type)
            }
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expr_under_tryop_might_mutate_param(name, arg, registry, param_type)
            }),
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
    pub(crate) fn receiver_root_local_identifier<'e>(expr: &'e Expression<'e>) -> Option<&'e str> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                Self::receiver_root_local_identifier(object)
            }
            _ => None,
        }
    }

    /// Recursively collect all variable mutations
    pub(crate) fn collect_mutations(
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
    pub(crate) fn collect_mutations_in_expression(
        &mut self,
        expr: &Expression,
        registry: &SignatureRegistry,
    ) {
        if let Expression::MethodCall { object, .. } = expr {
            if let Some(root) = Self::receiver_root_local_identifier(object) {
                if root != "self" && self.has_mutable_method_call(root, expr, registry, None) {
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
    pub(crate) fn collect_mutation_target(&mut self, expr: &Expression) {
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

fn type_base_for_registry_lookup(ty: &Type) -> Option<String> {
    crate::type_classification::type_to_registry_base(ty)
}
