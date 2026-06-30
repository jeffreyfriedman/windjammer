//! Per-function analysis: inherent and trait impl bodies, signatures for codegen.

use std::collections::{HashMap, HashSet};

use crate::auto_clone::AutoCloneAnalysis;
use crate::parser::*;

use super::{
    AnalyzedFunction, Analyzer, FunctionSignature, ImplSelfFieldContext, OwnershipMode,
    SignatureRegistry,
};

impl<'ast> Analyzer<'ast> {
    /// All methods for `type_name` across every `impl` block in the program (including inherent + trait impls).
    /// Used so `self.helper()` in `impl Trait for T` resolves `helper` from `impl T` on the same type.
    pub(crate) fn merged_impl_methods_for_type(
        program: &Program<'ast>,
        type_name: &str,
    ) -> HashMap<String, FunctionDecl<'ast>> {
        let type_base = type_name.split('<').next().unwrap_or(type_name);
        let mut merged = HashMap::new();
        Self::collect_impl_methods_recursive(&program.items, type_base, &mut merged);
        merged
    }

    pub(crate) fn collect_impl_methods_recursive(
        items: &[Item<'ast>],
        type_base: &str,
        merged: &mut HashMap<String, FunctionDecl<'ast>>,
    ) {
        for item in items {
            match item {
                Item::Impl { block, .. } => {
                    let block_base = block
                        .type_name
                        .split('<')
                        .next()
                        .unwrap_or(&block.type_name);
                    if block_base == type_base {
                        for f in &block.functions {
                            merged.insert(f.name.clone(), f.clone());
                        }
                    }
                }
                Item::Mod { items: inner, .. } => {
                    Self::collect_impl_methods_recursive(inner, type_base, merged);
                }
                _ => {}
            }
        }
    }

    /// Analyze a function within an impl block (has access to other methods for cross-method analysis)
    pub(crate) fn analyze_function_in_impl(
        &mut self,
        func: &FunctionDecl<'ast>,
        impl_block: &crate::parser::ast::ImplBlock<'ast>,
        program: &Program<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        // Same-type impl merge: trait impl methods can call inherent helpers on the same type.
        self.current_impl_functions = Some(Self::merged_impl_methods_for_type(
            program,
            &impl_block.type_name,
        ));
        let impl_base = impl_block
            .type_name
            .split('<')
            .next()
            .unwrap_or(impl_block.type_name.as_str())
            .to_string();
        self.self_impl_context = Some(ImplSelfFieldContext::new(impl_base, program));
        let mut analyzed = self.analyze_function(func, registry)?;

        // Inherent impls: `for x in self.field` + `x.foo()` where `foo` is `&mut self` on a trait object
        // requires `&mut self` on the outer method (codegen emits `&mut self.field`).
        if impl_block.trait_name.is_none() {
            self.maybe_upgrade_self_for_dispatch_for_loops(
                &mut analyzed,
                func,
                impl_block.type_name.as_str(),
                program,
                registry,
            );
        }

        // Clear impl block after analysis
        self.self_impl_context = None;
        self.current_impl_functions = None;

        Ok(analyzed)
    }

    /// Infer `self` receiver ownership for impl methods. Windjammer always writes bare
    /// `self` in source; codegen maps to `&self`, `&mut self`, or owned `self`.
    pub(super) fn infer_impl_self_receiver_ownership(
        &self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
    ) -> OwnershipMode {
        let mut visited = HashSet::new();
        self.infer_impl_self_receiver_ownership_inner(func, registry, &mut visited)
    }

    /// Inner inference with cycle detection (`a` → `b` → `a` must not recurse infinitely).
    pub(super) fn infer_impl_self_receiver_ownership_inner(
        &self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
        visited: &mut HashSet<String>,
    ) -> OwnershipMode {
        if !visited.insert(func.name.clone()) {
            return OwnershipMode::Borrowed;
        }

        // Multiple recursive `self.method(...)` calls cannot take owned `self`.
        // But read-only recursion should get `&self`, not `&mut self`.
        // Let the normal analysis run and only prevent Owned at the end.
        let has_multi_recursive = self.count_self_method_calls(func, func.name.as_str()) >= 2;

        if func.is_extern && func.body.is_empty() && func.parent_type.is_some() {
            return OwnershipMode::MutBorrowed;
        }

        let modifies_fields =
            self.function_modifies_self_fields_with_registry(func, Some(registry));
        let returns_self = self.function_returns_self(func);
        let body_moves_fields = self.function_body_moves_non_copy_self_fields(func);
        let snapshot_factory = self.function_returns_new_instance_from_self_fields(func);

        let consumes_self = if has_multi_recursive {
            false
        } else {
            body_moves_fields
                || self.function_moves_self_into_return(func)
                || (!snapshot_factory
                    && (returns_self
                        || self.function_body_consumes_bare_self(func)
                        || self.function_calls_consuming_method_on_self_with_visited(
                            func, registry, visited,
                        )
                        || self.function_calls_explicit_owned_self_method(func, registry)
                        || self.function_matches_on_self(func)
                        || self.function_consumes_self_field_elements(func, Some(registry))))
        };
        let calls_owned_on_self = if has_multi_recursive {
            false
        } else {
            self.function_calls_consuming_method_on_self_with_visited(func, registry, visited)
                || self.function_calls_explicit_owned_self_method(func, registry)
        };
        let mut mutating_call_visited = HashSet::new();
        let calls_mutating = self.function_calls_mutating_self_methods_with_registry(
            func,
            Some(registry),
            &mut mutating_call_visited,
        );
        self.resolve_impl_self_receiver_ownership(
            func,
            modifies_fields,
            returns_self,
            body_moves_fields,
            consumes_self,
            calls_mutating,
            calls_owned_on_self,
        )
    }

    fn resolve_impl_self_receiver_ownership(
        &self,
        func: &FunctionDecl<'ast>,
        modifies_fields: bool,
        returns_self: bool,
        body_moves_fields: bool,
        consumes_self: bool,
        calls_mutating: bool,
        calls_owned_on_self: bool,
    ) -> OwnershipMode {
        // In-place mutation without returning/consuming self → &mut self, not owned self.
        // Dogfooding: RenderPort::render_frame mutates fields but must not take owned self.
        // Skip when the method calls another method with owned `self` (e.g. evaluate → evaluate_node).
        if (modifies_fields || calls_mutating)
            && !returns_self
            && !body_moves_fields
            && !self.function_moves_self_into_return(func)
            && !calls_owned_on_self
        {
            return OwnershipMode::MutBorrowed;
        }

        if consumes_self {
            OwnershipMode::Owned
        } else if modifies_fields || calls_mutating {
            OwnershipMode::MutBorrowed
        } else if self.is_used_in_binary_op("self", &func.body) {
            OwnershipMode::Owned
        } else {
            OwnershipMode::Borrowed
        }
    }

    pub(crate) fn analyze_function(
        &mut self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        let mut inferred_ownership = HashMap::new();

        // Check if this is a game decorator function
        let is_game_decorator = func.decorators.iter().any(|d| {
            matches!(
                d.name.as_str(),
                "init" | "update" | "render" | "render3d" | "input" | "cleanup"
            )
        });
        let is_render3d = func.decorators.iter().any(|d| d.name == "render3d");

        // THE WINDJAMMER WAY: Auto-Self Inference
        // If a method uses `self` in its body but doesn't declare it as a parameter,
        // automatically infer and add it.
        let declares_self = func.parameters.iter().any(|p| p.name == "self");
        let uses_self = self.function_uses_identifier("self", &func.body);

        if uses_self && !declares_self {
            // Auto-infer self ownership based on usage
            let modifies_fields =
                self.function_modifies_self_fields_with_registry(func, Some(registry));
            let returns_self = self.function_returns_self(func);
            let body_moves_fields = self.function_body_moves_non_copy_self_fields(func);
            let snapshot_factory = self.function_returns_new_instance_from_self_fields(func);

            let consumes_self = body_moves_fields
                || self.function_moves_self_into_return(func)
                || (!snapshot_factory
                    && (returns_self
                        || self.function_body_consumes_bare_self(func)
                        || self.function_calls_consuming_method_on_self(func, registry)
                        || self.function_calls_explicit_owned_self_method(func, registry)
                        || self.function_matches_on_self(func)
                        || self.function_consumes_self_field_elements(func, Some(registry))));
            let mut mutating_call_visited = HashSet::new();
            let calls_mutating = self.function_calls_mutating_self_methods_with_registry(
                func,
                Some(registry),
                &mut mutating_call_visited,
            );
            let calls_owned_on_self = self.function_calls_consuming_method_on_self(func, registry)
                || self.function_calls_explicit_owned_self_method(func, registry);
            let self_ownership = self.resolve_impl_self_receiver_ownership(
                func,
                modifies_fields,
                returns_self,
                body_moves_fields,
                consumes_self,
                calls_mutating,
                calls_owned_on_self,
            );

            // Store inferred self ownership
            inferred_ownership.insert("self".to_string(), self_ownership);
        }

        // Analyze each parameter to infer ownership mode
        for (i, param) in func.parameters.iter().enumerate() {
            let mode = match param.ownership {
                OwnershipHint::Owned => {
                    // Windjammer writes `self` without & — in impl methods, infer receiver
                    // ownership from body usage (distance(self) → &self, not owned self).
                    if param.name == "self" && func.parent_type.is_some() {
                        self.infer_impl_self_receiver_ownership(func, registry)
                    } else {
                        OwnershipMode::Owned
                    }
                }
                OwnershipHint::Mut => {
                    if Self::is_generic_type_param(&param.type_) {
                        OwnershipMode::Owned
                    } else {
                        OwnershipMode::MutBorrowed
                    }
                }
                OwnershipHint::Ref => {
                    // SMART FIX: If user wrote &self but function modifies fields, upgrade to &mut self
                    // This prevents a common user error
                    if param.name == "self"
                        && self.function_modifies_self_fields_with_registry(func, Some(registry))
                    {
                        OwnershipMode::MutBorrowed
                    } else {
                        OwnershipMode::Borrowed
                    }
                }
                OwnershipHint::Inferred => {
                    // Special case: Game decorator functions always take &mut for first parameter (game state)
                    if is_game_decorator && i == 0 {
                        OwnershipMode::MutBorrowed
                    } else if is_render3d && i == 2 {
                        // Special case: @render3d functions take &mut for camera parameter (3rd param)
                        OwnershipMode::MutBorrowed
                    } else if param.name == "self" {
                        if func.parent_type.is_some() {
                            self.infer_impl_self_receiver_ownership(func, registry)
                        } else if func.is_extern && func.body.is_empty() {
                            OwnershipMode::MutBorrowed
                        } else {
                            let modifies_fields = self
                                .function_modifies_self_fields_with_registry(func, Some(registry));
                            let returns_self = self.function_returns_self(func);
                            let body_moves_fields =
                                self.function_body_moves_non_copy_self_fields(func);
                            let snapshot_factory =
                                self.function_returns_new_instance_from_self_fields(func);

                            let consumes_self = body_moves_fields
                                || self.function_moves_self_into_return(func)
                                || (!snapshot_factory
                                    && (returns_self
                                        || self.function_body_consumes_bare_self(func)
                                        || self.function_calls_consuming_method_on_self(
                                            func, registry,
                                        )
                                        || self.function_calls_explicit_owned_self_method(
                                            func, registry,
                                        )
                                        || self.function_matches_on_self(func)
                                        || self.function_consumes_self_field_elements(
                                            func,
                                            Some(registry),
                                        )));
                            if consumes_self {
                                OwnershipMode::Owned
                            } else if modifies_fields {
                                OwnershipMode::MutBorrowed
                            } else if self.is_used_in_binary_op("self", &func.body) {
                                OwnershipMode::Owned
                            } else {
                                OwnershipMode::Borrowed
                            }
                        }
                    } else {
                        // For Copy types, check if they're mutated first
                        // Mutated Copy types should be &mut, not Owned
                        let is_copy = self.is_copy_type(&param.type_);

                        if param.is_mutable && !is_copy {
                            OwnershipMode::Owned
                        } else if param.is_mutable && is_copy {
                            OwnershipMode::MutBorrowed
                        } else if is_copy {
                            let mutated = self.is_mutated(
                                &param.name,
                                &func.body,
                                registry,
                                Some(&param.type_),
                            );
                            let passthrough_mut = matches!(
                                self.infer_passthrough_ownership(
                                    &param.name,
                                    &param.type_,
                                    &func.body,
                                    registry,
                                    &func.name,
                                    func,
                                ),
                                Some(OwnershipMode::MutBorrowed)
                            );
                            if std::env::var("WJ_DEBUG_OWNERSHIP").is_ok() {
                                eprintln!(
                                    "  [OWNERSHIP-COPY] {} in {}: is_copy=true mutated={} passthrough_mut={} (type: {:?})",
                                    param.name, func.name, mutated, passthrough_mut, param.type_
                                );
                            }
                            if mutated || passthrough_mut {
                                OwnershipMode::MutBorrowed
                            } else {
                                self.infer_parameter_ownership(
                                    &param.name,
                                    &param.type_,
                                    &func.body,
                                    &func.return_type,
                                    registry,
                                    &func.name,
                                    func,
                                )?
                            }
                        } else {
                            // Perform inference based on usage in function body
                            let inferred_mode = self.infer_parameter_ownership(
                                &param.name,
                                &param.type_,
                                &func.body,
                                &func.return_type,
                                registry,
                                &func.name,
                                func,
                            )?;

                            // DEBUG: Log ownership inference for non-Copy parameters
                            if std::env::var("WJ_DEBUG_OWNERSHIP").is_ok() {
                                eprintln!(
                                    "  [OWNERSHIP] {} in {}: {:?} (type: {:?})",
                                    param.name, func.name, inferred_mode, param.type_
                                );
                            }

                            inferred_mode
                        }
                    }
                }
            };

            inferred_ownership.insert(param.name.clone(), mode);
        }

        // During multipass convergence, skip expensive optimization detectors.
        // Only ownership inference matters for convergence; codegen-only
        // optimizations run in the final pass.
        let (
            clone_optimizations,
            struct_mapping_optimizations,
            string_optimizations,
            assignment_optimizations,
            defer_drop_optimizations,
            auto_clone_analysis,
            mutated_variables,
            mutated_parameters,
            const_static_optimizations,
            smallvec_optimizations,
            cow_optimizations,
            cache_locality,
            str_ref_optimizable_params,
            inferred_param_types,
        ) = if self.convergence_only {
            // str_ref analysis is cheap and affects signatures (param_types change
            // from String to &str), so it MUST run during convergence.
            // Only skip the truly expensive codegen-only optimizations.
            let str_ref_optimizable_params =
                self.analyze_str_ref_optimizable_params(func, registry);
            let mut str_ref_optimizable_params = str_ref_optimizable_params;
            str_ref_optimizable_params
                .retain(|name| inferred_ownership.get(name) != Some(&OwnershipMode::Owned));
            let inferred_param_types: Vec<Type> = func
                .parameters
                .iter()
                .map(|param| {
                    if str_ref_optimizable_params.contains(&param.name) {
                        Type::Reference(Box::new(Type::Custom("str".to_string())))
                    } else {
                        param.type_.clone()
                    }
                })
                .collect();
            (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                AutoCloneAnalysis::default(),
                HashSet::new(),
                HashSet::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                super::CacheLocalityAnalysis::default(),
                str_ref_optimizable_params,
                inferred_param_types,
            )
        } else {
            let clone_optimizations = self.detect_unnecessary_clones(func);
            let struct_mapping_optimizations = self.detect_struct_mappings(func);
            let string_optimizations = self.detect_string_optimizations(func);
            let assignment_optimizations = self.detect_assignment_optimizations(func);
            let defer_drop_optimizations = self.detect_defer_drop_opportunities(func, registry);
            let auto_clone_analysis = AutoCloneAnalysis::analyze_function(func);

            self.track_mutations(&func.body, registry);
            let mutated_variables = self.mutated_variables.clone();

            let mut mutated_parameters = HashSet::new();
            for param in &func.parameters {
                if self.is_mutated(&param.name, &func.body, registry, Some(&param.type_)) {
                    mutated_parameters.insert(param.name.clone());
                }
            }

            let const_static_optimizations = Vec::new();
            let smallvec_optimizations = Vec::new();
            let cow_optimizations = Vec::new();
            let cache_locality = super::CacheLocalityAnalysis::default();
            let str_ref_optimizable_params =
                self.analyze_str_ref_optimizable_params(func, registry);
            let mut str_ref_optimizable_params = str_ref_optimizable_params;
            str_ref_optimizable_params
                .retain(|name| inferred_ownership.get(name) != Some(&OwnershipMode::Owned));

            let inferred_param_types: Vec<Type> = func
                .parameters
                .iter()
                .map(|param| {
                    if str_ref_optimizable_params.contains(&param.name) {
                        Type::Reference(Box::new(Type::Custom("str".to_string())))
                    } else {
                        param.type_.clone()
                    }
                })
                .collect();

            (
                clone_optimizations,
                struct_mapping_optimizations,
                string_optimizations,
                assignment_optimizations,
                defer_drop_optimizations,
                auto_clone_analysis,
                mutated_variables,
                mutated_parameters,
                const_static_optimizations,
                smallvec_optimizations,
                cow_optimizations,
                cache_locality,
                str_ref_optimizable_params,
                inferred_param_types,
            )
        };

        Ok(AnalyzedFunction {
            decl: func.clone(),
            inferred_ownership,
            inferred_param_types,
            mutated_variables,
            mutated_parameters,
            auto_clone_analysis,
            clone_optimizations,
            struct_mapping_optimizations,
            string_optimizations,
            assignment_optimizations,
            defer_drop_optimizations,
            const_static_optimizations,
            smallvec_optimizations,
            cow_optimizations,
            cache_locality,
            str_ref_optimizable_params,
        })
    }

    /// Analyze a function that implements a trait method
    /// Use the trait's method signature instead of inferring
    pub(crate) fn analyze_trait_impl_function(
        &mut self,
        func: &FunctionDecl<'ast>,
        trait_name: &str,
        impl_block: &crate::parser::ast::ImplBlock<'ast>,
        program: &Program<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        // Trait impl bodies may call `self.inherent_helper()` from `impl Type` — merge those decls.
        self.current_impl_functions = Some(Self::merged_impl_methods_for_type(
            program,
            &impl_block.type_name,
        ));
        let impl_base = impl_block
            .type_name
            .split('<')
            .next()
            .unwrap_or(impl_block.type_name.as_str())
            .to_string();
        self.self_impl_context = Some(ImplSelfFieldContext::new(impl_base, program));
        let analyzed_base = self.analyze_function(func, registry);
        self.self_impl_context = None;
        self.current_impl_functions = None;
        let mut analyzed = analyzed_base?;

        // Look up the trait definition
        // Try both the full trait name and just the last segment (e.g., "std::ops::Add" -> "Add")
        let trait_key = if let Some(pos) = trait_name.rfind("::") {
            &trait_name[pos + 2..]
        } else {
            trait_name
        };

        let is_std_operator_trait =
            crate::type_classification::is_consuming_operator_trait(trait_key);

        // For standard operator traits, use `self` (owned) instead of `&self`
        if is_std_operator_trait {
            // Standard operator trait (Add, Sub, Mul, etc.) - not defined in Windjammer stdlib
            // These traits use `self` (owned) for the first parameter (self), not `&self`
            // Example: `fn add(self, rhs: Rhs) -> Output`

            // For the first parameter (self), use Owned for Copy types
            if let Some(first_param) = func.parameters.first() {
                if first_param.name == "self" {
                    // Use Owned (self) for operator traits on Copy types
                    analyzed
                        .inferred_ownership
                        .insert("self".to_string(), OwnershipMode::Owned);
                }
            }
        } else if let Some(trait_decl) = self.trait_definitions.get(trait_key) {
            // Defer mutable trait registry updates until after this immutable borrow ends.
            let mut self_receiver_upgrades: Vec<(String, String, String, OwnershipMode)> =
                Vec::new();
            // Find the matching trait method
            if let Some(trait_method) = trait_decl.methods.iter().find(|m| m.name == func.name) {
                // Override ALL parameters to match trait signature
                // Trait implementations must match the trait's exact signature
                // Match by POSITION, not by name (trait uses "rhs", impl might use "other")
                for (i, trait_param) in trait_method.parameters.iter().enumerate() {
                    // Get the corresponding parameter from the implementation by position
                    if let Some(impl_param) = func.parameters.get(i) {
                        // WINDJAMMER PHILOSOPHY: Use ANALYZED trait method ownership, not AST ownership!
                        // The AST might have `self` (Owned) but analysis infers `&self` (Borrowed).
                        // Check if this trait method was analyzed (has default implementation)
                        let trait_methods_opt = self
                            .analyzed_trait_methods
                            .get(trait_key)
                            .or_else(|| self.analyzed_trait_methods.get(trait_name));
                        let trait_mode = if let Some(trait_methods) = trait_methods_opt {
                            if let Some(analyzed_trait_method) = trait_methods.get(&func.name) {
                                analyzed_trait_method
                                    .inferred_ownership
                                    .get(&trait_param.name)
                                    .copied()
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let impl_body_mode =
                            analyzed.inferred_ownership.get(&impl_param.name).copied();
                        let final_mode = if impl_param.name == "self" {
                            match (trait_mode, impl_body_mode) {
                                (Some(trait_m), Some(impl_m)) => {
                                    Self::merge_borrow_trait_receivers(trait_m, impl_m)
                                }
                                (Some(trait_m), None) => trait_m,
                                (None, Some(impl_m)) => impl_m,
                                (None, None) => self.convert_ownership_hint_to_mode(
                                    &trait_param.ownership,
                                    &trait_param.name,
                                ),
                            }
                        } else if let Some(mode) = trait_mode {
                            // E0053: `fn foo(self, key: string)` in the trait item is owned `String`
                            // in Rust — merged borrow hints from read-only impl bodies must not
                            // downgrade to `&str` when the trait declaration uses plain `string`.
                            if Self::trait_param_is_owned_string(&trait_param.type_) {
                                OwnershipMode::Owned
                            } else {
                                mode
                            }
                        } else {
                            // E0053: Non-self trait impl params must match the trait declaration,
                            // not the impl body's borrow inference (e.g. read-only `string` → Owned).
                            self.convert_ownership_hint_to_mode(
                                &trait_param.ownership,
                                &trait_param.name,
                            )
                        };

                        if impl_param.name == "self" {
                            if let Some(trait_m) = trait_mode {
                                if final_mode != trait_m {
                                    self_receiver_upgrades.push((
                                        trait_name.to_string(),
                                        trait_key.to_string(),
                                        func.name.clone(),
                                        final_mode,
                                    ));
                                }
                            }
                        }

                        // INSERT or UPDATE with the final ownership mode
                        analyzed
                            .inferred_ownership
                            .insert(impl_param.name.clone(), final_mode);
                    }
                }
            }
            for (upgrade_trait_name, upgrade_trait_key, method_name, receiver) in
                self_receiver_upgrades
            {
                self.upgrade_trait_method_self_receiver(
                    &upgrade_trait_name,
                    &upgrade_trait_key,
                    &method_name,
                    receiver,
                );
            }
        }

        // E0053: Parameter types in generated Rust must match the trait declaration (impls may
        // rename parameters or use incompatible aliases). Ownership already matches the trait above.
        if !is_std_operator_trait {
            let analyzed_trait_fn = self
                .analyzed_trait_methods
                .get(trait_key)
                .and_then(|m| m.get(&func.name))
                .or_else(|| {
                    self.analyzed_trait_methods
                        .get(trait_name)
                        .and_then(|m| m.get(&func.name))
                });
            if let Some(trait_decl) = self.trait_definitions.get(trait_key) {
                if let Some(trait_method) = trait_decl.methods.iter().find(|m| m.name == func.name)
                {
                    for (i, trait_param) in trait_method.parameters.iter().enumerate() {
                        if i >= analyzed.inferred_param_types.len() {
                            break;
                        }
                        if Self::trait_param_is_owned_string(&trait_param.type_) {
                            if let Some(impl_param) = func.parameters.get(i) {
                                analyzed
                                    .inferred_ownership
                                    .insert(impl_param.name.clone(), OwnershipMode::Owned);
                                analyzed.str_ref_optimizable_params.remove(&impl_param.name);
                            }
                            analyzed.inferred_param_types[i] = trait_param.type_.clone();
                        } else if let Some(trait_ty) =
                            analyzed_trait_fn.and_then(|tf| tf.inferred_param_types.get(i))
                        {
                            analyzed.inferred_param_types[i] = trait_ty.clone();
                        }
                    }
                }
            } else if let Some(trait_fn) = analyzed_trait_fn {
                for (i, trait_ty) in trait_fn.inferred_param_types.iter().enumerate() {
                    if i < analyzed.inferred_param_types.len() {
                        analyzed.inferred_param_types[i] = trait_ty.clone();
                    }
                }
            }
        }

        Ok(analyzed)
    }
    pub(crate) fn build_signature(
        &self,
        func: &AnalyzedFunction,
        registry: &SignatureRegistry,
    ) -> FunctionSignature {
        let param_ownership: Vec<OwnershipMode> = func
            .decl
            .parameters
            .iter()
            .map(|param| {
                // CRITICAL FIX: Check the actual type annotation FIRST
                // If parameter is explicitly declared as &T or &mut T, respect that
                use crate::parser::Type;
                match &param.type_ {
                    Type::Reference(_) => {
                        // Parameter is explicitly &T - must borrow
                        return OwnershipMode::Borrowed;
                    }
                    Type::MutableReference(_) => {
                        // Parameter is explicitly &mut T - must mut borrow
                        return OwnershipMode::MutBorrowed;
                    }
                    _ => {
                        // Not an explicit reference, use inference
                    }
                }

                if self.is_only_hashmap_lookup_key_param(&param.name, &func.decl.body, &func.decl) {
                    if self.is_copy_type(&param.type_) {
                        return OwnershipMode::Owned;
                    }
                    return OwnershipMode::Borrowed;
                }

                let inferred = func
                    .inferred_ownership
                    .get(&param.name)
                    .cloned()
                    .unwrap_or(OwnershipMode::Owned);

                // CRITICAL: Generic type parameters (like G in fn foo<G: Trait>(g: G))
                // should ALWAYS be Owned. The trait bound is on G, not on &G.
                // Adding & at call sites would break trait bounds.
                if Self::is_generic_type_param(&param.type_) {
                    return OwnershipMode::Owned;
                }

                // Copy types: usually owned, but respect MutBorrowed when the analyzer
                // determined the parameter is actually mutated. This ensures `fn increment(x: int)`
                // where `x = x + 1` generates `fn increment(x: &mut i64)` and the call site
                // auto-inserts `&mut`. Read-only Copy params stay Owned (pass by value).
                if self.is_copy_type(&param.type_) && inferred != OwnershipMode::MutBorrowed {
                    return OwnershipMode::Owned;
                }
                // THE WINDJAMMER WAY: The compiler infers ownership, not the user.
                // Non-Copy types follow the analyzer's inference:
                // - Borrowed: parameter is only read (default for read-only params)
                // - MutBorrowed: parameter is mutated
                // - Owned: parameter is consumed (returned, stored, iterated, etc.)
                //
                // Users write `data: Vec<f32>` and the compiler figures out whether
                // it should be `&Vec<f32>`, `&mut Vec<f32>`, or `Vec<f32>` in Rust.
                // This matches call sites where `&self.data` is naturally passed.
                inferred
            })
            .collect();

        // PHASE 2 STRING OPTIMIZATION: Use inferred parameter types when available
        // The analyzer determines which string parameters can be &str vs &String
        // based on how they're used in the function body.
        let mut param_types: Vec<Type> = func
            .decl
            .parameters
            .iter()
            .enumerate()
            .map(|(idx, param)| {
                func.inferred_param_types
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| param.type_.clone())
            })
            .collect();
        let explicit_self = func
            .decl
            .parameters
            .first()
            .is_some_and(|p| p.name == "self" || p.name == "mut self");

        // Omitted `self` in source (`fn touch() { self... }`): analyzer stores ownership under
        // "self" but decl.parameters has no receiver. SignatureRegistry must still expose
        // `has_self_receiver` + `param_ownership[0]` so cross-type calls (e.g. `.touch()`) resolve.
        let synthetic_self_receiver =
            func.inferred_ownership.contains_key("self") && !explicit_self;

        let mut param_ownership = param_ownership;
        if synthetic_self_receiver {
            let self_mode = func
                .inferred_ownership
                .get("self")
                .copied()
                .unwrap_or(OwnershipMode::Borrowed);
            param_ownership.insert(0, self_mode);
            let self_ty = func
                .decl
                .parent_type
                .as_ref()
                .map(|n| Type::Custom(n.clone()))
                .unwrap_or(Type::Custom("Self".to_string()));
            param_types.insert(0, self_ty);
        }

        let has_self_receiver = explicit_self || synthetic_self_receiver;

        // Phase 2: inferred_param_types may become `Reference`/`MutableReference` while
        // ownership inference still marks the parameter `Owned` (e.g. forwarding calls where
        // the body is only `Call(FieldAccess)` and ref analysis does not recurse).
        // Rust lowers these parameters as `&T` / `&mut T` — call-site helpers that key off
        // `param_ownership` must agree or we emit `.clone()` / `.to_string()` incorrectly.
        use crate::parser::Type as PType;
        for (idx, ty) in param_types.iter().enumerate() {
            let mode = match ty {
                PType::Reference(_) => Some(OwnershipMode::Borrowed),
                PType::MutableReference(_) => Some(OwnershipMode::MutBorrowed),
                _ => None,
            };
            if let Some(mode) = mode {
                if let Some(slot) = param_ownership.get_mut(idx) {
                    *slot = mode;
                }
            }
        }

        // Formal types: AST annotations before body-inferred Reference wrap (call-site vs formal).
        let mut formal_param_types: Vec<PType> = func
            .decl
            .parameters
            .iter()
            .map(|p| p.type_.clone())
            .collect();
        if synthetic_self_receiver {
            let self_ty = func
                .decl
                .parent_type
                .as_ref()
                .map(|n| PType::Custom(n.clone()))
                .unwrap_or(PType::Custom("Self".to_string()));
            formal_param_types.insert(0, self_ty);
        }

        // Module-level plain `string` formals stay owned `String` (pit-of-success / E0053).
        // Struct/enum impl methods keep body-inferred &str; trait items are handled in trait_analysis.
        // Body-inferred Borrowed (read-only concat params, passthrough wrappers) is preserved.
        if func.decl.parent_type.is_none() {
            for (idx, param) in func.decl.parameters.iter().enumerate() {
                if Self::trait_param_is_owned_string(&param.type_) {
                    if idx < formal_param_types.len() {
                        formal_param_types[idx] = param.type_.clone();
                    }
                    if idx < param_ownership.len()
                        && !matches!(
                            param_ownership[idx],
                            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                        )
                    {
                        param_ownership[idx] = OwnershipMode::Owned;
                    }
                }
            }
        }

        // Extern FFI string parameters are converted at the call site (string_to_ffi).
        // Registry ownership for passthrough must stay Borrowed so Windjammer wrappers
        // keep &str APIs and callers auto-borrow; codegen adds .to_string() at the boundary.
        if func.decl.is_extern {
            for (idx, ty) in formal_param_types.iter().enumerate() {
                if has_self_receiver && idx == 0 {
                    continue;
                }
                if Self::is_windjammer_text_param_type(ty) {
                    if let Some(slot) = param_ownership.get_mut(idx) {
                        *slot = OwnershipMode::Borrowed;
                    }
                }
            }
        }

        // Phase 3: Mirror impl-method registration — when ownership is Borrowed/MutBorrowed
        // but param_types still use bare `T`, wrap as `Reference(T)` / `MutableReference(T)`
        // so call-site lowering agrees with formal parameter codegen. Stale engine metadata
        // that marks owned `Custom(T)` as Borrowed keeps bare `T` and is handled separately.
        // Only `param_types` is wrapped — `formal_param_types` stays bare AST types.
        //
        // For borrowed string params, use str_ref optimization to determine &str vs &String:
        // - Params that are str_ref-optimizable → &str (performance)
        // - Params that need &String (e.g., passed to Vec::contains) → &String (correctness)
        let str_ref_optimized = if !func.decl.is_extern {
            self.analyze_str_ref_optimizable_params(&func.decl, registry)
        } else {
            std::collections::HashSet::new()
        };
        for (idx, ownership) in param_ownership.iter().enumerate() {
            if has_self_receiver && idx == 0 {
                continue;
            }
            let Some(ty) = param_types.get_mut(idx) else {
                continue;
            };
            match ownership {
                OwnershipMode::Borrowed
                    if !matches!(ty, PType::Reference(_) | PType::MutableReference(_))
                        && !self.is_copy_type(ty)
                        && !(func.decl.parent_type.is_none()
                            && matches!(ownership, OwnershipMode::Owned)
                            && func
                                .decl
                                .parameters
                                .get(idx)
                                .is_some_and(|p| Self::trait_param_is_owned_string(&p.type_))) =>
                {
                    *ty = if Self::is_windjammer_text_param_type(ty) {
                        let adj_idx = if has_self_receiver { idx - 1 } else { idx };
                        let param_name = func.decl.parameters.get(adj_idx).map(|p| p.name.as_str());
                        if param_name.is_some_and(|n| str_ref_optimized.contains(n)) {
                            PType::Reference(Box::new(PType::Custom("str".into())))
                        } else {
                            PType::Reference(Box::new(PType::String))
                        }
                    } else {
                        PType::Reference(Box::new(ty.clone()))
                    };
                }
                OwnershipMode::MutBorrowed
                    if !matches!(ty, PType::MutableReference(_)) && !self.is_copy_type(ty) =>
                {
                    *ty = PType::MutableReference(Box::new(ty.clone()));
                }
                _ => {}
            }
        }

        // Extract return type for smart string inference
        let return_type = func.decl.return_type.clone();

        FunctionSignature {
            name: func.decl.name.clone(),
            param_types,
            formal_param_types,
            param_ownership,
            return_type,
            return_ownership: OwnershipMode::Owned, // For now, always owned
            has_self_receiver,
            is_extern: func.decl.is_extern,
        }
    }

    /// Register all analyzed trait methods into a signature registry under
    /// `TraitName::method_name` keys.
    pub fn register_trait_methods_in_registry(
        &self,
        trait_methods: &std::collections::HashMap<
            String,
            std::collections::HashMap<String, AnalyzedFunction<'_>>,
        >,
        registry: &mut super::SignatureRegistry,
    ) {
        for (trait_name, methods) in trait_methods {
            for (method_name, analyzed_func) in methods {
                let sig = self.build_signature(analyzed_func, registry);
                let qualified_name = format!("{}::{}", trait_name, method_name);
                registry.add_function(qualified_name, sig);
            }
        }
    }
}
