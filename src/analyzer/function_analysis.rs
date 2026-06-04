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

            let self_ownership = if returns_self {
                OwnershipMode::Owned
            } else if body_moves_fields {
                OwnershipMode::Owned
            } else if self.function_body_consumes_bare_self(func) {
                OwnershipMode::Owned
            } else if self.function_calls_consuming_method_on_self(func, registry) {
                OwnershipMode::Owned
            } else if self.function_matches_on_self(func) {
                OwnershipMode::Owned
            } else if self.function_consumes_self_field_elements(func, Some(registry)) {
                OwnershipMode::Owned
            } else if modifies_fields {
                OwnershipMode::MutBorrowed
            } else if self.is_used_in_binary_op("self", &func.body) {
                OwnershipMode::Owned
            } else {
                OwnershipMode::Borrowed
            };

            // Store inferred self ownership
            inferred_ownership.insert("self".to_string(), self_ownership);
        }

        // Analyze each parameter to infer ownership mode
        for (i, param) in func.parameters.iter().enumerate() {
            let mode = match param.ownership {
                OwnershipHint::Owned => {
                    // DOGFOODING FIX #1: Respect explicit ownership annotations!
                    // If user writes `self` (not `&self` or `&mut self`), they want OWNED.
                    // Bug was: analyzer checked modifies_fields and downgraded to &mut self
                    // Fix: When Owned is explicit, use it - don't analyze or modify!
                    // Analysis should ONLY happen for OwnershipHint::Inferred.
                    OwnershipMode::Owned
                }
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
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
                        // `extern impl Type { fn f(self) {} }` / empty extern bodies: the signature is an
                        // FFI stub; bare `self` in Windjammer means a receiver is passed — for inherent
                        // impl methods, treat as `&mut self` so `self.field.method()` is dispatchable
                        // without moving the struct (see ownership_self_field_mutation test).
                        if func.is_extern && func.body.is_empty() && func.parent_type.is_some() {
                            OwnershipMode::MutBorrowed
                        } else {
                            let modifies_fields = self
                                .function_modifies_self_fields_with_registry(func, Some(registry));
                            let returns_self = self.function_returns_self(func);
                            let body_moves_fields =
                                self.function_body_moves_non_copy_self_fields(func);

                            if returns_self || body_moves_fields {
                                OwnershipMode::Owned
                            } else if self.function_moves_self_into_return(func) {
                                OwnershipMode::Owned
                            } else if self.function_body_consumes_bare_self(func) {
                                OwnershipMode::Owned
                            } else if self.function_calls_consuming_method_on_self(func, registry)
                            {
                                OwnershipMode::Owned
                            } else if self.function_matches_on_self(func) {
                                OwnershipMode::Owned
                            } else if self
                                .function_consumes_self_field_elements(func, Some(registry))
                            {
                                OwnershipMode::Owned
                            } else if self
                                .function_explicit_self_for_loops_self_field_by_value(func)
                            {
                                OwnershipMode::Owned
                            } else if modifies_fields {
                                OwnershipMode::MutBorrowed
                            } else {
                                if self.is_used_in_binary_op("self", &func.body) {
                                    OwnershipMode::Owned
                                } else {
                                    OwnershipMode::Borrowed
                                }
                            }
                        }
                    } else {
                        // For Copy types, check if they're mutated first
                        // Mutated Copy types should be &mut, not Owned
                        let is_copy = self.is_copy_type(&param.type_);

                        if is_copy {
                            let mutated = self.is_mutated(&param.name, &func.body, registry, Some(&param.type_));
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
                                OwnershipMode::Owned
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

        // PHASE 2 OPTIMIZATION: Detect unnecessary clones
        let clone_optimizations = self.detect_unnecessary_clones(func);

        // PHASE 3 OPTIMIZATION: Detect struct mapping opportunities
        let struct_mapping_optimizations = self.detect_struct_mappings(func);

        // PHASE 4 OPTIMIZATION: Detect string operation opportunities
        let string_optimizations = self.detect_string_optimizations(func);

        // PHASE 5: Detect assignment operations that can use compound operators
        let assignment_optimizations = self.detect_assignment_optimizations(func);
        let defer_drop_optimizations = self.detect_defer_drop_opportunities(func, registry);

        // AUTO-CLONE: Analyze where clones should be automatically inserted
        let auto_clone_analysis = AutoCloneAnalysis::analyze_function(func);

        // AUTO-MUT: Track which local variables are mutated (for automatic mut inference)
        self.track_mutations(&func.body, registry);
        let mutated_variables = self.mutated_variables.clone();

        // LINTER: Track which parameters are mutated (for owned-but-not-returned lint)
        let mut mutated_parameters = HashSet::new();
        for param in &func.parameters {
            if self.is_mutated(&param.name, &func.body, registry, Some(&param.type_)) {
                mutated_parameters.insert(param.name.clone());
            }
        }

        // PHASE 7-9: Additional optimizations (future implementation)
        let const_static_optimizations = Vec::new(); // TODO: Implement detection
        let smallvec_optimizations = Vec::new(); // TODO: Implement detection
        let cow_optimizations = Vec::new(); // TODO: Implement detection

        let cache_locality = super::CacheLocalityAnalysis::default();

        // PHASE 2: Analyze which string parameters can use &str optimization
        let str_ref_optimizable_params = self.analyze_str_ref_optimizable_params(func, registry);

        // Build inferred parameter types based on Phase 2 analysis
        let inferred_param_types: Vec<Type> = func
            .parameters
            .iter()
            .map(|param| {
                // Check if this parameter can be optimized to &str (instead of &String)
                let can_use_str_ref = str_ref_optimizable_params.contains(&param.name);

                if can_use_str_ref {
                    // Optimize to &str (not &String)
                    Type::Reference(Box::new(Type::Custom("str".to_string())))
                } else {
                    // Keep original type (will become &String for string params)
                    param.type_.clone()
                }
            })
            .collect();

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

                        // When the trait has concrete ownership data (from default
                        // impl or from a previous impl upgrade), the impl must match.
                        // When the trait has NO data (abstract method, never analyzed),
                        // the impl's own body analysis drives ownership, and the trait
                        // entry will be upgraded in the post-analysis merge step.
                        let final_mode = if let Some(mode) = trait_mode {
                            mode
                        } else {
                            analyzed
                                .inferred_ownership
                                .get(&impl_param.name)
                                .copied()
                                .unwrap_or_else(|| {
                                    self.convert_ownership_hint_to_mode(
                                        &trait_param.ownership,
                                        &trait_param.name,
                                    )
                                })
                        };

                        // INSERT or UPDATE with the final ownership mode
                        analyzed
                            .inferred_ownership
                            .insert(impl_param.name.clone(), final_mode);
                    }
                }
            }
        }

        // E0186 / E0053: Impl receiver must match the trait — never infer only from the impl body.
        // Replacing analyzed_trait_methods with the impl used to drop implicit `self` when the impl
        // body was empty or omitted `self`, producing trait/impl mismatches in Rust.
        if let Some(self_mode) =
            self.trait_method_receiver_ownership(trait_name, trait_key, &func.name)
        {
            analyzed
                .inferred_ownership
                .insert("self".to_string(), self_mode);
        }

        // E0053: Parameter types in generated Rust must match the trait declaration (impls may
        // rename parameters or use incompatible aliases). Ownership already matches the trait above.
        if !is_std_operator_trait {
            if let Some(analyzed_trait_fn) = self
                .analyzed_trait_methods
                .get(trait_key)
                .and_then(|m| m.get(&func.name))
                .or_else(|| {
                    self.analyzed_trait_methods
                        .get(trait_name)
                        .and_then(|m| m.get(&func.name))
                })
            {
                for (i, _) in func.parameters.iter().enumerate() {
                    if let Some(trait_ty) = analyzed_trait_fn.inferred_param_types.get(i) {
                        if i < analyzed.inferred_param_types.len() {
                            analyzed.inferred_param_types[i] = trait_ty.clone();
                        }
                    }
                }
            }
            // If multipass never stored analyzed trait fn types, still copy AST parameter types so
            // generated Rust matches the trait item (E0053).
            if let Some(trait_decl) = self.trait_definitions.get(trait_key) {
                if let Some(trait_method) = trait_decl.methods.iter().find(|m| m.name == func.name)
                {
                    let tf = self
                        .analyzed_trait_methods
                        .get(trait_key)
                        .and_then(|m| m.get(&func.name))
                        .or_else(|| {
                            self.analyzed_trait_methods
                                .get(trait_name)
                                .and_then(|m| m.get(&func.name))
                        });
                    for (i, trait_param) in trait_method.parameters.iter().enumerate() {
                        if i >= analyzed.inferred_param_types.len() {
                            break;
                        }
                        let use_ast = tf.and_then(|t| t.inferred_param_types.get(i)).is_none();
                        if use_ast {
                            analyzed.inferred_param_types[i] = trait_param.type_.clone();
                        }
                    }
                }
            }
        }

        Ok(analyzed)
    }
    pub(crate) fn build_signature(&self, func: &AnalyzedFunction) -> FunctionSignature {
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

                if Self::is_windjammer_text_param_type(&param.type_)
                    && self.is_only_hashmap_lookup_key_param(
                        &param.name,
                        &func.decl.body,
                        &func.decl,
                    )
                {
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

                // Copy types are always passed by value (Owned) unless mutated
                // This must match the logic in codegen.rs
                if self.is_copy_type(&param.type_) {
                    // Copy types: pass by value unless they need to be mutated
                    if inferred == OwnershipMode::MutBorrowed {
                        OwnershipMode::MutBorrowed
                    } else {
                        OwnershipMode::Owned
                    }
                } else {
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
                }
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
                // Use inferred type if available (Phase 2 optimization)
                // Otherwise fall back to explicit type annotation
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

        // Phase 2: `string` may become `Reference(Custom("str"))` in inferred_param_types while
        // ownership inference still marks the parameter `Owned` (e.g. forwarding calls where
        // the body is only `Call(FieldAccess)` and string-ref analysis does not recurse).
        // Rust lowers these parameters as `&str` with a borrow — call-site helpers that key
        // off `param_ownership` (`should_add_to_string`, `OwnershipMode::Owned` in bare `Call`)
        // must agree or we emit `arg.to_string()` / bad conversions for `&str` → `&str` calls.
        use crate::parser::Type as PType;
        for (idx, ty) in param_types.iter().enumerate() {
            if matches!(
                ty,
                PType::Reference(inner)
                    if matches!(&**inner, PType::Custom(s) if s == "str")
            ) {
                if let Some(slot) = param_ownership.get_mut(idx) {
                    *slot = OwnershipMode::Borrowed;
                }
            }
        }

        // Extract return type for smart string inference
        let return_type = func.decl.return_type.clone();

        FunctionSignature {
            name: func.decl.name.clone(),
            param_types,
            param_ownership,
            return_type,
            return_ownership: OwnershipMode::Owned, // For now, always owned
            has_self_receiver,
            is_extern: func.decl.is_extern,
        }
    }
}
