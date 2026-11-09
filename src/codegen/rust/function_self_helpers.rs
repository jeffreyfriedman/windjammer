//! Self receiver, trait ownership, lifetime, and #[inline] heuristics for function codegen.

use crate::analyzer::*;
use crate::codegen::rust::ast_utilities;
use crate::parser::*;

use super::CodeGenerator;

/// A receiver the source declares (`&self` / `&mut self`): the contract of a trait method and its impls (W0016).
pub(super) fn declared_receiver(param: &Parameter) -> Option<&'static str> {
    match param.ownership {
        _ if param.name != "self" => None,
        OwnershipHint::Ref => Some("&self"),
        OwnershipHint::Mut => Some("&mut self"),
        _ => None,
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// The one receiver decision codegen makes (convergence C5): spell a mode, never re-decide it.
    /// Outside trait impls the mode is the IR solver's (the analyzer's only when no IR exists: isolated
    /// unit tests); in a trait impl it is the trait's. `Owned` becomes `mut self` when the body writes
    /// through `self`. `None` only when there is no mode at all.
    pub(super) fn self_receiver(&mut self, analyzed: &AnalyzedFunction<'ast>) -> Option<&'static str> {
        let func = &analyzed.decl;
        let (mode, modifies) = if self.in_trait_impl {
            (self.get_effective_self_ownership(&func.name, analyzed)?, self.function_modifies_self(func))
        } else {
            let mode = self
                .ir_param_ownership_definitive("self")
                .or_else(|| analyzed.inferred_ownership.get("self").copied())?;
            (mode, self.function_modifies_self_or_derived(func))
        };
        let receiver = match mode {
            OwnershipMode::Borrowed => "&self",
            OwnershipMode::MutBorrowed => "&mut self",
            OwnershipMode::Owned if modifies => "mut self",
            OwnershipMode::Owned => "self",
        };
        self.sync_self_borrow_tracking_from_emitted_params(&[receiver.to_string()]);
        self.record_self_receiver_upgrade(&func.name, None, receiver);
        Some(receiver)
    }

    /// The formal the IR decides: the receiver spelling for `self`, else the value-formal mode.
    pub(super) fn ir_decided_formal(&mut self, param: &Parameter<'ast>, analyzed: &AnalyzedFunction<'ast>, unused: &std::collections::HashSet<String>) -> Option<String> {
        if param.name == "self" {
            return Some(self.self_receiver(analyzed).unwrap_or("&self").to_string());
        }
        self.ir_value_formal(param, analyzed, unused)
    }

    /// O4 (convergence C5): a plain value formal (a non-Copy user type or a `Vec`, declared by value,
    /// no pattern or pin) is spelled from the IR mode: `&T`, `&mut T` or `T`. Text, Copy, generic and
    /// trait-impl formals keep their own spelling rules for now. `None` when the IR has no mode.
    fn ir_value_formal(&mut self, param: &Parameter<'ast>, analyzed: &AnalyzedFunction<'ast>, unused: &std::collections::HashSet<String>) -> Option<String> {
        let plain = matches!(&param.type_, Type::Custom(_) | Type::Vec(_) | Type::Parameterized(..))
            && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
            && !self.is_type_copy(&param.type_)
            && !matches!(&param.type_, Type::Custom(n) if crate::analyzer::Analyzer::is_generic_type_param(&Type::Custom(n.clone())));
        // Public formals too: other crates' call sites read the emitted formals from `metadata.json`.
        if self.in_trait_impl || param.pattern.is_some() || !param.decorators.is_empty() || !plain {
            return None;
        }
        let ty = self.type_to_rust(&param.type_);
        let name = param.name.clone();
        // The IR's mode; the analyzer's only where no IR exists (isolated unit tests).
        let mode = self.ir_param_ownership_definitive(&name).or_else(|| analyzed.inferred_ownership.get(&name).copied())?;
        // `mut p: T` declares an owned, rebindable binding (`p = next(p)`), whatever the body writes through it.
        let mode = if param.is_mutable { OwnershipMode::Owned } else { mode };
        let shown = if unused.contains(&name) { format!("_{name}") } else { name.clone() };
        Some(match mode {
            OwnershipMode::Borrowed => {
                self.emitted_rust_ref_formals.insert(name.clone());
                self.inferred_borrowed_params.insert(name.clone());
                self.inferred_mut_borrowed_params.remove(&name);
                format!("{shown}: &{ty}")
            }
            OwnershipMode::MutBorrowed => {
                self.emitted_rust_ref_formals.insert(name.clone());
                self.inferred_mut_borrowed_params.insert(name.clone());
                self.inferred_borrowed_params.remove(&name);
                format!("{shown}: &mut {ty}")
            }
            OwnershipMode::Owned => {
                self.emitted_rust_ref_formals.remove(&name);
                self.inferred_borrowed_params.remove(&name);
                self.inferred_mut_borrowed_params.remove(&name);
                let rebinds = param.is_mutable || analyzed.mutated_parameters.contains(&name);
                format!("{}{shown}: {ty}", if rebinds { "mut " } else { "" })
            }
        })
    }

    pub(super) fn method_returns_impl_struct(&self, func: &FunctionDecl) -> bool {
        use crate::parser::Type;
        let returns_self_type = match &func.return_type {
            Some(Type::Custom(name)) => {
                self.current_struct_name.as_ref().is_some_and(|s| s == name)
            }
            _ => false,
        };
        if !returns_self_type {
            return false;
        }

        // Builder / consuming pattern: method flows `self` through or mutates while building.
        // Read-only factories (snapshot, clone, etc.) construct a NEW instance from cloned
        // fields and must use `&self` — not a hardcoded name list.
        if self.function_returns_self_type(func) {
            return true;
        }
        // Mutate-then-rebuild (`self.fields.push(x); Type { fields: self.fields }`) is a
        // consuming builder, not a snapshot. Check mutation/moves before the clone-factory path.
        if self.function_modifies_self(func)
            || super::self_analysis::function_return_moves_self_fields(func)
            || super::self_analysis::function_flows_self_through_local(func)
        {
            return true;
        }
        if super::self_analysis::function_returns_new_instance_from_self_fields(
            func,
            Some(&self.signature_registry),
            self.current_struct_name.as_deref(),
        ) {
            // Non-Copy: field-read rebuild is snapshot/clone (`&self`). Copy types use
            // cheap by-value receivers for consuming transforms like `double(self) -> Point`.
            return self.current_struct_is_copy();
        }
        false
    }

    pub(super) fn function_modifies_self_or_derived(&self, func: &FunctionDecl) -> bool {
        super::self_analysis::function_modifies_self_or_derived_local(
            func,
            Some(&self.signature_registry),
            self.current_struct_name.as_deref(),
            Some(&self.struct_field_types),
            Some(&self.self_receiver_upgrades),
        )
    }

    /// Resolved self-receiver ownership for a method, including codegen upgrades recorded
    /// while emitting earlier methods in the same compilation unit.
    pub(super) fn effective_method_self_ownership(
        &self,
        qualified: &str,
        sig: &crate::analyzer::FunctionSignature,
    ) -> crate::analyzer::OwnershipMode {
        if let Some(upgraded) = self.self_receiver_upgrades.get(qualified) {
            return *upgraded;
        }
        sig.param_ownership
            .first()
            .copied()
            .unwrap_or(crate::analyzer::OwnershipMode::Borrowed)
    }

    pub(super) fn method_requires_consuming_self_receiver(
        &self,
        qualified: &str,
        sig: &crate::analyzer::FunctionSignature,
    ) -> bool {
        match self.effective_method_self_ownership(qualified, sig) {
            crate::analyzer::OwnershipMode::Owned => true,
            crate::analyzer::OwnershipMode::MutBorrowed => {
                sig.return_type.as_ref().is_some_and(|t| match t {
                    Type::Custom(name) => self
                        .current_struct_name
                        .as_ref()
                        .is_some_and(|sn| sn == name),
                    _ => false,
                })
            }
            _ => false,
        }
    }

    /// Copy `self.field` reads (`pos_x: f32` into `Vec3::new`) are not partial moves.
    // Missing field types stay moves so non-Copy ctor moves keep `mut self`.
    pub(super) fn self_field_partial_move_needs_owned_receiver(&self, func: &FunctionDecl) -> bool {
        let struct_name = self.current_struct_name.clone();
        super::self_analysis::function_partial_moves_self_field_then_assigns_other_if(
            func,
            |field| {
                let Some(sn) = struct_name.as_deref() else {
                    return true;
                };
                let Some(fields) = self.lookup_struct_field_types(sn) else {
                    return true;
                };
                match fields.get(field) {
                    Some(ty) => !self.is_type_copy(ty),
                    None => true,
                }
            },
        )
    }


    /// Check if the struct currently being implemented is a Copy type.
    /// For Copy types, `self` (by value) is preferred over `&self` since
    /// the copy is trivially cheap and avoids unnecessary indirection.
    pub(super) fn current_struct_is_copy(&self) -> bool {
        if let Some(ref name) = self.current_struct_name {
            self.is_type_copy(&Type::Custom(name.clone()))
        } else {
            false
        }
    }

    pub(super) fn function_returns_self_type(&self, func: &FunctionDecl) -> bool {
        // Check if the function returns Self (for builder pattern detection)
        use crate::parser::{Expression, Statement, Type};

        // First check if return type is a custom type (struct type)
        let returns_custom_type = matches!(&func.return_type, Some(Type::Custom(_)));

        if !returns_custom_type {
            return false;
        }

        // Now check if the function body actually returns `self`
        // Check the last statement in the body
        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Explicit return self
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                Statement::Expression { expr, .. } => {
                    // Implicit return self (last expression)
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub(super) fn function_modifies_self(&self, func: &FunctionDecl) -> bool {
        super::self_analysis::function_modifies_self(
            func,
            Some(&self.signature_registry),
            self.current_struct_name.as_deref(),
            Some(&self.struct_field_types),
            Some(&self.self_receiver_upgrades),
        )
    }

    /// Record when codegen upgrades a self receiver beyond what the analyzer inferred.
    /// For example, when analyzer says Borrowed but body analysis detects mutation,
    /// codegen generates &mut self. This must be recorded so metadata reflects the
    /// actual generated code for cross-file builds.
    pub(super) fn record_self_receiver_upgrade(
        &mut self,
        func_name: &str,
        analyzer_ownership: Option<OwnershipMode>,
        actual_receiver: &str,
    ) {
        // `mut self` is by-value owned (builder / partial move) — not `&mut self`.
        // Collapsing it into MutBorrowed made the upgrade table force `&mut self` on
        // re-emit and broke owned field moves (P3.584).
        let actual_mode = match actual_receiver {
            "&mut self" => OwnershipMode::MutBorrowed,
            "&self" => OwnershipMode::Borrowed,
            "self" | "mut self" => OwnershipMode::Owned,
            _ => return,
        };

        let _analyzer_mode = analyzer_ownership.unwrap_or(OwnershipMode::Borrowed);
        if let Some(struct_name) = &self.current_struct_name {
            let qualified = format!("{}::{}", struct_name, func_name);
            self.self_receiver_upgrades.insert(qualified, actual_mode);
        }
    }

    pub(super) fn function_calls_self_with_recorded_receiver(
        &self,
        func: &FunctionDecl,
        mode: OwnershipMode,
    ) -> bool {
        let Some(struct_name) = self.current_struct_name.as_ref() else {
            return false;
        };
        func.body.iter().any(|stmt| {
            self.statement_calls_self_method_with_recorded_receiver(stmt, struct_name, mode)
        })
    }

    fn statement_calls_self_method_with_recorded_receiver(
        &self,
        stmt: &Statement,
        struct_name: &str,
        mode: OwnershipMode,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_calls_self_method_with_recorded_receiver(expr, struct_name, mode)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_calls_self_method_with_recorded_receiver(expr, struct_name, mode),
            Statement::Let { value, .. } => {
                self.expression_calls_self_method_with_recorded_receiver(value, struct_name, mode)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_calls_self_method_with_recorded_receiver(
                    condition,
                    struct_name,
                    mode,
                ) || then_block.iter().any(|s| {
                    self.statement_calls_self_method_with_recorded_receiver(s, struct_name, mode)
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.statement_calls_self_method_with_recorded_receiver(
                            s,
                            struct_name,
                            mode,
                        )
                    })
                })
            }
            Statement::For { body, .. } => body.iter().any(|s| {
                self.statement_calls_self_method_with_recorded_receiver(s, struct_name, mode)
            }),
            Statement::While {
                condition, body, ..
            } => {
                self.expression_calls_self_method_with_recorded_receiver(
                    condition,
                    struct_name,
                    mode,
                ) || body.iter().any(|s| {
                    self.statement_calls_self_method_with_recorded_receiver(s, struct_name, mode)
                })
            }
            Statement::Match { value, arms, .. } => {
                self.expression_calls_self_method_with_recorded_receiver(value, struct_name, mode)
                    || arms.iter().any(|arm| {
                        if let Expression::Block { statements, .. } = arm.body {
                            statements.iter().any(|s| {
                                self.statement_calls_self_method_with_recorded_receiver(
                                    s,
                                    struct_name,
                                    mode,
                                )
                            })
                        } else {
                            self.expression_calls_self_method_with_recorded_receiver(
                                arm.body,
                                struct_name,
                                mode,
                            )
                        }
                    })
            }
            _ => false,
        }
    }

    fn expression_calls_self_method_with_recorded_receiver(
        &self,
        expr: &Expression,
        struct_name: &str,
        mode: OwnershipMode,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } if matches!(&**object, Expression::Identifier { name, .. } if name == "self") =>
            {
                let qualified = format!("{struct_name}::{method}");
                if self
                    .self_receiver_upgrades
                    .get(&qualified)
                    .is_some_and(|m| *m == mode)
                {
                    return true;
                }
                if let Some(sig) = self.get_signature_with_global(&qualified) {
                    return sig.has_self_receiver
                        && sig.param_ownership.first().is_some_and(|m| *m == mode);
                }
                false
            }
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.statement_calls_self_method_with_recorded_receiver(s, struct_name, mode)
            }),
            Expression::Binary { left, right, .. } => {
                self.expression_calls_self_method_with_recorded_receiver(left, struct_name, mode)
                    || self.expression_calls_self_method_with_recorded_receiver(
                        right,
                        struct_name,
                        mode,
                    )
            }
            Expression::Unary { operand, .. } => {
                self.expression_calls_self_method_with_recorded_receiver(operand, struct_name, mode)
            }
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expression_calls_self_method_with_recorded_receiver(arg, struct_name, mode)
            }),
            _ => false,
        }
    }

    /// E0053 FIX: Get effective self ownership - trait's when in trait impl, else analyzed's.
    /// Impl methods MUST match trait signature exactly.
    ///
    /// The trait codegen defaults to `&mut self` for most methods (unless analysis
    /// explicitly found `Borrowed`). The impl must match what the TRAIT generated.
    /// When we can't determine the trait's choice, default to `&mut self` (most permissive).
    pub(super) fn get_effective_self_ownership(
        &self,
        func_name: &str,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> Option<OwnershipMode> {
        if self.in_trait_impl {
            if let Some(trait_name) = &self.current_trait_impl_name {
                let methods = self.analyzed_trait_methods.get(trait_name).or_else(|| {
                    trait_name
                        .rfind("::")
                        .map(|i| &trait_name[i + 2..])
                        .and_then(|key| self.analyzed_trait_methods.get(key))
                });
                if let Some(trait_method) = methods.and_then(|m| m.get(func_name)) {
                    // The trait's declared receiver is the contract (W0016).
                    match trait_method.decl.parameters.first().and_then(declared_receiver) {
                        Some("&self") => return Some(OwnershipMode::Borrowed),
                        Some(_) => return Some(OwnershipMode::MutBorrowed),
                        None => {}
                    }
                    if let Some(ownership)= trait_method.inferred_ownership.get("self").copied() {
                        return Some(ownership);
                    }
                    if let Some(reg_own) =
                        self.lookup_trait_method_ownership_in_registry(trait_name, func_name)
                    {
                        return Some(reg_own);
                    }
                    // Abstract trait method with no inferred receiver yet — default &self.
                    return Some(OwnershipMode::Borrowed);
                }
                // Trait exists but this specific method wasn't in the analysis map.
                // Trait codegen defaults to &self for unanalyzed methods, so match that.
                if methods.is_some() {
                    return Some(OwnershipMode::Borrowed);
                }
                // Cross-file trait impl: trait not in registry at all (single-file compilation).
                // Choose self ownership based on known trait conventions:
                // - Operator traits (Add, Sub, etc.) require consuming self
                // - Derive traits (Display, Debug, etc.) require &self
                // - Custom traits: check signature registry first, then body analysis
                let base_trait = trait_name
                    .rfind("::")
                    .map(|i| &trait_name[i + 2..])
                    .unwrap_or(trait_name);
                return Some(
                    if crate::type_classification::is_owned_self_trait(base_trait) {
                        OwnershipMode::Owned
                    } else if crate::type_classification::is_ref_receiver_trait(base_trait) {
                        OwnershipMode::Borrowed
                    } else {
                        // Check signature registry for the trait method's self ownership.
                        // This handles cross-file trait impls where metadata was loaded.
                        let registry_ownership =
                            self.lookup_trait_method_ownership_in_registry(trait_name, func_name);
                        if let Some(reg_own) = registry_ownership {
                            reg_own
                        } else if let Some(body_ownership) =
                            self.get_param_ownership("self", analyzed)
                        {
                            body_ownership
                        } else {
                            if analyzed.decl.return_type.is_some() {
                                OwnershipMode::Borrowed
                            } else {
                                OwnershipMode::MutBorrowed
                            }
                        }
                    },
                );
            }
        }
        self.get_param_ownership("self", analyzed)
    }

    /// Look up a trait method's self-ownership in the signature registry.
    /// Checks patterns like "TraitName::method", "module::TraitName::method", etc.
    fn lookup_trait_method_ownership_in_registry(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Option<OwnershipMode> {
        let base_trait = trait_name
            .rfind("::")
            .map(|i| &trait_name[i + 2..])
            .unwrap_or(trait_name);

        // Try various qualified name patterns
        let patterns = [
            format!("{}::{}", base_trait, method_name),
            format!("{}::{}", trait_name, method_name),
        ];

        for pattern in &patterns {
            if let Some(sig) = self.get_signature_with_global(pattern) {
                if sig.has_self_receiver {
                    if let Some(&ownership) = sig.param_ownership.first() {
                        return Some(ownership);
                    }
                }
            }
        }

        // Also try suffix matching for fully qualified paths
        let suffix = format!("{}::{}", base_trait, method_name);
        for (key, sig) in &self.signature_registry.signatures {
            if key.ends_with(&suffix) && sig.has_self_receiver {
                if let Some(&ownership) = sig.param_ownership.first() {
                    return Some(ownership);
                }
            }
        }

        None
    }

    /// WINDJAMMER LIFETIME INFERENCE: Determine if a function needs explicit lifetime annotations.
    ///
    /// Rust's lifetime elision rules handle most cases:
    ///   1. Single input reference → output gets that lifetime
    ///   2. &self/&mut self → output gets self's lifetime
    ///   3. Multiple input references with no self → MUST be explicit
    ///
    /// We only add 'a when case 3 applies AND the return type contains references.
    pub(super) fn function_needs_lifetime_annotations(
        &self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> bool {
        use crate::codegen::rust::types::type_contains_reference;

        // First check: does the return type contain any references?
        let return_has_ref = match &func.return_type {
            Some(ret_type) => type_contains_reference(ret_type),
            None => false,
        };

        if !return_has_ref {
            return false;
        }

        // Check if there's a self parameter (explicit or inferred)
        let has_self = func.parameters.iter().any(|p| p.name == "self")
            || self.has_param_ownership("self", analyzed);

        if has_self {
            // &self/&mut self methods: Rust elision rule 2 handles this
            return false;
        }

        // Count the number of reference parameters (explicit refs + analyzer-inferred refs)
        let ref_param_count = func
            .parameters
            .iter()
            .enumerate()
            .filter(|(param_idx, param)| {
                if param.name == "self" {
                    return false;
                }

                // Check if the parameter type is already a reference
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(*param_idx)
                    .unwrap_or(&param.type_);

                if matches!(
                    inferred_type,
                    Type::Reference(_) | Type::MutableReference(_)
                ) {
                    return true;
                }

                // Check explicit ownership hints
                if matches!(
                    param.ownership,
                    crate::parser::OwnershipHint::Ref | crate::parser::OwnershipHint::Mut
                ) {
                    return true;
                }

                // Check analyzer-inferred ownership
                if let Some(ownership) = self.get_param_ownership(&param.name, analyzed) {
                    matches!(
                        ownership,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    )
                } else {
                    false
                }
            })
            .count();

        // Need explicit lifetime when 2+ reference params and reference return
        ref_param_count >= 2
    }

    /// OPTIMIZATION: Determine if a function should be marked #[inline]
    /// Phase 1: Generate Inlinable Code
    ///
    /// Heuristics for inlining:
    /// 1. Module functions (stdlib wrappers) - always inline for zero-cost abstraction
    /// 2. Small functions (< 10 statements) - likely to benefit from inlining
    /// 3. Trivial getters/setters - always inline
    /// 4. Functions with only one return statement - simple enough to inline
    /// 5. Don't inline: main(), test functions, async functions, large functions
    pub(super) fn should_inline_function(
        &self,
        func: &FunctionDecl,
        _analyzed: &AnalyzedFunction,
    ) -> bool {
        // Never inline main
        if func.name == "main" {
            return false;
        }

        // Never inline test functions
        if func.decorators.iter().any(|d| d.name == "test") {
            return false;
        }

        // Don't inline async functions (they're already state machines)
        if func.decorators.iter().any(|d| d.name == "async") {
            return false;
        }

        // ALWAYS inline module functions (stdlib wrappers)
        // These are thin wrappers around Rust stdlib and should have zero overhead
        if self.is_module {
            return true;
        }

        // Count statements in function body
        let statement_count = ast_utilities::count_statements(&func.body);

        // Inline small functions (< 10 statements)
        if statement_count < 10 {
            return true;
        }

        // Inline trivial single-expression functions
        if statement_count == 1 {
            if let Statement::Return { value: Some(_), .. } = &func.body[0] {
                return true;
            }
            if let Statement::Expression { .. } = &func.body[0] {
                return true;
            }
        }

        // Default: don't inline large functions
        false
    }
}
