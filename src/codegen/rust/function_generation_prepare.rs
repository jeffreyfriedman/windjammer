//! Setup and per-function analyzer state loaded before emitting a regular function.

use crate::analyzer::*;
use crate::codegen::rust::type_analysis;
use crate::parser::*;

use super::CodeGenerator;

/// How a parameter appears in an expression tree (for readonly demotion decisions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProjectionUsage {
    None,
    FieldOrIndexOnly,
    BareOrOther,
}

impl ProjectionUsage {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (ProjectionUsage::BareOrOther, _) | (_, ProjectionUsage::BareOrOther) => {
                ProjectionUsage::BareOrOther
            }
            (ProjectionUsage::FieldOrIndexOnly, _) | (_, ProjectionUsage::FieldOrIndexOnly) => {
                ProjectionUsage::FieldOrIndexOnly
            }
            (ProjectionUsage::None, ProjectionUsage::None) => ProjectionUsage::None,
        }
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// Push `#[test]` for `test_*` functions in `*_test.wj` files when no `@test` / `@property_test`.
    pub(in crate::codegen::rust) fn push_auto_test_attribute_if_needed(
        &self,
        func: &FunctionDecl<'ast>,
        output: &mut String,
    ) {
        let filename_str = self.current_wj_file.to_string_lossy();
        let is_test_file = filename_str.ends_with("_test.wj") || filename_str.contains("_test.wj");
        let is_test_function = func.name.starts_with("test_");
        let has_test_decorator = func.decorators.iter().any(|d| d.name == "test");
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");

        if is_test_file && is_test_function && !has_test_decorator && !has_property_test {
            output.push_str("#[test]\n");
        }
    }

    /// Configure `CodeGenerator` fields from `AnalyzedFunction` before signature/body emission.
    pub(in crate::codegen::rust) fn prepare_codegen_environment_for_regular_function(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
    ) {
        let func = &analyzed.decl;

        // LOCAL VARIABLE TRACKING: Push new scope for this function
        self.local_variable_scopes
            .push(std::collections::HashSet::new());

        // AUTO-CLONE: Load auto-clone analysis for this function.
        // When IR cutover is active for clones, read from IrFunction's optimization hints.
        if self.ir_cutover.clones && self.current_ir_function.is_some() {
            if let Some(ir_fn) = &self.current_ir_function {
                self.auto_clone_analysis = Some(ir_fn.optimizations.auto_clone.clone());
            }
        } else {
            self.auto_clone_analysis = Some(analyzed.auto_clone_analysis.clone());
        }
        self.auto_clone_counter = 0;

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        self.current_function_params = func.parameters.clone();
        self.current_function_name = Some(func.name.to_string());
        // Combine inline bounds (<T: Foo>) and where clause for trait resolution
        let mut all_bounds: Vec<(String, Vec<String>)> = func
            .type_params
            .iter()
            .filter(|tp| !tp.bounds.is_empty())
            .map(|tp| (tp.name.clone(), tp.bounds.clone()))
            .collect();
        for (name, bounds) in &func.where_clause {
            if let Some(existing) = all_bounds.iter_mut().find(|(n, _)| n == name) {
                existing.1.extend(bounds.iter().cloned());
            } else {
                all_bounds.push((name.clone(), bounds.clone()));
            }
        }
        self.current_function_type_bounds = all_bounds;

        // Clear local variable types for new function scope
        self.local_var_types.clear();
        self.borrowed_iterator_vars.clear();

        // Track function return type for string literal conversion
        self.current_function_return_type = func.return_type.clone();

        // Track method return types for usize inference in comparisons
        // When in an impl block, record the return type so expression_produces_usize
        // can resolve method calls like animation.frame_count() → usize
        if self.in_impl_block {
            if let Some(ref ret_type) = func.return_type {
                self.method_return_types
                    .insert(func.name.to_string(), ret_type.clone());
            }

            // NEW ARCHITECTURE: Register method signature for type-based parameter resolution
            // This replaces ALL hard-coded method name heuristics
            if let Some(impl_type) = self.current_struct_name.clone() {
                self.register_impl_method_signature_from_analyzed(&impl_type, func, analyzed);
            }
        }

        // Track function body for data flow analysis
        self.current_function_body = func.body.clone();
        self.full_function_body_snapshot = func.body.clone();

        // FOR-LOOP AUTO-BORROW: Pre-scan function body to find local variables
        // that are iterated in for-loops and also used after the loop.
        // These need `&` auto-inserted to prevent consuming the collection.
        self.precompute_for_loop_borrows(&func.body);

        // Track parameters inferred as borrowed/mut-borrowed for codegen decisions.
        // Uses IR-backed helpers when cutover is enabled, falling back to AnalyzedFunction.
        self.inferred_borrowed_params.clear();
        self.inferred_mut_borrowed_params.clear();
        self.current_fn_emitted_mut_arg_indices.clear();
        self.str_ref_optimized_params.clear();
        self.collection_key_owned_params.clear();
        self.emitted_rust_ref_formals.clear();
        self.current_fn_mixed_forwarder_params.clear();
        self.current_fn_forward_ref_if_params.clear();
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            // Single-arg pure delegates keep owned formals — not mixed forwarders.
            if self.param_is_single_arg_call_only_delegate(param, func) {
                continue;
            }
            let if_branch_facade = self.param_used_in_if_with_condition_and_branches(
                func.body.as_slice(),
                &param.name,
            ) || self.param_used_in_if_else_both_branches(
                func.body.as_slice(),
                &param.name,
            );
            // Multi-arg single-call delegates borrow at formals — not mixed forwarders.
            if !if_branch_facade
                && self.param_only_used_as_call_argument(func.body.as_slice(), &param.name, func)
                && self.count_non_self_params(func) >= 2
            {
                continue;
            }
            let passes_owned = self.param_passes_to_wj_owned_sibling_call(
                func.body.as_slice(),
                &param.name,
                func,
            ) || self.param_passed_to_owned_self_method_arg(
                func.body.as_slice(),
                &param.name,
                func,
            );
            let passes_borrow =
                self.param_passed_to_borrowing_callee(func.body.as_slice(), &param.name, func);
            if (passes_owned && passes_borrow) || if_branch_facade {
                self.current_fn_mixed_forwarder_params
                    .insert(param.name.clone());
            }
            if if_branch_facade {
                self.current_fn_forward_ref_if_params
                    .insert(param.name.clone());
            } else if self.body_forwards_param_in_if_condition(&param.name, func)
                || self.param_has_forward_ref_keep_owned(
                    func.body.as_slice(),
                    &param.name,
                    func,
                )
            {
                self.current_fn_forward_ref_if_params
                    .insert(param.name.clone());
            }
        }
        self.current_func_is_pure_forwarding_delegate =
            self.func_is_pure_forwarding_delegate(func);
        for (param_name, ownership) in self.get_all_param_ownership(analyzed) {
            if param_name != "self" {
                if func.parameters.iter().any(|p| {
                    p.name == param_name
                        && self.current_struct_name.as_ref().is_some_and(|sn| {
                            self.struct_is_owned_engine_key_facade(sn, p)
                        })
                }) {
                    continue;
                }
                if self.param_passed_to_owned_self_method_arg(func.body.as_slice(), &param_name, func) {
                    continue;
                }
                if self.param_passes_to_wj_owned_sibling_call(
                    func.body.as_slice(),
                    &param_name,
                    func,
                ) {
                    continue;
                }
                if self.param_only_used_in_discarding_let_binding(
                    func.body.as_slice(),
                    &param_name,
                    func,
                ) {
                    continue;
                }
            }
            match ownership {
                crate::analyzer::OwnershipMode::Borrowed => {
                    self.inferred_borrowed_params.insert(param_name);
                }
                crate::analyzer::OwnershipMode::MutBorrowed => {
                    self.inferred_mut_borrowed_params.insert(param_name);
                }
                _ => {}
            }
        }
        // Legacy analyzer path only — IR solver is authoritative when ownership cutover is on.
        if !self.ir_cutover.ownership {
            for (param_name, ownership) in &analyzed.inferred_ownership {
                if param_name != "self" {
                    if self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), param_name, func) {
                        continue;
                    }
                    if self.param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        param_name,
                        func,
                    ) {
                        continue;
                    }
                }
                match ownership {
                    crate::analyzer::OwnershipMode::Borrowed => {
                        self.inferred_borrowed_params.insert(param_name.clone());
                    }
                    crate::analyzer::OwnershipMode::MutBorrowed => {
                        self.inferred_mut_borrowed_params.insert(param_name.clone());
                    }
                    _ => {}
                }
            }
        }

        // Copy aggregates borrow only when converged signature carries Reference(T)
        // (field-enum-match readonly pattern). Stale Borrowed metadata alone must not
        // emit `&Vec3` formals (bug_copy_vec3_formal_param_not_ref).
        for (idx, param) in func.parameters.iter().enumerate() {
            if param.name == "self" {
                continue;
            }
            if !self.is_type_copy(&param.type_) {
                continue;
            }
            if crate::type_classification::is_copy_pass_by_value_formal(&param.type_) {
                continue;
            }
            let converged_borrow = self
                .get_signature_with_global(&func.name)
                .and_then(|sig| sig.param_ownership.get(idx))
                .is_some_and(|o| {
                    matches!(
                        o,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    )
                });
            if !converged_borrow {
                let analyzer_borrowed = analyzed
                    .inferred_ownership
                    .get(&param.name)
                    .is_some_and(|m| {
                        matches!(
                            m,
                            crate::analyzer::OwnershipMode::Borrowed
                                | crate::analyzer::OwnershipMode::MutBorrowed
                        )
                    });
                if !analyzer_borrowed {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.inferred_mut_borrowed_params.remove(&param.name);
                }
            }
        }

        // Registry converged Owned overrides stale borrow hints (Copy Vec3 field reads).
        for (idx, param) in func.parameters.iter().enumerate() {
            if param.name == "self" {
                continue;
            }
            if self
                .get_signature_with_global(&func.name)
                .and_then(|sig| sig.param_ownership.get(idx))
                == Some(&crate::analyzer::OwnershipMode::Owned)
            {
                self.inferred_borrowed_params.remove(&param.name);
                self.inferred_mut_borrowed_params.remove(&param.name);
            }
        }

        // Track Phase 2 string-optimized parameters (string type params that become &str).
        // Uses IR-backed str_ref check when cutover is enabled.
        if self.ir_cutover.str_ref && self.current_ir_function.is_some() {
            if let Some(ir_fn) = &self.current_ir_function {
                for param_name in &ir_fn.str_ref_params {
                    if self.get_param_ownership(param_name, analyzed)
                        == Some(crate::analyzer::OwnershipMode::Owned)
                    {
                        continue;
                    }
                    if analyzed.inferred_ownership.get(param_name)
                        == Some(&crate::analyzer::OwnershipMode::Owned)
                    {
                        continue;
                    }
                    if !self.param_only_forwarded_to_qualified_collection_key_callee(
                        func.body.as_slice(),
                        param_name,
                        func,
                    ) {
                        self.str_ref_optimized_params.insert(param_name.clone());
                    }
                }
            }
            for param_name in &analyzed.str_ref_optimizable_params {
                if self.str_ref_optimized_params.contains(param_name) {
                    continue;
                }
                if self.get_param_ownership(param_name, analyzed)
                    == Some(crate::analyzer::OwnershipMode::Owned)
                {
                    continue;
                }
                if analyzed.inferred_ownership.get(param_name)
                    == Some(&crate::analyzer::OwnershipMode::Owned)
                {
                    continue;
                }
                if !self.param_only_forwarded_to_qualified_collection_key_callee(
                    func.body.as_slice(),
                    param_name,
                    func,
                ) {
                    self.str_ref_optimized_params.insert(param_name.clone());
                }
            }
        } else {
            for param_name in &analyzed.str_ref_optimizable_params {
                if analyzed.inferred_ownership.get(param_name)
                    == Some(&crate::analyzer::OwnershipMode::Owned)
                {
                    continue;
                }
                if !self.param_only_forwarded_to_qualified_collection_key_callee(
                    func.body.as_slice(),
                    param_name,
                    func,
                ) {
                    self.str_ref_optimized_params.insert(param_name.clone());
                }
            }
        }

        if self.ir_cutover.ownership {
            for param in &func.parameters {
                if param.name == "self" {
                    continue;
                }
                if self.current_struct_name.as_ref().is_some_and(|sn| {
                    self.struct_is_owned_engine_key_facade(sn, param)
                }) {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    continue;
                }
                if self.current_fn_mixed_forwarder_params.contains(&param.name)
                    || self.param_has_forward_ref_keep_owned(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.str_ref_optimized_params.remove(&param.name);
                    continue;
                }
                if self.param_stored_in_owned_payload(func.body.as_slice(), &param.name) {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    self.str_ref_optimized_params.remove(&param.name);
                    continue;
                }
                if self.param_only_used_in_discarding_let_binding(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) {
                    continue;
                }
                match self
                    .get_param_ownership(&param.name, analyzed)
                    .or_else(|| analyzed.inferred_ownership.get(&param.name).copied())
                {
                    Some(crate::analyzer::OwnershipMode::Borrowed) => {
                        if self.param_only_used_as_call_argument(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        ) && !self.param_should_emit_borrowed_delegation_formal(param, func)
                            && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                            && !self.is_type_copy(&param.type_)
                        {
                            // has_key → get: body-converged borrow must not emit `&Key`
                            // when the callee chain keeps WJ-owned formals.
                        } else {
                            self.inferred_borrowed_params.insert(param.name.clone());
                        }
                    }
                    Some(crate::analyzer::OwnershipMode::MutBorrowed) => {
                        self.inferred_mut_borrowed_params.insert(param.name.clone());
                    }
                    _ => {}
                }
            }
        } else {
            for (idx, param) in func.parameters.iter().enumerate() {
                if param.name != "self"
                    && self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    continue;
                }
                if self.collection_key_owned_params.contains(&param.name)
                    || (func.parent_type.is_none()
                        && self.param_only_forwarded_to_qualified_collection_key_callee(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        ))
                {
                    continue;
                }
                if self.param_only_used_in_discarding_let_binding(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) {
                    continue;
                }
                if let Some(Type::Reference(inner)) = analyzed.inferred_param_types.get(idx) {
                    if matches!(
                        &**inner,
                        Type::Custom(s) if s == "str" || s == "string" || s == "String"
                    ) || matches!(&**inner, Type::String)
                    {
                        self.str_ref_optimized_params.insert(param.name.clone());
                    }
                    self.inferred_borrowed_params.insert(param.name.clone());
                } else if let Some(Type::MutableReference(_)) = analyzed.inferred_param_types.get(idx) {
                    self.inferred_mut_borrowed_params.insert(param.name.clone());
                }
            }
        }

        // Track explicit &String/&string params that become &str via type_to_rust
        // (Type::Reference(String) → "&str"). These aren't Phase 2 optimized but still
        // need .to_string() conversions in the body (e.g., Some(s) → Some(s.to_string())).
        for param in &func.parameters {
            if matches!(&param.type_, Type::Reference(inner)
                if matches!(&**inner, Type::String)
                    || matches!(&**inner, Type::Custom(ref n) if n == "string" || n == "String"))
            {
                self.str_ref_optimized_params.insert(param.name.clone());
            }
        }

        // METHOD PARAM OWNERSHIP: Register this method's parameter ownership modes
        // for use at call sites (auto-borrow arguments).
        {
            let ownership_vec: Vec<(String, crate::analyzer::OwnershipMode)> = func
                .parameters
                .iter()
                .filter(|p| p.name != "self")
                .map(|param| {
                    let mode = if self.current_fn_mixed_forwarder_params.contains(&param.name)
                        || self.param_has_forward_ref_keep_owned(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        )
                    {
                        crate::analyzer::OwnershipMode::Owned
                    } else if self.inferred_mut_borrowed_params.contains(&param.name) {
                        crate::analyzer::OwnershipMode::MutBorrowed
                    } else if self.inferred_borrowed_params.contains(&param.name)
                        || self.str_ref_optimized_params.contains(&param.name)
                    {
                        crate::analyzer::OwnershipMode::Borrowed
                    } else {
                        self.get_param_ownership(&param.name, analyzed)
                            .unwrap_or(crate::analyzer::OwnershipMode::Owned)
                    };
                    (param.name.clone(), mode)
                })
                .collect();
            if !ownership_vec.is_empty() {
                self.method_param_ownership
                    .insert(func.name.to_string(), ownership_vec);
            }
        }

        // WINDJAMMER FIX: Track usize-typed parameters for auto-cast logic
        // DON'T clear here - we need to accumulate variables from let statements during generation!
        // Only clear at the very beginning of function generation, before body processing.
        // TDD FIX (Bug #3): Moved clear to happen BEFORE pre-passes, so marking during
        // statement generation can accumulate variables.

        // Clear ONCE at function start (before any analysis)
        self.usize_variables.clear();

        // When a parameter is declared as `usize`, add it to usize_variables
        // so expression_produces_usize() correctly identifies it
        for (param_idx, param) in func.parameters.iter().enumerate() {
            // Use inferred type if available, otherwise use declared type
            let param_type = analyzed
                .inferred_param_types
                .get(param_idx)
                .unwrap_or(&param.type_);

            // Check if this parameter is usize
            if matches!(param_type, Type::Custom(name) if name == "usize") {
                self.usize_variables.insert(param.name.clone());
            }
        }

        // PHASE 8 OPTIMIZATION: Load SmallVec optimizations for this function
        // DISABLED: SmallVec optimizations conflict with return types
        // TODO: Re-enable with smarter conversion at return sites
        self.smallvec_optimizations.clear();
        // for opt in &analyzed.smallvec_optimizations {
        //     self.smallvec_optimizations
        //         .insert(opt.variable.clone(), opt.clone());
        //     self.needs_smallvec_import = true; // Mark that we need the smallvec crate
        // }

        // PHASE 9 OPTIMIZATION: Load Cow optimizations for this function
        self.cow_optimizations.clear();
        for opt in &analyzed.cow_optimizations {
            self.cow_optimizations.insert(opt.variable.clone());
            self.needs_cow_import = true; // Mark that we need Cow from std::borrow
        }

        // PHASE 3 OPTIMIZATION: Load struct mapping optimizations
        // Track which structs can use optimized construction strategies
        self.struct_mapping_hints.clear();
        for opt in &analyzed.struct_mapping_optimizations {
            self.struct_mapping_hints
                .insert(opt.target_struct.clone(), opt.strategy.clone());
        }

        // PHASE 4 OPTIMIZATION: Load string operation optimizations
        // Track capacity hints for string operations
        self.string_capacity_hints.clear();

        // PHASE 5 OPTIMIZATION: Load assignment operation optimizations
        // Track which variables can use compound assignment operators
        self.assignment_optimizations.clear();
        for opt in &analyzed.assignment_optimizations {
            self.assignment_optimizations
                .insert(opt.variable.clone(), opt.operation.clone());
        }
        for opt in &analyzed.string_optimizations {
            if let Some(capacity) = opt.estimated_capacity {
                self.string_capacity_hints.insert(opt.location, capacity);
            }
        }

        // PHASE 6 OPTIMIZATION: Load defer drop optimizations
        // Track variables that should have their drops deferred to background thread
        self.defer_drop_optimizations = analyzed.defer_drop_optimizations.clone();

        self.promote_callee_forwarded_borrows(func);
        self.promote_readonly_operand_borrows(func, analyzed);
        self.strip_borrow_inference_for_owning_param_uses(func);
        if self.in_impl_block {
            if self.current_struct_name.is_some() {
                self.sync_method_registry_from_inferred_borrows(func, analyzed);
                self.promote_unused_readonly_vec_params_to_borrowed(func);
            }
        }
        self.preserve_owned_formals_for_collection_key_only_params(func);

        // Analyzer converged Borrowed beats stale IR Owned for readonly passthrough wrappers.
        if self.ir_cutover.ownership {
            for (param_name, ownership) in &analyzed.inferred_ownership {
                if param_name == "self" {
                    continue;
                }
                if !matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
                    continue;
                }
                if self.collection_key_owned_params.contains(param_name) {
                    continue;
                }
                if self.current_fn_mixed_forwarder_params.contains(param_name)
                    || self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        param_name,
                        func,
                    )
                {
                    continue;
                }
                if self
                    .current_struct_name
                    .as_ref()
                    .is_some_and(|sn| {
                        func.parameters.iter().any(|p| {
                            p.name == *param_name
                                && self.struct_is_owned_engine_key_facade(sn, p)
                        })
                    })
                {
                    continue;
                }
                self.inferred_borrowed_params.insert(param_name.clone());
                self.inferred_mut_borrowed_params.remove(param_name);
            }
        }
    }

    /// Text map/set key params keep owned `String` formals; call sites add `&`.
    ///
    /// Non-text keys (`QuestId`, `Key`, …) must **not** force owned formals — body-only
    /// lookup usage converges to `&T` (and must beat stale engine metadata `Owned`).
    pub(in crate::codegen::rust) fn is_collection_key_owned_param(
        &self,
        param: &Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if !crate::codegen::rust::types::is_windjammer_text_type(&param.type_) {
            return false;
        }
        if self.collection_key_owned_params.contains(&param.name) {
            return true;
        }
        if self.param_only_forwarded_to_map_key_callee(
            func.body.as_slice(),
            &param.name,
            func,
        ) || (func.parent_type.is_none()
            && self.param_only_forwarded_to_qualified_map_key_callee(
                func.body.as_slice(),
                &param.name,
                func,
            ))
        {
            return true;
        }
        if self.str_ref_optimized_params.contains(&param.name)
            || self.inferred_borrowed_params.contains(&param.name)
        {
            return false;
        }
        // Text keys: owned formals for HashMap forwarding only; HashSet::contains keeps &str.
        false
    }

    /// Text map/set key helpers keep owned `String` formals; call sites add `&`.
    /// Non-text keys keep analyzer/IR borrow convergence (`&QuestId`).
    fn preserve_owned_formals_for_collection_key_only_params(
        &mut self,
        func: &FunctionDecl<'ast>,
    ) {
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            if !crate::codegen::rust::types::is_windjammer_text_type(&param.type_) {
                continue;
            }
            let preserve = if self.str_ref_optimized_params.contains(&param.name)
                || self.inferred_borrowed_params.contains(&param.name)
            {
                false
            } else {
                self.param_only_forwarded_to_qualified_map_key_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) || self.param_only_forwarded_to_map_key_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                )
            };
            if preserve {
                self.inferred_borrowed_params.remove(&param.name);
                self.inferred_mut_borrowed_params.remove(&param.name);
                self.str_ref_optimized_params.remove(&param.name);
                self.collection_key_owned_params.insert(param.name.clone());
            }
        }
    }

    /// Promote readonly/unused `Vec` params to borrowed formals (`.len()` / `[i]` reads only).
    fn promote_unused_readonly_vec_params_to_borrowed(&mut self, func: &FunctionDecl<'ast>) {
        let unused = self.compute_unused_formal_parameter_names(func);
        for param in &func.parameters {
            if param.name == "self" || !Self::param_type_is_vec_container(&param.type_) {
                continue;
            }
            if self.inferred_borrowed_params.contains(&param.name)
                || self.inferred_mut_borrowed_params.contains(&param.name)
            {
                continue;
            }
            if self.param_has_owning_method_use(func.body.as_slice(), &param.name, func) {
                continue;
            }
            if self.param_consumed_as_for_loop_iterable(func.body.as_slice(), &param.name) {
                continue;
            }
            if unused.contains(&param.name)
                || self.param_has_readonly_expression_use(func.body.as_slice(), &param.name)
                // Store-only Vec constructors (`from_bytes` / field init) emit `&Vec` and
                // clone at the struct field — callers can borrow (WDB-049).
                || self.param_stored_in_owned_payload(func.body.as_slice(), &param.name)
                || self.param_moves_via_struct_literal_init(func.body.as_slice(), &param.name)
            {
                self.inferred_borrowed_params.insert(param.name.clone());
            }
        }
    }

    /// Params forwarded to both borrowing and owning callees keep owned formals (wdb `Key`).
    fn strip_borrow_inference_for_owning_param_uses(&mut self, func: &FunctionDecl<'ast>) {
        for param in &func.parameters {
            if param.name == "self" || type_analysis::is_copy_type(&param.type_) {
                continue;
            }
            if self.param_is_non_self_forward_facade_borrow_candidate(param, func) {
                continue;
            }
            if self.param_passed_to_owned_self_method_arg(func.body.as_slice(), &param.name, func)
                || self.param_passes_to_wj_owned_sibling_call(
                    func.body.as_slice(),
                    &param.name,
                    func,
                )
                || self.param_only_forwards_to_emitted_owned_callees(
                    func.body.as_slice(),
                    &param.name,
                    func,
                )
                || self.param_stored_in_owned_payload(func.body.as_slice(), &param.name)
                || self.param_consumed_as_for_loop_iterable(func.body.as_slice(), &param.name)
                || (self
                    .current_struct_name
                    .as_ref()
                    .is_some_and(|sn| self.struct_is_owned_engine_key_facade(sn, param))
                    // Asymmetric Value: field-projection-only params keep `&T` even on
                    // Key-facade structs (`apply_patch_put(key, &value)`).
                    && !self.param_only_used_via_field_or_index_projection(
                        func.body.as_slice(),
                        &param.name,
                    ))
            {
                self.inferred_borrowed_params.remove(&param.name);
                self.inferred_mut_borrowed_params.remove(&param.name);
            }
        }
    }

    /// Promote non-Copy params used only as read operands (e.g. `value.data.len()`) to borrowed
    /// formals so preregister converges `&Value` before sibling callers emit (`put_value` →
    /// `apply_patch_put(key, &value)`).
    fn promote_readonly_operand_borrows(
        &mut self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'_>,
    ) {
        for param in &func.parameters {
            if param.name == "self" || type_analysis::is_copy_type(&param.type_) {
                continue;
            }
            if matches!(
                &param.type_,
                Type::Reference(_) | Type::MutableReference(_)
            ) {
                continue;
            }
            if self.inferred_borrowed_params.contains(&param.name)
                || self.inferred_mut_borrowed_params.contains(&param.name)
            {
                continue;
            }
            // Mutated params (field writes or `&mut self` methods) must not become shared `&T`.
            // Owned vs MutBorrowed is decided by analyzer/IR — only block the demotion here.
            if analyzed.mutated_parameters.contains(&param.name) {
                self.inferred_borrowed_params.remove(&param.name);
                continue;
            }
            // Returned/moved params must stay Owned (solver lattice + API intent).
            if analyzed.returned_parameters.contains(&param.name) {
                self.inferred_borrowed_params.remove(&param.name);
                continue;
            }
            if self.param_stored_in_owned_payload(func.body.as_slice(), &param.name) {
                self.inferred_borrowed_params.remove(&param.name);
                continue;
            }
            // Trust analyzer/IR Owned — do not demote to shared `&T` in codegen.
            // Exceptions:
            // - `Vec` params used only for readonly `.len()` / `[i]` (indexing over-owns)
            // - Custom/enum params used only via field projection (`value.data.len()`) so
            //   asymmetric facades converge `&Value` before callers emit (apply_patch_put).
            // - WJ `string` params only discarded (`let _ = path`) → `&str`.
            if matches!(
                analyzed.inferred_ownership.get(&param.name),
                Some(crate::analyzer::OwnershipMode::Owned)
            ) {
                let vec_readonly = Self::param_type_is_vec_container(&param.type_)
                    && self.param_has_readonly_expression_use(func.body.as_slice(), &param.name)
                    && !self.param_has_owning_method_use(func.body.as_slice(), &param.name, func);
                let field_proj_readonly = self.param_only_used_via_field_or_index_projection(
                    func.body.as_slice(),
                    &param.name,
                ) && !self.param_has_owning_method_use(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) && !self.param_stored_in_owned_payload(
                    func.body.as_slice(),
                    &param.name,
                );
                let text_discard_only = crate::codegen::rust::types::is_windjammer_text_type(
                    &param.type_,
                ) && self.param_only_used_in_simple_or_tuple_discard(
                    func.body.as_slice(),
                    &param.name,
                );
                if !vec_readonly && !field_proj_readonly && !text_discard_only {
                    continue;
                }
                if text_discard_only {
                    self.inferred_borrowed_params.insert(param.name.clone());
                    self.str_ref_optimized_params.insert(param.name.clone());
                    continue;
                }
            }
            if (self.current_fn_mixed_forwarder_params.contains(&param.name)
                || self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
                || self.param_passes_to_wj_owned_sibling_call(
                    func.body.as_slice(),
                    &param.name,
                    func,
                )
                || self.current_struct_name.as_ref().is_some_and(|sn| {
                    self.struct_is_owned_engine_key_facade(sn, param)
                }))
                && !self.param_is_non_self_forward_facade_borrow_candidate(param, func)
            {
                // Asymmetric facades: field-projection-only params (e.g. `value.data.len()`)
                // still demote to `&T` even when the sibling Key param is a keep-owned facade.
                if !self.param_only_used_via_field_or_index_projection(
                    func.body.as_slice(),
                    &param.name,
                ) {
                    continue;
                }
            }
            if self.param_is_non_self_forward_facade_borrow_candidate(param, func) {
                self.inferred_borrowed_params.insert(param.name.clone());
                continue;
            }
            if self.param_has_owning_method_use(func.body.as_slice(), &param.name, func) {
                continue;
            }
            if self.param_consumed_as_for_loop_iterable(func.body.as_slice(), &param.name) {
                continue;
            }
            if self.param_only_used_in_discarding_let_binding(
                func.body.as_slice(),
                &param.name,
                func,
            ) && !(Self::param_type_is_vec_container(&param.type_)
                && self.param_has_readonly_expression_use(func.body.as_slice(), &param.name)
                && !self.param_has_owning_method_use(func.body.as_slice(), &param.name, func))
            {
                continue;
            }
            if self.param_has_readonly_expression_use(func.body.as_slice(), &param.name) {
                self.inferred_borrowed_params.insert(param.name.clone());
            }
        }
    }

    fn param_is_non_self_forward_facade_borrow_candidate(
        &self,
        param: &crate::parser::Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        // Registry-aware Copy aggregates (Vec3, …) must stay owned formals.
        if self.is_type_copy(&param.type_) {
            return false;
        }
        self.count_non_self_params(func) >= 2
            && !self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
            && self.param_passed_to_non_self_receiver_method_arg(
                func.body.as_slice(),
                &param.name,
                func,
            )
    }

    /// Read-only use including `let _ = param.field` discard bindings (wdb apply_patch_put value).
    pub(in crate::codegen::rust) fn param_has_readonly_expression_use(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        for stmt in body {
            match stmt {
                Statement::Let { value, else_block, .. } => {
                    if self.expression_uses_param_as_read_operand(value, param_name) {
                        return true;
                    }
                    if else_block.as_ref().is_some_and(|b| {
                        self.param_has_readonly_expression_use(b.as_slice(), param_name)
                    }) {
                        return true;
                    }
                }
                _ => {
                    if self.statement_uses_param_as_read_operand(stmt, param_name) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// True when every mention of `param_name` is under a field/index projection
    /// (`param.field`, `param[i]`, `param.field.method()`), never as a bare value.
    ///
    /// Distinguishes `let _ = value.data.len()` (field projection → may borrow) from
    /// `let _ = (key, value)` (bare move into tuple → keep owned).
    pub(in crate::codegen::rust) fn param_only_used_via_field_or_index_projection(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        let mut found = false;
        for stmt in body {
            match self.statement_param_projection_usage(stmt, param_name) {
                ProjectionUsage::None => {}
                ProjectionUsage::FieldOrIndexOnly => found = true,
                ProjectionUsage::BareOrOther => return false,
            }
        }
        found
    }

    fn statement_param_projection_usage(
        &self,
        stmt: &Statement<'ast>,
        param_name: &str,
    ) -> ProjectionUsage {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_param_projection_usage(expr, param_name)
            }
            Statement::Return { .. } => ProjectionUsage::None,
            Statement::Let { value, else_block, .. } => {
                let mut usage = self.expression_param_projection_usage(value, param_name);
                if let Some(b) = else_block {
                    for s in b {
                        usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                    }
                }
                usage
            }
            Statement::Assignment { value, .. } => {
                self.expression_param_projection_usage(value, param_name)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let mut usage = self.expression_param_projection_usage(condition, param_name);
                for s in then_block {
                    usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                }
                if let Some(b) = else_block {
                    for s in b {
                        usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                    }
                }
                usage
            }
            Statement::While { condition, body, .. } => {
                let mut usage = self.expression_param_projection_usage(condition, param_name);
                for s in body {
                    usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                }
                usage
            }
            Statement::For { iterable, body, .. } => {
                let mut usage = self.expression_param_projection_usage(iterable, param_name);
                for s in body {
                    usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                }
                usage
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                let mut usage = ProjectionUsage::None;
                for s in body {
                    usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                }
                usage
            }
            Statement::Match { value, arms, .. } => {
                let mut usage = self.expression_param_projection_usage(value, param_name);
                for arm in arms {
                    usage = usage.merge(self.expression_param_projection_usage(&arm.body, param_name));
                }
                usage
            }
            Statement::Defer { statement, .. } => {
                self.statement_param_projection_usage(statement, param_name)
            }
            _ => {
                if Self::statement_mentions_identifier(stmt, param_name) {
                    ProjectionUsage::BareOrOther
                } else {
                    ProjectionUsage::None
                }
            }
        }
    }

    fn expression_param_projection_usage(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
    ) -> ProjectionUsage {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => {
                ProjectionUsage::BareOrOther
            }
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                if matches!(
                    object,
                    Expression::Identifier { name, .. } if name == param_name
                ) {
                    // `param.field` / `param[i]` — projection root. Nested mentions inside
                    // index expressions are checked separately for Index.
                    if let Expression::Index { index, .. } = expr {
                        self.expression_param_projection_usage(index, param_name)
                            .merge(ProjectionUsage::FieldOrIndexOnly)
                    } else {
                        // Nested field chains (`param.a.b`) and method calls on fields
                        // (`param.a.len()`) are still field-projection-only.
                        ProjectionUsage::FieldOrIndexOnly
                    }
                } else {
                    let mut usage = self.expression_param_projection_usage(object, param_name);
                    if let Expression::Index { index, .. } = expr {
                        usage = usage
                            .merge(self.expression_param_projection_usage(index, param_name));
                    }
                    usage
                }
            }
            Expression::MethodCall { object, arguments, .. } => {
                // `param.field.method()` — projection as method receiver is readonly.
                // `callee(param.field)` — field value is moved into the callee.
                let mut usage = self.expression_param_projection_usage(object, param_name);
                for (_, arg) in arguments {
                    usage = usage.merge(self.expression_field_arg_usage(arg, param_name));
                }
                usage
            }
            Expression::Call { function, arguments, .. } => {
                let mut usage = self.expression_param_projection_usage(function, param_name);
                for (_, arg) in arguments {
                    usage = usage.merge(self.expression_field_arg_usage(arg, param_name));
                }
                usage
            }
            Expression::Binary { left, right, .. } => self
                .expression_param_projection_usage(left, param_name)
                .merge(self.expression_param_projection_usage(right, param_name)),
            Expression::Unary { operand, .. } => {
                self.expression_param_projection_usage(operand, param_name)
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                let mut usage = ProjectionUsage::None;
                for elem in elements {
                    usage = usage.merge(self.expression_param_projection_usage(elem, param_name));
                }
                usage
            }
            Expression::Block { statements, .. } => {
                let mut usage = ProjectionUsage::None;
                for s in statements {
                    usage = usage.merge(self.statement_param_projection_usage(s, param_name));
                }
                usage
            }
            Expression::StructLiteral { fields, .. } => {
                let mut usage = ProjectionUsage::None;
                for (_, value) in fields {
                    // Field values in struct literals are moves into owned payload.
                    usage = usage.merge(self.expression_field_arg_usage(value, param_name));
                }
                usage
            }
            _ => {
                if Self::expression_mentions_identifier(expr, param_name) {
                    ProjectionUsage::BareOrOther
                } else {
                    ProjectionUsage::None
                }
            }
        }
    }

    /// Call/struct args: a bare `param.field` is a field *move*, not a readonly projection.
    fn expression_field_arg_usage(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
    ) -> ProjectionUsage {
        match expr {
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. }
                if matches!(
                    object,
                    Expression::Identifier { name, .. } if name == param_name
                ) =>
            {
                ProjectionUsage::BareOrOther
            }
            Expression::FieldAccess { object, .. } => {
                // Nested `param.a.b` as a moved arg is still a move of projected data.
                if Self::expression_mentions_identifier(object, param_name) {
                    ProjectionUsage::BareOrOther
                } else {
                    self.expression_param_projection_usage(expr, param_name)
                }
            }
            _ => self.expression_param_projection_usage(expr, param_name),
        }
    }

    /// Promote owned params to borrowed when the body only forwards them to borrowing callees.
    fn promote_callee_forwarded_borrows(&mut self, func: &FunctionDecl<'ast>) {
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            if type_analysis::is_copy_type(&param.type_) {
                continue;
            }
            if self.inferred_borrowed_params.contains(&param.name)
                || self.inferred_mut_borrowed_params.contains(&param.name)
            {
                continue;
            }
            if self.current_struct_name.as_ref().is_some_and(|sn| {
                self.struct_is_owned_engine_key_facade(sn, param)
            }) {
                self.inferred_borrowed_params.remove(&param.name);
                continue;
            }
            if self.param_passed_to_borrowing_callee(func.body.as_slice(), &param.name, func)
                && !self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
                && !self.param_only_forwarded_to_qualified_collection_key_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                )
                && !self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
            {
                self.inferred_borrowed_params.insert(param.name.clone());
                if crate::codegen::rust::types::is_windjammer_text_type(&param.type_) {
                    self.str_ref_optimized_params.insert(param.name.clone());
                }
            }
        }
    }

    /// True when every use of `param_name` is passing it as a call/method argument (no field reads).
    pub(in crate::codegen::rust) fn param_only_used_as_call_argument(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.param_passed_as_call_argument(body, param_name, func)
            && !self.param_has_use_outside_call_arguments(body, param_name, func)
    }

    fn param_has_use_outside_call_arguments(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_has_use_outside_call_arguments(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    fn statement_has_use_outside_call_arguments(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_has_use_outside_call_arguments(expr, param_name, func, false)
            }
            Statement::Return { .. } => false,
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_has_use_outside_call_arguments(condition, param_name, func, false)
                    || self.param_has_use_outside_call_arguments(
                        then_block.as_slice(),
                        param_name,
                        func,
                    )
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_has_use_outside_call_arguments(b.as_slice(), param_name, func)
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_has_use_outside_call_arguments(condition, param_name, func, false)
                    || self.param_has_use_outside_call_arguments(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::For { body, iterable, .. } => {
                self.expression_has_use_outside_call_arguments(iterable, param_name, func, false)
                    || self.param_has_use_outside_call_arguments(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_has_use_outside_call_arguments(value, param_name, func, false)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_has_use_outside_call_arguments(b.as_slice(), param_name, func)
                    })
            }
            Statement::Match { value, arms, .. } => {
                self.expression_has_use_outside_call_arguments(value, param_name, func, false)
                    || arms.iter().any(|arm| {
                        self.expression_has_use_outside_call_arguments(
                            &arm.body,
                            param_name,
                            func,
                            false,
                        )
                    })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.param_has_use_outside_call_arguments(
                body.as_slice(),
                param_name,
                func,
            ),
            Statement::Assignment { value, .. } => {
                self.expression_has_use_outside_call_arguments(value, param_name, func, false)
            }
            _ => false,
        }
    }

    fn expression_has_use_outside_call_arguments(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        in_call_argument: bool,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => !in_call_argument,
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                self.expression_has_use_outside_call_arguments(object, param_name, func, false)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_use_outside_call_arguments(
                            arg,
                            param_name,
                            func,
                            true,
                        )
                    })
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_has_use_outside_call_arguments(function, param_name, func, false)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_use_outside_call_arguments(
                            arg,
                            param_name,
                            func,
                            true,
                        )
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_use_outside_call_arguments(left, param_name, func, in_call_argument)
                    || self.expression_has_use_outside_call_arguments(
                        right,
                        param_name,
                        func,
                        in_call_argument,
                    )
            }
            Expression::Unary { operand, .. } => self.expression_has_use_outside_call_arguments(
                operand,
                param_name,
                func,
                in_call_argument,
            ),
            Expression::FieldAccess { object, .. } => {
                self.expression_has_use_outside_call_arguments(object, param_name, func, in_call_argument)
            }
            Expression::Index { object, index, .. } => {
                self.expression_has_use_outside_call_arguments(object, param_name, func, in_call_argument)
                    || self.expression_has_use_outside_call_arguments(
                        index,
                        param_name,
                        func,
                        in_call_argument,
                    )
            }
            Expression::Block { statements, .. } => self.param_has_use_outside_call_arguments(
                statements.as_slice(),
                param_name,
                func,
            ),
            Expression::Tuple { elements, .. } => elements.iter().any(|elem| {
                self.expression_has_use_outside_call_arguments(
                    elem,
                    param_name,
                    func,
                    in_call_argument,
                )
            }),
            _ => false,
        }
    }

    /// Parameter ownership for signature registration before bodies are emitted.
    /// Uses per-file IR when available (matches formal emission), not only `current_ir_function`.
    pub(in crate::codegen::rust) fn get_param_ownership_for_registration(
        &self,
        _func_name: &str,
        param_name: &str,
        analyzed: &AnalyzedFunction<'_>,
    ) -> Option<crate::analyzer::OwnershipMode> {
        if self.ir_cutover.ownership {
            if let Some(ir_fn) = &self.current_ir_function {
                if let Some(safety_ty) = ir_fn.param_types.get(param_name) {
                    return Some(crate::codegen::rust::generator::owned_type_to_ownership_mode(
                        &safety_ty.ownership,
                    ));
                }
            }
        }
        analyzed.inferred_ownership.get(param_name).copied()
    }

    /// Register analyzed impl method signatures for call-site lookup (Self:: forward refs).
    pub(in crate::codegen::rust) fn register_impl_method_signature_from_analyzed(
        &mut self,
        impl_type: &str,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
    ) {
        let qualified = format!("{impl_type}::{}", func.name);
        let has_self_receiver = func.parameters.iter().any(|p| p.name == "self");
        let registry_sig = self.signature_registry.get_signature(&qualified);

        // Preregister/formal emission already converged emitted Rust contracts — do not downgrade.
        // Stale `Some([false, …])` from analyzer pre-registration must not block codegen refresh.
        if registry_sig
            .or_else(|| self.get_signature_with_global(&qualified))
            .is_some_and(|reg| {
                reg.emitted_rust_ref_params
                    .as_ref()
                    .is_some_and(|flags| flags.iter().any(|&f| f))
            })
        {
            return;
        }

        // Static methods: register analyzed signatures for call-site lookup, except when
        // body-inferred borrow would override an owned Copy formal (MannequinMesh::generate).
        if !has_self_receiver {
            if let Some(reg) = registry_sig {
                let skip_body_borrow_over_owned_copy = func.parameters.iter().enumerate().any(
                    |(idx, param)| {
                        if param.name == "self" {
                            return false;
                        }
                        let body_borrow = self
                            .get_param_ownership(&param.name, analyzed)
                            .is_some_and(|o| matches!(o, crate::analyzer::OwnershipMode::Borrowed));
                        body_borrow
                            && crate::codegen::rust::signature_promotion::param_type_is_owned_non_text(
                                reg, idx,
                            )
                            && reg
                                .formal_param_type(idx)
                                .is_some_and(crate::codegen::rust::type_analysis::is_copy_type)
                    },
                );
                if skip_body_borrow_over_owned_copy {
                    return;
                }
            }
        }

        let mut param_types = Vec::new();
        let mut formal_param_types = Vec::new();
        let mut param_ownership = Vec::new();

        for (idx, param) in func.parameters.iter().enumerate() {
            if param.name != "self" {
                let ast_owned_string =
                    crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                        && !matches!(param.type_, Type::Reference(_) | Type::MutableReference(_));
                let is_module_level = func.parent_type.is_none();

                let mut p_type = analyzed
                    .inferred_param_types
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| param.type_.clone());

                let mut ownership = if has_self_receiver {
                    self.get_param_ownership_for_registration(&func.name, &param.name, analyzed)
                        .unwrap_or(crate::analyzer::OwnershipMode::Owned)
                } else {
                    self.get_param_ownership_for_registration(&func.name, &param.name, analyzed)
                        .or_else(|| {
                            registry_sig.and_then(|sig| sig.param_ownership.get(idx).copied())
                        })
                        .unwrap_or(crate::analyzer::OwnershipMode::Owned)
                };

                // Module-level `string` formals stay owned unless discard/unused converges to `&str`.
                if ast_owned_string && is_module_level {
                    let discard_or_unused = self.param_only_used_in_simple_or_tuple_discard(
                        func.body.as_slice(),
                        &param.name,
                    ) || self
                        .compute_unused_formal_parameter_names(func)
                        .contains(&param.name)
                        || self.str_ref_optimized_params.contains(&param.name)
                        || self.inferred_borrowed_params.contains(&param.name);
                    if !discard_or_unused {
                        p_type = param.type_.clone();
                        ownership = crate::analyzer::OwnershipMode::Owned;
                    } else {
                        ownership = crate::analyzer::OwnershipMode::Borrowed;
                        p_type = Type::Reference(Box::new(Type::Custom("str".to_string())));
                    }
                } else if let Some(reg) = registry_sig {
                    if let Some(formal) = reg.formal_param_type(idx) {
                        if crate::codegen::rust::types::is_windjammer_text_type(formal)
                            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                            && is_module_level
                        {
                            let discard_or_unused = self.param_only_used_in_simple_or_tuple_discard(
                                func.body.as_slice(),
                                &param.name,
                            ) || self
                                .compute_unused_formal_parameter_names(func)
                                .contains(&param.name)
                                || self.str_ref_optimized_params.contains(&param.name)
                                || self.inferred_borrowed_params.contains(&param.name);
                            if !discard_or_unused {
                                p_type = formal.clone();
                                ownership = crate::analyzer::OwnershipMode::Owned;
                            }
                        }
                    }
                }

                if matches!(&p_type, Type::Reference(_)) && !ast_owned_string {
                    if !self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) {
                        ownership = crate::analyzer::OwnershipMode::Borrowed;
                    }
                } else if matches!(&p_type, Type::MutableReference(_)) && !ast_owned_string {
                    ownership = crate::analyzer::OwnershipMode::MutBorrowed;
                }

                // Store/lookup facades keep owned non-Copy custom formals (WDB-046),
                // except single-arg non-self owned forwards (`latest.has_key(key)` → `&Key` + clone).
                let keep_owned_facade = self.param_keeps_owned_engine_key_facade(impl_type, param, func);

                // Emitted Rust formals follow body-inferred ownership; IR param_types can
                // lag as Owned for non-Copy user types (e.g. wdb `Key` → `&Key` at call sites).
                if let Some(analyzed_own) = analyzed.inferred_ownership.get(&param.name) {
                    if matches!(
                        analyzed_own,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    ) && matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                        && !type_analysis::is_copy_type(&p_type)
                        && !keep_owned_facade
                        && !self.param_passes_to_wj_owned_sibling_call(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        )
                        && !self.param_has_forward_ref_keep_owned(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        )
                        && !self.current_fn_mixed_forwarder_params.contains(&param.name)
                    {
                        ownership = *analyzed_own;
                    }
                }

                // Phase-2 str_ref emits `&str` in Rust — register Borrowed so static
                // `Self::helper(self.field)` call sites borrow instead of cloning fields.
                // Never demote solver/analyzer Owned (enum payload, struct field store).
                if analyzed.str_ref_optimizable_params.contains(&param.name)
                    && !matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                    && analyzed.inferred_ownership.get(&param.name)
                        != Some(&crate::analyzer::OwnershipMode::Owned)
                {
                    ownership = crate::analyzer::OwnershipMode::Borrowed;
                }

                if matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                    && !type_analysis::is_copy_type(&p_type)
                    && !keep_owned_facade
                    && self.param_passed_to_borrowing_callee(func.body.as_slice(), &param.name, func)
                    && !self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
                    && !self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
                {
                    ownership = crate::analyzer::OwnershipMode::Borrowed;
                }

                if keep_owned_facade {
                    ownership = crate::analyzer::OwnershipMode::Owned;
                    p_type = param.type_.clone();
                }

                if !has_self_receiver {
                    if let Some(reg) = registry_sig {
                        if let Some(formal_ty) = reg.param_types.get(idx) {
                            if matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                                && !matches!(
                                    &p_type,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                            {
                                p_type = formal_ty.clone();
                            }
                        }
                    }
                }

                if self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
                {
                    ownership = crate::analyzer::OwnershipMode::Owned;
                    p_type = param.type_.clone();
                }

                let stored_type = match ownership {
                    crate::analyzer::OwnershipMode::Borrowed
                        if !matches!(&p_type, Type::Reference(_) | Type::MutableReference(_))
                            && !type_analysis::is_copy_type(&p_type) =>
                    {
                        if crate::codegen::rust::types::is_windjammer_text_type(&p_type) {
                            Type::Reference(Box::new(Type::Custom("str".into())))
                        } else {
                            Type::Reference(Box::new(p_type))
                        }
                    }
                    crate::analyzer::OwnershipMode::MutBorrowed
                        if !matches!(&p_type, Type::MutableReference(_)) =>
                    {
                        Type::MutableReference(Box::new(p_type))
                    }
                    _ => p_type,
                };
                param_types.push(stored_type);
                formal_param_types.push(param.type_.clone());
                param_ownership.push(ownership);
            }
        }

        let forwarding_borrow_params: Vec<bool> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .map(|p| {
                self.param_only_used_as_call_argument(func.body.as_slice(), &p.name, func)
                    && self.param_passed_to_borrowing_callee(
                        func.body.as_slice(),
                        &p.name,
                        func,
                    )
            })
            .collect();

        let signature = crate::codegen::rust::generator::MethodSignature {
            receiver_type: impl_type.to_string(),
            method_name: func.name.clone(),
            param_types,
            formal_param_types,
            param_ownership,
            return_type: func.return_type.clone(),
            has_self_receiver,
            forwarding_borrow_params,
            emitted_rust_ref_params: None,
        };

        self.register_method_signature(signature);
    }

    /// After prepare, align method registry with inferred borrows (emitted `&Key`, `&str`, etc.).
    pub(in crate::codegen::rust) fn sync_method_registry_from_inferred_borrows(
        &mut self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
    ) {
        let Some(impl_type) = self.current_struct_name.clone() else {
            return;
        };
        let borrow_flags: Vec<bool> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .map(|param| {
                let forwarded = self.param_passed_to_borrowing_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                );
                let owning = self.param_passes_to_wj_owned_sibling_call(
                    func.body.as_slice(),
                    &param.name,
                    func,
                );
                if owning
                    || self.current_fn_mixed_forwarder_params.contains(&param.name)
                    || self.param_has_forward_ref_keep_owned(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    || (self.param_keeps_owned_engine_key_facade(&impl_type, param, func)
                        && !self.param_only_used_via_field_or_index_projection(
                            func.body.as_slice(),
                            &param.name,
                        ))
                {
                    return false;
                }
                if !forwarded
                    && !self.param_used_as_read_operand(func.body.as_slice(), &param.name)
                    && !self.inferred_borrowed_params.contains(&param.name)
                    && !self.str_ref_optimized_params.contains(&param.name)
                {
                    return false;
                }
                self.inferred_borrowed_params.contains(&param.name)
                    || self
                        .get_param_ownership(&param.name, analyzed)
                        .is_some_and(|o| matches!(o, crate::analyzer::OwnershipMode::Borrowed))
                    || analyzed
                        .inferred_ownership
                        .get(&param.name)
                        .is_some_and(|o| matches!(o, crate::analyzer::OwnershipMode::Borrowed))
                    || self.param_used_as_read_operand(func.body.as_slice(), &param.name)
                    || self.str_ref_optimized_params.contains(&param.name)
            })
            .collect();
        let Some(methods) = self.method_signatures_by_type.get_mut(&impl_type) else {
            return;
        };
        let Some(sig) = methods.get_mut(func.name.as_str()) else {
            return;
        };
        let mut param_idx = 0;
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            if param_idx >= sig.param_types.len() {
                break;
            }
            let should_borrow = borrow_flags.get(param_idx).copied().unwrap_or(false);
            if should_borrow && !matches!(sig.param_types[param_idx], Type::Reference(_))
            {
                sig.param_types[param_idx] = Type::Reference(Box::new(param.type_.clone()));
                if param_idx < sig.param_ownership.len() {
                    sig.param_ownership[param_idx] = crate::analyzer::OwnershipMode::Borrowed;
                }
            } else if self.inferred_mut_borrowed_params.contains(&param.name)
                && !matches!(sig.param_types[param_idx], Type::MutableReference(_))
            {
                sig.param_types[param_idx] =
                    Type::MutableReference(Box::new(param.type_.clone()));
                if param_idx < sig.param_ownership.len() {
                    sig.param_ownership[param_idx] = crate::analyzer::OwnershipMode::MutBorrowed;
                }
            }
            param_idx += 1;
        }
    }

    /// True when `param_name` is the iterable in `for _ in param` (consumes the collection).
    pub(in crate::codegen::rust) fn param_consumed_as_for_loop_iterable(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        for stmt in body {
            match stmt {
                Statement::For { iterable, body, .. } => {
                    if let Expression::Identifier { name, .. } = iterable {
                        if name == param_name {
                            return true;
                        }
                    }
                    if self.param_consumed_as_for_loop_iterable(body.as_slice(), param_name) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.param_consumed_as_for_loop_iterable(then_block.as_slice(), param_name) {
                        return true;
                    }
                    if let Some(e) = else_block {
                        if self.param_consumed_as_for_loop_iterable(e.as_slice(), param_name) {
                            return true;
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if self.param_consumed_as_for_loop_iterable(body.as_slice(), param_name) {
                        return true;
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            if self.param_consumed_as_for_loop_iterable(statements, param_name) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// True when `param_name` appears in a read context (comparison, field access chain, etc.).
    pub(in crate::codegen::rust) fn param_used_as_read_operand(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        for stmt in body {
            if self.statement_uses_param_as_read_operand(stmt, param_name) {
                return true;
            }
        }
        false
    }

    fn statement_uses_param_as_read_operand(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_uses_param_as_read_operand(expr, param_name)
            }
            Statement::Return { .. } => false,
            Statement::Let { pattern, value, else_block, .. } => {
                let discard = Self::is_discarding_let_pattern(pattern);
                (!discard && self.expression_uses_param_as_read_operand(value, param_name))
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_used_as_read_operand(b.as_slice(), param_name)
                    })
            }
            Statement::Assignment { value, .. } => {
                self.expression_uses_param_as_read_operand(value, param_name)
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_uses_param_as_read_operand(condition, param_name)
                    || self.param_used_as_read_operand(then_block.as_slice(), param_name)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_used_as_read_operand(b.as_slice(), param_name)
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_uses_param_as_read_operand(condition, param_name)
                    || self.param_used_as_read_operand(body.as_slice(), param_name)
            }
            Statement::For { body, iterable, .. } => {
                // `for x in items` consumes `items` — not a readonly operand (WDB-006).
                let iterable_consumes = matches!(
                    iterable,
                    Expression::Identifier { name, .. } if name == param_name
                );
                (!iterable_consumes
                    && self.expression_uses_param_as_read_operand(iterable, param_name))
                    || self.param_used_as_read_operand(body.as_slice(), param_name)
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.param_used_as_read_operand(body.as_slice(), param_name)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_uses_param_as_read_operand(value, param_name)
                    || arms.iter().any(|arm| {
                        self.expression_uses_param_as_read_operand(&arm.body, param_name)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.statement_uses_param_as_read_operand(statement, param_name)
            }
            _ => false,
        }
    }

    fn expression_uses_param_as_read_operand(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => true,
            Expression::Binary { left, right, .. } => {
                self.expression_uses_param_as_read_operand(left, param_name)
                    || self.expression_uses_param_as_read_operand(right, param_name)
            }
            Expression::Unary { operand, .. } => {
                self.expression_uses_param_as_read_operand(operand, param_name)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_uses_param_as_read_operand(object, param_name)
            }
            Expression::Index { object, index, .. } => {
                self.expression_uses_param_as_read_operand(object, param_name)
                    || self.expression_uses_param_as_read_operand(index, param_name)
            }
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                self.expression_uses_param_as_read_operand(object, param_name)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_uses_param_as_read_operand(arg, param_name)
                    })
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_uses_param_as_read_operand(function, param_name)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_uses_param_as_read_operand(arg, param_name)
                    })
            }
            Expression::Block { statements, .. } => {
                self.param_used_as_read_operand(statements.as_slice(), param_name)
            }
            Expression::Tuple { elements, .. } => elements.iter().any(|elem| {
                self.expression_uses_param_as_read_operand(elem, param_name)
            }),
            Expression::Array { elements, .. } => elements.iter().any(|elem| {
                self.expression_uses_param_as_read_operand(elem, param_name)
            }),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn param_has_owning_method_use(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_has_owning_method_use(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    fn statement_has_owning_method_use(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_has_owning_method_use(expr, param_name, func)
            }
            Statement::Return { .. } => false,
            Statement::Let { value, else_block, .. } => {
                self.expression_has_owning_method_use(value, param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_has_owning_method_use(b.as_slice(), param_name, func)
                    })
            }
            Statement::Assignment { value, .. } => {
                self.expression_has_owning_method_use(value, param_name, func)
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_has_owning_method_use(condition, param_name, func)
                    || self.param_has_owning_method_use(then_block.as_slice(), param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_has_owning_method_use(b.as_slice(), param_name, func)
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_has_owning_method_use(condition, param_name, func)
                    || self.param_has_owning_method_use(body.as_slice(), param_name, func)
            }
            Statement::For { body, iterable, .. } => {
                self.expression_has_owning_method_use(iterable, param_name, func)
                    || self.param_has_owning_method_use(body.as_slice(), param_name, func)
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.param_has_owning_method_use(body.as_slice(), param_name, func)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_has_owning_method_use(value, param_name, func)
                    || arms.iter().any(|arm| {
                        self.expression_has_owning_method_use(&arm.body, param_name, func)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.statement_has_owning_method_use(statement, param_name, func)
            }
            _ => false,
        }
    }

    fn expression_has_owning_method_use(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                        if self.method_call_arg_expects_borrow(object, method, i, func) {
                            continue;
                        }
                        if self.method_call_sibling_ast_expects_owned_arg(
                            object, method, i, func,
                        ) {
                            return true;
                        }
                        if self.method_call_arg_formal_is_owned_non_copy(object, method, i, func) {
                            return true;
                        }
                    }
                }
                self.expression_has_owning_method_use(object, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_owning_method_use(arg, param_name, func)
                    })
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_has_owning_method_use(function, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_owning_method_use(arg, param_name, func)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_owning_method_use(left, param_name, func)
                    || self.expression_has_owning_method_use(right, param_name, func)
            }
            Expression::Unary { operand, .. } => {
                self.expression_has_owning_method_use(operand, param_name, func)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_has_owning_method_use(object, param_name, func)
            }
            Expression::Index { object, index, .. } => {
                self.expression_has_owning_method_use(object, param_name, func)
                    || self.expression_has_owning_method_use(index, param_name, func)
            }
            Expression::Block { statements, .. } => {
                self.param_has_owning_method_use(statements.as_slice(), param_name, func)
            }
            _ => false,
        }
    }

    /// `if f(param) { ... param ... } else { ... param ... }` — owned outer formal (wdb put_value).
    pub(in crate::codegen::rust) fn param_used_in_if_with_condition_and_branches(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        body.iter()
            .any(|stmt| self.stmt_has_if_with_condition_and_branches(param_name, stmt))
    }

    /// `if cond { ... param ... } else { ... param ... }` — keep owned outer param (value in both arms).
    pub(in crate::codegen::rust) fn param_used_in_if_else_both_branches(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        body.iter()
            .any(|stmt| self.stmt_has_if_else_both_branches(param_name, stmt))
    }

    fn stmt_has_if_else_both_branches(&self, param_name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::If {
                then_block,
                else_block: Some(else_block),
                ..
            } => {
                self.stmts_mention_param_name(param_name, then_block)
                    && self.stmts_mention_param_name(param_name, else_block)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.stmts_have_if_else_both_branches(param_name, then_block)
                    || else_block.as_ref().is_some_and(|block| {
                        self.stmts_have_if_else_both_branches(param_name, block)
                    })
            }
            _ => false,
        }
    }

    fn stmts_have_if_else_both_branches(
        &self,
        param_name: &str,
        stmts: &[&'ast Statement<'ast>],
    ) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_has_if_else_both_branches(param_name, stmt))
    }

    fn stmt_has_if_with_condition_and_branches(
        &self,
        param_name: &str,
        stmt: &Statement<'ast>,
    ) -> bool {
        match stmt {
            Statement::If {
                condition,
                then_block,
                else_block: Some(else_block),
                ..
            } => {
                self.expr_mentions_param_name(param_name, condition)
                    && self.stmts_mention_param_name(param_name, then_block)
                    && self.stmts_mention_param_name(param_name, else_block)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.stmts_have_if_with_condition_and_branches(param_name, then_block)
                    || else_block.as_ref().is_some_and(|block| {
                        self.stmts_have_if_with_condition_and_branches(param_name, block)
                    })
            }
            _ => false,
        }
    }

    fn stmts_have_if_with_condition_and_branches(
        &self,
        param_name: &str,
        stmts: &[&'ast Statement<'ast>],
    ) -> bool {
        stmts.iter().any(|stmt| {
            self.stmt_has_if_with_condition_and_branches(param_name, stmt)
        })
    }

    fn stmts_mention_param_name(
        &self,
        param_name: &str,
        stmts: &[&'ast Statement<'ast>],
    ) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_mentions_param_name(param_name, stmt))
    }

    fn stmt_mentions_param_name(&self, param_name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expr_mentions_param_name(param_name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_mentions_param_name(param_name, expr),
            Statement::Let { value, .. } => self.expr_mentions_param_name(param_name, value),
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expr_mentions_param_name(param_name, condition)
                    || self.stmts_mention_param_name(param_name, then_block)
                    || else_block.as_ref().is_some_and(|block| {
                        self.stmts_mention_param_name(param_name, block)
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expr_mentions_param_name(param_name, condition)
                    || self.stmts_mention_param_name(param_name, body)
            }
            Statement::For { body, iterable, .. } => {
                self.expr_mentions_param_name(param_name, iterable)
                    || self.stmts_mention_param_name(param_name, body)
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.stmts_mention_param_name(param_name, body),
            Statement::Match { value, arms, .. } => {
                self.expr_mentions_param_name(param_name, value)
                    || arms.iter().any(|arm| {
                        self.expr_mentions_param_name(param_name, &arm.body)
                    })
            }
            _ => false,
        }
    }

    fn expr_mentions_param_name(&self, param_name: &str, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == param_name,
            Expression::FieldAccess { object, .. } => {
                self.expr_mentions_param_name(param_name, object)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_mentions_param_name(param_name, function)
                    || arguments.iter().any(|(_, arg)| {
                        self.expr_mentions_param_name(param_name, arg)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_param_name(param_name, left)
                    || self.expr_mentions_param_name(param_name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expr_mentions_param_name(param_name, operand)
            }
            Expression::TryOp { expr, .. } => self.expr_mentions_param_name(param_name, expr),
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                self.expr_mentions_param_name(param_name, object)
                    || arguments.iter().any(|(_, arg)| {
                        self.expr_mentions_param_name(param_name, arg)
                    })
            }
            Expression::Block { statements, .. } => {
                self.stmts_mention_param_name(param_name, statements)
            }
            _ => false,
        }
    }

    /// True when the body forwards `param_name` to a borrowing callee and also reads
    /// the param afterward (forward-ref pattern: keep owned formal, borrow at call site).
    pub(in crate::codegen::rust) fn param_has_forward_ref_keep_owned(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if self.param_used_in_if_with_condition_and_branches(body, param_name) {
            return true;
        }
        if !self.param_used_as_read_operand(body, param_name) {
            return false;
        }
        if self.param_passes_to_wj_owned_sibling_call(body, param_name, func) {
            return false;
        }
        // Pure delegation (only forwarded as call args) keeps borrowed formals.
        if self.param_only_used_as_call_argument(body, param_name, func) {
            return false;
        }
        self.param_passed_to_borrowing_callee(body, param_name, func)
            || self.param_passed_as_call_argument(body, param_name, func)
    }

    /// True when `param_name` is passed as a direct argument to any call or method.
    pub(in crate::codegen::rust) fn param_passed_as_call_argument(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_param_as_call_argument(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    fn statement_passes_param_as_call_argument(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                // Bare `path` as a statement expression is not a call argument.
                if matches!(expr, Expression::Identifier { name, .. } if name == param_name) {
                    return false;
                }
                self.expression_passes_param_as_call_argument(expr, param_name, func)
            }
            Statement::Let { pattern, value, else_block, .. } => {
                // Discard bindings (`let _ = path` / `let _ = (path, x)`) mention the
                // param without passing it to a callee — do not treat as call-arg use.
                // Only nested Call/MethodCall nodes inside the discard value count.
                let from_value = if Self::is_discarding_let_pattern(pattern) {
                    self.expression_has_nested_call_passing_param(value, param_name, func)
                } else {
                    self.expression_passes_param_as_call_argument(value, param_name, func)
                };
                from_value
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_as_call_argument(b.as_slice(), param_name, func)
                    })
            }
            Statement::Assignment { value, .. } => {
                self.expression_passes_param_as_call_argument(value, param_name, func)
            }
            Statement::Return { value, .. } => value
                .is_some_and(|v| self.expression_passes_param_as_call_argument(v, param_name, func)),
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_param_as_call_argument(condition, param_name, func)
                    || self.param_passed_as_call_argument(then_block.as_slice(), param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_as_call_argument(b.as_slice(), param_name, func)
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_param_as_call_argument(condition, param_name, func)
                    || self.param_passed_as_call_argument(body.as_slice(), param_name, func)
            }
            Statement::For { body, iterable, .. } => {
                // `for x in items` is iteration, not a call-argument use of `items`.
                // Only nested Call/MethodCall nodes inside the iterable count.
                self.expression_has_nested_call_passing_param(iterable, param_name, func)
                    || self.param_passed_as_call_argument(body.as_slice(), param_name, func)
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.param_passed_as_call_argument(body.as_slice(), param_name, func)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_passes_param_as_call_argument(value, param_name, func)
                    || arms.iter().any(|arm| {
                        self.expression_passes_param_as_call_argument(&arm.body, param_name, func)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.statement_passes_param_as_call_argument(statement, param_name, func)
            }
            _ => false,
        }
    }

    /// True when `expr` contains a Call/MethodCall that receives `param_name` as an argument.
    /// Unlike [`expression_passes_param_as_call_argument`], bare identifiers and tuples that
    /// merely mention the param (e.g. discard bindings) return false.
    fn expression_has_nested_call_passing_param(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                arguments.iter().any(|(_, arg)| {
                    self.expression_passes_param_as_call_argument(arg, param_name, func)
                }) || self.expression_has_nested_call_passing_param(function, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_nested_call_passing_param(arg, param_name, func)
                    })
            }
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                arguments.iter().any(|(_, arg)| {
                    self.expression_passes_param_as_call_argument(arg, param_name, func)
                }) || self.expression_has_nested_call_passing_param(object, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_has_nested_call_passing_param(arg, param_name, func)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_nested_call_passing_param(left, param_name, func)
                    || self.expression_has_nested_call_passing_param(right, param_name, func)
            }
            Expression::Unary { operand, .. } => {
                self.expression_has_nested_call_passing_param(operand, param_name, func)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_has_nested_call_passing_param(object, param_name, func)
            }
            Expression::Index { object, index, .. } => {
                self.expression_has_nested_call_passing_param(object, param_name, func)
                    || self.expression_has_nested_call_passing_param(index, param_name, func)
            }
            Expression::Tuple { elements, .. } => elements.iter().any(|e| {
                self.expression_has_nested_call_passing_param(e, param_name, func)
            }),
            Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, v)| {
                self.expression_has_nested_call_passing_param(v, param_name, func)
            }),
            Expression::Block { statements, .. } => {
                self.param_passed_as_call_argument(statements.as_slice(), param_name, func)
            }
            _ => false,
        }
    }

    fn expression_passes_param_as_call_argument(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            // Bare identifiers are not call arguments; Call/MethodCall arms detect
            // params inside argument expressions. Returning true here made every
            // comparison (`item == id`) look like a call-arg use and forced Owned
            // formals via `param_has_forward_ref_keep_owned` (Inventory::has).
            Expression::Identifier { .. } => false,
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                arguments.iter().any(|(_, arg)| {
                    matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        || self.expression_passes_param_as_call_argument(arg, param_name, func)
                }) || self.expression_passes_param_as_call_argument(object, param_name, func)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                arguments.iter().any(|(_, arg)| {
                    matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        || self.expression_passes_param_as_call_argument(arg, param_name, func)
                }) || self.expression_passes_param_as_call_argument(function, param_name, func)
            }
            Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, v)| {
                self.expression_passes_param_as_call_argument(v, param_name, func)
            }),
            Expression::Tuple { elements, .. } => elements.iter().any(|e| {
                self.expression_passes_param_as_call_argument(e, param_name, func)
            }),
            Expression::Binary { left, right, .. } => {
                self.expression_passes_param_as_call_argument(left, param_name, func)
                    || self.expression_passes_param_as_call_argument(right, param_name, func)
            }
            Expression::Unary { operand, .. } => {
                self.expression_passes_param_as_call_argument(operand, param_name, func)
            }
            // Field reads / indexing are not call-argument uses of the root binding.
            Expression::FieldAccess { object, .. } => {
                self.expression_has_nested_call_passing_param(object, param_name, func)
            }
            Expression::Index { object, index, .. } => {
                self.expression_has_nested_call_passing_param(object, param_name, func)
                    || self.expression_has_nested_call_passing_param(index, param_name, func)
            }
            Expression::Block { statements, .. } => {
                self.param_passed_as_call_argument(statements.as_slice(), param_name, func)
            }
            _ => false,
        }
    }

    /// True when the body forwards `param_name` to a method that already expects a borrow.
    pub(in crate::codegen::rust) fn param_passed_to_borrowing_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_param_to_borrowing_callee(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    /// True when every direct method-arg use of `param_name` is a map/set key lookup.
    pub(in crate::codegen::rust) fn param_only_forwarded_to_collection_key_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.param_only_forwarded_to_key_callee(body, param_name, func, true)
    }

    pub(in crate::codegen::rust) fn param_only_forwarded_to_map_key_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.param_only_forwarded_to_key_callee(body, param_name, func, false)
    }

    fn param_only_forwarded_to_key_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
        include_set_lookup: bool,
    ) -> bool {
        let mut key_uses = 0usize;
        let mut other_direct_uses = 0usize;
        self.collect_param_direct_method_arg_uses(body, param_name, func, &mut |object, method, arg_index| {
            if self.method_arg_is_key_method(object, method, arg_index, func, include_set_lookup) {
                key_uses += 1;
            } else {
                other_direct_uses += 1;
            }
        });
        key_uses > 0 && other_direct_uses == 0
    }

    /// FMP/wdb: module helpers on qualified `std::collections::HashMap` keep owned `String` keys.
    pub(in crate::codegen::rust) fn param_only_forwarded_to_qualified_collection_key_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.param_only_forwarded_to_qualified_key_callee(body, param_name, func, true)
    }

    pub(in crate::codegen::rust) fn param_only_forwarded_to_qualified_map_key_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.param_only_forwarded_to_qualified_key_callee(body, param_name, func, false)
    }

    fn param_only_forwarded_to_qualified_key_callee(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
        include_set_lookup: bool,
    ) -> bool {
        if func.parent_type.is_some() {
            return false;
        }
        let mut key_uses = 0usize;
        let mut other_direct_uses = 0usize;
        self.collect_param_direct_method_arg_uses(body, param_name, func, &mut |object, method, arg_index| {
            if self.method_arg_is_qualified_key(object, method, arg_index, func, include_set_lookup) {
                key_uses += 1;
            } else {
                other_direct_uses += 1;
            }
        });
        key_uses > 0 && other_direct_uses == 0
    }

    fn receiver_param_type_from_expr<'b>(
        &self,
        object: &'ast Expression<'ast>,
        func: &'b FunctionDecl<'ast>,
    ) -> Option<&'b Type> {
        if let Expression::Identifier { name, .. } = object {
            func.parameters.iter().find(|p| p.name == *name).map(|p| &p.type_)
        } else {
            None
        }
    }

    fn collect_param_direct_method_arg_uses<F>(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visitor: &mut F,
    ) where
        F: FnMut(&Expression<'ast>, &str, usize),
    {
        for stmt in body {
            self.statement_collect_param_direct_method_arg_uses(stmt, param_name, func, visitor);
        }
    }

    fn statement_collect_param_direct_method_arg_uses<F>(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visitor: &mut F,
    ) where
        F: FnMut(&Expression<'ast>, &str, usize),
    {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_collect_param_direct_method_arg_uses(expr, param_name, func, visitor);
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_collect_param_direct_method_arg_uses(value, param_name, func, visitor);
                if let Some(b) = else_block {
                    self.collect_param_direct_method_arg_uses(b.as_slice(), param_name, func, visitor);
                }
            }
            Statement::Assignment { value, .. } => {
                self.expression_collect_param_direct_method_arg_uses(value, param_name, func, visitor);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.expression_collect_param_direct_method_arg_uses(v, param_name, func, visitor);
                }
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_collect_param_direct_method_arg_uses(condition, param_name, func, visitor);
                self.collect_param_direct_method_arg_uses(then_block.as_slice(), param_name, func, visitor);
                if let Some(b) = else_block {
                    self.collect_param_direct_method_arg_uses(b.as_slice(), param_name, func, visitor);
                }
            }
            Statement::While { body, condition, .. } => {
                self.expression_collect_param_direct_method_arg_uses(condition, param_name, func, visitor);
                self.collect_param_direct_method_arg_uses(body.as_slice(), param_name, func, visitor);
            }
            Statement::For { body, iterable, .. } => {
                self.expression_collect_param_direct_method_arg_uses(iterable, param_name, func, visitor);
                self.collect_param_direct_method_arg_uses(body.as_slice(), param_name, func, visitor);
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.collect_param_direct_method_arg_uses(body.as_slice(), param_name, func, visitor);
            }
            Statement::Match { value, arms, .. } => {
                self.expression_collect_param_direct_method_arg_uses(value, param_name, func, visitor);
                for arm in arms {
                    self.expression_collect_param_direct_method_arg_uses(&arm.body, param_name, func, visitor);
                }
            }
            Statement::Defer { statement, .. } => {
                self.statement_collect_param_direct_method_arg_uses(statement, param_name, func, visitor);
            }
            _ => {}
        }
    }

    fn expression_collect_param_direct_method_arg_uses<F>(
        &self,
        expr: &'ast Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visitor: &mut F,
    ) where
        F: FnMut(&Expression<'ast>, &str, usize),
    {
        if let Some((object, method, arguments)) =
            crate::analyzer::stdlib_method_traits::decompose_collection_key_lookup(expr)
        {
            for (i, (_, arg)) in arguments.iter().enumerate() {
                if crate::codegen::rust::expression_helpers::expression_directly_uses_binding(
                    arg, param_name,
                ) {
                    visitor(object, method, i);
                }
            }
            self.expression_collect_param_direct_method_arg_uses(object, param_name, func, visitor);
            for (_, arg) in arguments {
                self.expression_collect_param_direct_method_arg_uses(arg, param_name, func, visitor);
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
                    if crate::codegen::rust::expression_helpers::expression_directly_uses_binding(
                        arg, param_name,
                    ) {
                        visitor(object, method, i);
                    }
                }
                self.expression_collect_param_direct_method_arg_uses(object, param_name, func, visitor);
                for (_, arg) in arguments {
                    self.expression_collect_param_direct_method_arg_uses(arg, param_name, func, visitor);
                }
            }
            Expression::Call { arguments, function, .. } => {
                self.expression_collect_param_direct_method_arg_uses(function, param_name, func, visitor);
                for (_, arg) in arguments {
                    self.expression_collect_param_direct_method_arg_uses(arg, param_name, func, visitor);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.expression_collect_param_direct_method_arg_uses(left, param_name, func, visitor);
                self.expression_collect_param_direct_method_arg_uses(right, param_name, func, visitor);
            }
            Expression::Unary { operand, .. } => {
                self.expression_collect_param_direct_method_arg_uses(operand, param_name, func, visitor);
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_collect_param_direct_method_arg_uses(object, param_name, func, visitor);
            }
            Expression::Index { object, index, .. } => {
                self.expression_collect_param_direct_method_arg_uses(object, param_name, func, visitor);
                self.expression_collect_param_direct_method_arg_uses(index, param_name, func, visitor);
            }
            Expression::Block { statements, .. } => {
                self.collect_param_direct_method_arg_uses(statements.as_slice(), param_name, func, visitor);
            }
            _ => {}
        }
    }

    fn method_arg_is_collection_key(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.method_arg_is_key_method(object, method, arg_index, func, true)
    }

    fn method_arg_is_key_method(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
        include_set_lookup: bool,
    ) -> bool {
        if arg_index != 0 {
            return false;
        }
        let is_key_method = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
            || (include_set_lookup
                && crate::codegen::rust::stdlib_method_traits::is_set_lookup_method(method));
        if !is_key_method {
            return false;
        }
        if let Expression::Identifier { name, .. } = object {
            if let Some(param) = func.parameters.iter().find(|p| p.name == *name) {
                if crate::codegen::rust::stdlib_method_traits::is_map_type(&param.type_) {
                    return true;
                }
                if include_set_lookup
                    && crate::codegen::rust::stdlib_method_traits::is_set_type(&param.type_)
                {
                    return true;
                }
            }
        }
        if let Expression::FieldAccess { object, field, .. } = object {
            if matches!(&**object, Expression::Identifier { name, .. } if name == "self") {
                if let Some(sn) = self.current_struct_name.as_ref() {
                    if let Some(fields) = self.lookup_struct_field_types(sn) {
                        if let Some(field_ty) = fields.get(field.as_str()) {
                            if crate::codegen::rust::stdlib_method_traits::is_map_type(field_ty) {
                                return true;
                            }
                            if include_set_lookup
                                && crate::codegen::rust::stdlib_method_traits::is_set_type(field_ty)
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        let Some(rt) = (if let Expression::Identifier { name, .. } = object {
            func.parameters
                .iter()
                .find(|p| p.name == *name)
                .and_then(|p| Self::type_to_name(&p.type_))
                .or_else(|| self.infer_local_binding_type_name(func.body.as_slice(), name))
        } else {
            None
        })
        .or_else(|| self.mc_infer_method_receiver_type_name(object))
        .or_else(|| self.infer_type_name(object))
        else {
            return false;
        };
        let base = rt.split('<').next().unwrap_or(rt.as_str());
        crate::codegen::rust::stdlib_method_traits::is_map_type_name(base)
            || (include_set_lookup
                && crate::codegen::rust::stdlib_method_traits::is_set_type_name(base))
    }

    fn method_arg_is_qualified_collection_key(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.method_arg_is_qualified_key(object, method, arg_index, func, true)
    }

    fn method_arg_is_qualified_key(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
        include_set_lookup: bool,
    ) -> bool {
        self.method_arg_is_key_method(object, method, arg_index, func, include_set_lookup)
            && self
                .receiver_param_type_from_expr(object, func)
                .is_some_and(crate::analyzer::stdlib_method_traits::is_qualified_map_type)
    }

    fn collect_param_borrowing_method_uses<F>(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visitor: &mut F,
    ) where
        F: FnMut(&Expression<'ast>, &str, usize),
    {
        for stmt in body {
            self.statement_collect_param_borrowing_method_uses(stmt, param_name, func, visitor);
        }
    }

    fn statement_collect_param_borrowing_method_uses<F>(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visitor: &mut F,
    ) where
        F: FnMut(&Expression<'ast>, &str, usize),
    {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_collect_param_borrowing_method_uses(expr, param_name, func, visitor);
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_collect_param_borrowing_method_uses(value, param_name, func, visitor);
                if let Some(b) = else_block {
                    self.collect_param_borrowing_method_uses(b.as_slice(), param_name, func, visitor);
                }
            }
            Statement::Assignment { value, .. } => {
                self.expression_collect_param_borrowing_method_uses(value, param_name, func, visitor);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.expression_collect_param_borrowing_method_uses(v, param_name, func, visitor);
                }
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_collect_param_borrowing_method_uses(condition, param_name, func, visitor);
                self.collect_param_borrowing_method_uses(then_block.as_slice(), param_name, func, visitor);
                if let Some(b) = else_block {
                    self.collect_param_borrowing_method_uses(b.as_slice(), param_name, func, visitor);
                }
            }
            Statement::While { body, condition, .. } => {
                self.expression_collect_param_borrowing_method_uses(condition, param_name, func, visitor);
                self.collect_param_borrowing_method_uses(body.as_slice(), param_name, func, visitor);
            }
            Statement::For { body, iterable, .. } => {
                self.expression_collect_param_borrowing_method_uses(iterable, param_name, func, visitor);
                self.collect_param_borrowing_method_uses(body.as_slice(), param_name, func, visitor);
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.collect_param_borrowing_method_uses(body.as_slice(), param_name, func, visitor);
            }
            Statement::Match { value, arms, .. } => {
                self.expression_collect_param_borrowing_method_uses(value, param_name, func, visitor);
                for arm in arms {
                    self.expression_collect_param_borrowing_method_uses(&arm.body, param_name, func, visitor);
                }
            }
            Statement::Defer { statement, .. } => {
                self.statement_collect_param_borrowing_method_uses(statement, param_name, func, visitor);
            }
            _ => {}
        }
    }

    fn expression_collect_param_borrowing_method_uses<F>(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visitor: &mut F,
    ) where
        F: FnMut(&Expression<'ast>, &str, usize),
    {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        && self.method_call_arg_expects_borrow(object, method, i, func)
                    {
                        visitor(object, method, i);
                    }
                }
                self.expression_collect_param_borrowing_method_uses(object, param_name, func, visitor);
                for (_, arg) in arguments {
                    self.expression_collect_param_borrowing_method_uses(arg, param_name, func, visitor);
                }
            }
            Expression::Call { arguments, function, .. } => {
                self.expression_collect_param_borrowing_method_uses(function, param_name, func, visitor);
                for (_, arg) in arguments {
                    self.expression_collect_param_borrowing_method_uses(arg, param_name, func, visitor);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.expression_collect_param_borrowing_method_uses(left, param_name, func, visitor);
                self.expression_collect_param_borrowing_method_uses(right, param_name, func, visitor);
            }
            Expression::Unary { operand, .. } => {
                self.expression_collect_param_borrowing_method_uses(operand, param_name, func, visitor);
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_collect_param_borrowing_method_uses(object, param_name, func, visitor);
            }
            Expression::Index { object, index, .. } => {
                self.expression_collect_param_borrowing_method_uses(object, param_name, func, visitor);
                self.expression_collect_param_borrowing_method_uses(index, param_name, func, visitor);
            }
            Expression::Block { statements, .. } => {
                self.collect_param_borrowing_method_uses(statements.as_slice(), param_name, func, visitor);
            }
            _ => {}
        }
    }

    fn statement_passes_param_to_borrowing_callee(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_passes_param_to_borrowing_callee(expr, param_name, func)
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_param_to_borrowing_callee(value, param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_to_borrowing_callee(b.as_slice(), param_name, func)
                    })
            }
            Statement::Assignment { value, .. } => {
                self.expression_passes_param_to_borrowing_callee(value, param_name, func)
            }
            Statement::Return { value, .. } => value
                .is_some_and(|v| self.expression_passes_param_to_borrowing_callee(v, param_name, func)),
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_param_to_borrowing_callee(condition, param_name, func)
                    || self.param_passed_to_borrowing_callee(then_block.as_slice(), param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_to_borrowing_callee(b.as_slice(), param_name, func)
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_param_to_borrowing_callee(condition, param_name, func)
                    || self.param_passed_to_borrowing_callee(body.as_slice(), param_name, func)
            }
            Statement::For { body, iterable, .. } => {
                self.expression_passes_param_to_borrowing_callee(iterable, param_name, func)
                    || self.param_passed_to_borrowing_callee(body.as_slice(), param_name, func)
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.param_passed_to_borrowing_callee(body.as_slice(), param_name, func)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_passes_param_to_borrowing_callee(value, param_name, func)
                    || arms.iter().any(|arm| {
                        self.expression_passes_param_to_borrowing_callee(&arm.body, param_name, func)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.statement_passes_param_to_borrowing_callee(statement, param_name, func)
            }
            _ => false,
        }
    }

    fn infer_local_binding_type_name(
        &self,
        body: &[&'ast Statement<'ast>],
        name: &str,
    ) -> Option<String> {
        for stmt in body {
            if let Statement::Let {
                pattern: Pattern::Identifier(binding),
                value,
                else_block,
                ..
            } = stmt
            {
                if binding == name {
                    return self
                        .infer_expression_type(value)
                        .and_then(|t| super::CodeGenerator::type_to_name(&t));
                }
                if let Some(else_body) = else_block {
                    if let Some(tn) =
                        self.infer_local_binding_type_name(else_body.as_slice(), name)
                    {
                        return Some(tn);
                    }
                }
            }
            let nested = match stmt {
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    let mut blocks = vec![then_block.as_slice()];
                    if let Some(e) = else_block {
                        blocks.push(e.as_slice());
                    }
                    blocks
                }
                Statement::While { body, .. }
                | Statement::Loop { body, .. }
                | Statement::Thread { body, .. }
                | Statement::Async { body, .. } => vec![body.as_slice()],
                Statement::For { body, .. } => vec![body.as_slice()],
                Statement::Match { arms, .. } => arms
                    .iter()
                    .filter_map(|arm| match &arm.body {
                        Expression::Block { statements, .. } => Some(statements.as_slice()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            for block in nested {
                if let Some(tn) = self.infer_local_binding_type_name(block, name) {
                    return Some(tn);
                }
            }
        }
        None
    }

    fn expression_passes_param_to_borrowing_callee(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        && self.method_call_arg_expects_borrow(object, method, i, func)
                    {
                        return true;
                    }
                }
                self.expression_passes_param_to_borrowing_callee(object, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_passes_param_to_borrowing_callee(arg, param_name, func)
                    })
            }
            Expression::Call { arguments, function, .. } => {
                if let Some(callee_name) = self.callee_name_from_call_function(function) {
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            && self.free_call_arg_expects_borrow(&callee_name, i)
                        {
                            return true;
                        }
                    }
                }
                self.expression_passes_param_to_borrowing_callee(function, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_passes_param_to_borrowing_callee(arg, param_name, func)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_param_to_borrowing_callee(left, param_name, func)
                    || self.expression_passes_param_to_borrowing_callee(right, param_name, func)
            }
            Expression::Unary { operand, .. } => {
                self.expression_passes_param_to_borrowing_callee(operand, param_name, func)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_passes_param_to_borrowing_callee(object, param_name, func)
            }
            Expression::Index { object, index, .. } => {
                self.expression_passes_param_to_borrowing_callee(object, param_name, func)
                    || self.expression_passes_param_to_borrowing_callee(index, param_name, func)
            }
            Expression::Block { statements, .. } => {
                self.param_passed_to_borrowing_callee(statements.as_slice(), param_name, func)
            }
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
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                            return true;
                        }
                    }
                }
                args.iter().any(|arg| {
                    self.expression_passes_param_to_borrowing_callee(arg, param_name, func)
                })
            }
            _ => false,
        }
    }

    /// WJ AST formals for sibling methods (authoritative before registry convergence).
    pub(in crate::codegen::rust) fn register_impl_ast_method_formals(
        &mut self,
        struct_name: &str,
        func: &FunctionDecl<'ast>,
    ) {
        let formals: Vec<Type> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .map(|p| p.type_.clone())
            .collect();
        self.struct_method_ast_formal_param_types
            .entry(struct_name.to_string())
            .or_default()
            .insert(func.name.clone(), formals);
        // Lookup facades (get-style): owned non-Copy custom formals read via field access.
        for param in &func.parameters {
            if param.name == "self" || self.is_type_copy(&param.type_) {
                continue;
            }
            if !matches!(&param.type_, Type::Custom(_)) {
                continue;
            }
            if matches!(
                &param.type_,
                Type::Reference(_) | Type::MutableReference(_)
            ) {
                continue;
            }
            if self.param_passes_to_wj_owned_sibling_call(
                func.body.as_slice(),
                &param.name,
                func,
            ) && self.param_only_used_as_call_argument(
                func.body.as_slice(),
                &param.name,
                func,
            ) {
                self.struct_has_owned_key_sibling_wrapper
                    .insert(struct_name.to_string());
            }
            if self.param_only_used_as_call_argument(func.body.as_slice(), &param.name, func) {
                continue;
            }
            if self.param_used_as_read_operand(func.body.as_slice(), &param.name)
                || Self::expression_mentions_field_access_of(
                    func.body.as_slice(),
                    &param.name,
                )
            {
                self.struct_has_owned_key_field_lookup
                    .insert(struct_name.to_string());
            }
        }
    }

    /// True when `param_name` appears in a field-access expression in `body`.
    fn expression_mentions_field_access_of(
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        body.iter().any(|stmt| {
            Self::statement_mentions_field_access_of(stmt, param_name)
        })
    }

    fn statement_mentions_field_access_of(stmt: &Statement<'ast>, param_name: &str) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return { value: Some(expr), .. }
            | Statement::Let { value: expr, .. }
            | Statement::Const { value: expr, .. } => {
                Self::expr_mentions_field_access_of(expr, param_name)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::expr_mentions_field_access_of(condition, param_name)
                    || Self::expression_mentions_field_access_of(then_block.as_slice(), param_name)
                    || else_block.as_ref().is_some_and(|b| {
                        Self::expression_mentions_field_access_of(b.as_slice(), param_name)
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::expr_mentions_field_access_of(condition, param_name)
                    || Self::expression_mentions_field_access_of(body.as_slice(), param_name)
            }
            Statement::For { iterable, body, .. } => {
                Self::expr_mentions_field_access_of(iterable, param_name)
                    || Self::expression_mentions_field_access_of(body.as_slice(), param_name)
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                Self::expression_mentions_field_access_of(body.as_slice(), param_name)
            }
            Statement::Match { value, arms, .. } => {
                Self::expr_mentions_field_access_of(value, param_name)
                    || arms.iter().any(|arm| {
                        Self::expr_mentions_field_access_of(&arm.body, param_name)
                    })
            }
            _ => false,
        }
    }

    fn expr_mentions_field_access_of(expr: &Expression<'ast>, param_name: &str) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == param_name)
                    || Self::expr_mentions_field_access_of(object, param_name)
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                Self::expr_mentions_field_access_of(object, param_name)
                    || arguments
                        .iter()
                        .any(|(_, a)| Self::expr_mentions_field_access_of(a, param_name))
            }
            Expression::Call { arguments, function, .. } => {
                Self::expr_mentions_field_access_of(function, param_name)
                    || arguments
                        .iter()
                        .any(|(_, a)| Self::expr_mentions_field_access_of(a, param_name))
            }
            Expression::Binary { left, right, .. } => {
                Self::expr_mentions_field_access_of(left, param_name)
                    || Self::expr_mentions_field_access_of(right, param_name)
            }
            Expression::Unary { operand, .. }
            | Expression::TryOp { expr: operand, .. }
            | Expression::Await { expr: operand, .. } => {
                Self::expr_mentions_field_access_of(operand, param_name)
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| Self::expr_mentions_field_access_of(e, param_name)),
            Expression::Block { statements, .. } => {
                Self::expression_mentions_field_access_of(statements.as_slice(), param_name)
            }
            _ => false,
        }
    }

    fn ast_sibling_method_arg_is_owned_non_copy_formal(
        &self,
        method: &str,
        arg_index: usize,
    ) -> bool {
        let Some(struct_name) = self.current_struct_name.as_ref() else {
            return false;
        };
        self.struct_method_ast_formal_param_types
            .get(struct_name)
            .and_then(|methods| methods.get(method))
            .and_then(|params| params.get(arg_index))
            .is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && !type_analysis::is_copy_type(t)
            })
    }

    /// True when every use of `param_name` is inside `let _ = (...)` discard bindings.
    /// True when the parameter only appears inside a **tuple** discard binding
    /// like `let _ = (key.bytes.len(), value)` — i.e., a multi-param "suppress
    /// unused" pattern. Keeps source-declared ownership.
    ///
    /// Returns false for simple value discards like `let _ = bytes.len()` —
    /// those represent genuine read-only uses inferred as borrows.
    ///
    /// Also returns false for text types (`string`/`String`/`str`) — borrowing
    /// to `&str` is always correct and more efficient for discarded string params.
    pub(in crate::codegen::rust) fn param_only_used_in_discarding_let_binding(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if body.is_empty() {
            return false;
        }
        if let Some(param) = func.parameters.iter().find(|p| p.name == param_name) {
            if crate::codegen::rust::types::is_windjammer_text_type(&param.type_) {
                return false;
            }
        }
        let mut found_in_tuple_discard = false;
        for stmt in body {
            match stmt {
                Statement::Let { pattern, value, .. } => {
                    if Self::expression_mentions_identifier(value, param_name) {
                        if Self::is_discarding_let_pattern(pattern)
                            && matches!(value, Expression::Tuple { .. })
                        {
                            found_in_tuple_discard = true;
                        } else {
                            return false;
                        }
                    }
                }
                _ => {
                    if Self::statement_mentions_identifier(stmt, param_name) {
                        return false;
                    }
                }
            }
        }
        found_in_tuple_discard
    }

    /// True when every mention of `param_name` is inside any discarding `let _ = …`
    /// (tuple or simple). Used to demote discarded WJ `string` formals to `&str`.
    pub(in crate::codegen::rust) fn param_only_used_in_simple_or_tuple_discard(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        if body.is_empty() {
            return false;
        }
        let mut found = false;
        for stmt in body {
            match stmt {
                Statement::Let { pattern, value, .. } => {
                    if Self::expression_mentions_identifier(value, param_name) {
                        if Self::is_discarding_let_pattern(pattern) {
                            found = true;
                        } else {
                            return false;
                        }
                    }
                }
                _ => {
                    if Self::statement_mentions_identifier(stmt, param_name) {
                        return false;
                    }
                }
            }
        }
        found
    }

    fn is_discarding_let_pattern(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard => true,
            Pattern::Identifier(name) if name == "_" => true,
            Pattern::Tuple(items) => items.iter().all(Self::is_discarding_let_pattern),
            _ => false,
        }
    }

    fn statement_mentions_identifier(stmt: &Statement, name: &str) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return { value: Some(expr), .. } => {
                Self::expression_mentions_identifier(expr, name)
            }
            Statement::Return { .. } => false,
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::expression_mentions_identifier(condition, name)
                    || then_block
                        .iter()
                        .any(|s| Self::statement_mentions_identifier(s, name))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| Self::statement_mentions_identifier(s, name))
                    })
            }
            Statement::While { condition, body, .. } => {
                Self::expression_mentions_identifier(condition, name)
                    || body
                        .iter()
                        .any(|s| Self::statement_mentions_identifier(s, name))
            }
            Statement::For { iterable, body, .. } => {
                Self::expression_mentions_identifier(iterable, name)
                    || body
                        .iter()
                        .any(|s| Self::statement_mentions_identifier(s, name))
            }
            Statement::Let { value, else_block, .. } => {
                Self::expression_mentions_identifier(value, name)
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| Self::statement_mentions_identifier(s, name))
                    })
            }
            Statement::Assignment { value, .. } => {
                Self::expression_mentions_identifier(value, name)
            }
            Statement::Match { value, arms, .. } => {
                Self::expression_mentions_identifier(value, name)
                    || arms.iter().any(|arm| {
                        Self::expression_mentions_identifier(&arm.body, name)
                    })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| Self::statement_mentions_identifier(s, name)),
            Statement::Defer { statement, .. } => {
                Self::statement_mentions_identifier(statement, name)
            }
            _ => false,
        }
    }

    fn expression_mentions_identifier(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::MethodCall { object, arguments, .. } => {
                Self::expression_mentions_identifier(object, name)
                    || arguments
                        .iter()
                        .any(|(_, arg)| Self::expression_mentions_identifier(arg, name))
            }
            Expression::Call { function, arguments, .. } => {
                Self::expression_mentions_identifier(function, name)
                    || arguments
                        .iter()
                        .any(|(_, arg)| Self::expression_mentions_identifier(arg, name))
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_mentions_identifier(left, name)
                    || Self::expression_mentions_identifier(right, name)
            }
            Expression::Unary { operand, .. } => {
                Self::expression_mentions_identifier(operand, name)
            }
            Expression::FieldAccess { object, .. } => {
                Self::expression_mentions_identifier(object, name)
            }
            Expression::Index { object, index, .. } => {
                Self::expression_mentions_identifier(object, name)
                    || Self::expression_mentions_identifier(index, name)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| Self::statement_mentions_identifier(s, name)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| Self::expression_mentions_identifier(elem, name)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, value)| Self::expression_mentions_identifier(value, name)),
            _ => false,
        }
    }

    /// True when `param_name` is moved into a returned/expressed struct or enum payload.
    ///
    /// Covers bare `{ field: param }` and nested constructors such as
    /// `Objective { kind: ObjectiveType::KillEnemies(enemy_type, count) }` so codegen
    /// does not demote stored text params to borrowed formals (`&String` + `.clone()`).
    ///
    /// A bare identifier return (`fn identity(data) { data }`) is **not** a payload
    /// store — that stays owned via `returned_parameters`, not this helper.
    /// True when `param_name` is moved into a struct field, enum variant, constructor,
    /// or other owned payload anywhere in the function body.
    pub(in crate::codegen::rust) fn param_stored_in_owned_payload(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        body.iter().any(|stmt| {
            self.statement_moves_param_into_owned_payload(stmt, param_name)
        })
    }

    fn statement_moves_param_into_owned_payload(
        &self,
        stmt: &Statement<'ast>,
        param_name: &str,
    ) -> bool {
        match stmt {
            Statement::Let { value, else_block, .. } => {
                self.expression_moves_param_into_owned_payload(value, param_name)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_stored_in_owned_payload(b.as_slice(), param_name)
                    })
            }
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_moves_param_into_owned_payload(expr, param_name),
            Statement::Assignment { value, .. } => {
                self.expression_moves_param_into_owned_payload(value, param_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.param_stored_in_owned_payload(then_block.as_slice(), param_name)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_stored_in_owned_payload(b.as_slice(), param_name)
                    })
            }
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::For { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.param_stored_in_owned_payload(body.as_slice(), param_name)
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                self.expression_moves_param_into_owned_payload(&arm.body, param_name)
            }),
            Statement::Defer { statement, .. } => {
                self.statement_moves_param_into_owned_payload(statement, param_name)
            }
            _ => false,
        }
    }

    /// Single-expression bodies that move a param into struct/enum/constructor payload.
    /// Prefer [`Self::param_stored_in_owned_payload`] for multi-statement functions.
    pub(in crate::codegen::rust) fn param_moves_via_struct_literal_init(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        self.param_stored_in_owned_payload(body, param_name)
    }

    fn call_argument_stores_owned_payload_in_registry(
        &self,
        function: &Expression<'ast>,
        arg_index: usize,
        arg_count: usize,
        registry: &crate::analyzer::SignatureRegistry,
    ) -> bool {
        crate::analyzer::Analyzer::call_argument_stores_owned_payload(
            function,
            arg_index,
            arg_count,
            registry,
        )
    }

    fn method_call_argument_stores_owned_payload_in_registry(
        &self,
        method: &str,
        receiver_type: Option<&str>,
        arg_index: usize,
        arg_count: usize,
        registry: &crate::analyzer::SignatureRegistry,
    ) -> bool {
        crate::analyzer::Analyzer::method_call_argument_stores_owned_payload(
            method,
            receiver_type,
            arg_index,
            arg_count,
            registry,
        )
    }

    fn call_argument_stores_owned_payload(
        &self,
        function: &Expression<'ast>,
        arg_index: usize,
        arg_count: usize,
    ) -> bool {
        if self.call_argument_stores_owned_payload_in_registry(
            function,
            arg_index,
            arg_count,
            &self.signature_registry,
        ) {
            return true;
        }
        self.global_signature_registry()
            .is_some_and(|global| {
                self.call_argument_stores_owned_payload_in_registry(
                    function,
                    arg_index,
                    arg_count,
                    global,
                )
            })
    }

    fn method_call_argument_stores_owned_payload(
        &self,
        method: &str,
        object: &Expression<'ast>,
        arg_index: usize,
        arg_count: usize,
    ) -> bool {
        let receiver_type = Self::receiver_type_name_for_payload_storage(object);
        if self.method_call_argument_stores_owned_payload_in_registry(
            method,
            receiver_type.as_deref(),
            arg_index,
            arg_count,
            &self.signature_registry,
        ) {
            return true;
        }
        self.global_signature_registry()
            .is_some_and(|global| {
                self.method_call_argument_stores_owned_payload_in_registry(
                    method,
                    receiver_type.as_deref(),
                    arg_index,
                    arg_count,
                    global,
                )
            })
    }

    fn receiver_type_name_for_payload_storage(object: &Expression<'ast>) -> Option<String> {
        match object {
            Expression::Identifier { name, .. }
                if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) =>
            {
                Some(name.clone())
            }
            Expression::FieldAccess { object, .. } => {
                Self::receiver_type_name_for_payload_storage(object)
            }
            _ => None,
        }
    }

    /// Identifier moved into a struct field, tuple/array element, or enum/Option/constructor.
    fn expression_moves_param_into_owned_payload(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == param_name,
            Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, value)| {
                self.expression_moves_param_into_owned_payload(value, param_name)
            }),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_moves_param_into_owned_payload(el, param_name)),
            Expression::Call {
                function,
                arguments,
                ..
            } => arguments.iter().enumerate().any(|(i, (_, arg))| {
                self.call_argument_stores_owned_payload(function, i, arguments.len())
                    && self.expression_moves_param_into_owned_payload(arg, param_name)
            }),
            Expression::MethodCall {
                method,
                arguments,
                object,
                ..
            } => arguments.iter().enumerate().any(|(i, (_, arg))| {
                self.method_call_argument_stores_owned_payload(
                    method,
                    object,
                    i,
                    arguments.len(),
                ) && self.expression_moves_param_into_owned_payload(arg, param_name)
            }),
            Expression::Unary { operand, .. } => {
                self.expression_moves_param_into_owned_payload(operand, param_name)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_moves_param_into_owned_payload(left, param_name)
                    || self.expression_moves_param_into_owned_payload(right, param_name)
            }
            Expression::MacroInvocation { args, .. } => args.iter().any(|arg| {
                self.expression_moves_param_into_owned_payload(arg, param_name)
            }),
            Expression::Block { statements, .. } => statements.iter().any(|s| match s {
                Statement::Expression { expr, .. }
                | Statement::Return {
                    value: Some(expr), ..
                } => self.expression_moves_param_into_owned_payload(expr, param_name),
                _ => false,
            }),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn param_only_used_in_same_name_struct_field_return(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
    ) -> bool {
        if body.len() != 1 {
            return false;
        }
        let expr = match body[0] {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => expr,
            _ => return false,
        };
        let Expression::StructLiteral { fields, .. } = expr else {
            return false;
        };
        fields.iter().any(|(field, value)| {
            field == param_name
                && matches!(value, Expression::Identifier { name, .. } if name == param_name)
        })
    }

    fn method_call_sibling_ast_expects_owned_arg(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if self.ast_sibling_method_arg_is_owned_non_copy_formal(method, arg_index) {
            return true;
        }
        let Some(rt) = (if let Expression::Identifier { name, .. } = object {
            if name == "self" && self.in_impl_block {
                self.current_struct_name.clone()
            } else {
                self.infer_local_binding_type_name(func.body.as_slice(), name)
                    .or_else(|| self.infer_type_name(object))
            }
        } else {
            self.mc_infer_method_receiver_type_name(object)
                .or_else(|| self.infer_type_name(object))
        }) else {
            return false;
        };
        let qualified = format!("{rt}::{method}");
        let registry_sig = self
            .signature_registry
            .get_signature(&qualified)
            .or_else(|| self.get_signature_with_global(&qualified));
        let mut sig = if let Some(ms) = self.lookup_method_signature(&rt, method) {
            let mut sig = ms.to_function_signature();
            if let Some(reg) = registry_sig {
                if reg.emitted_rust_ref_params.is_some() {
                    sig.emitted_rust_ref_params = reg.emitted_rust_ref_params.clone();
                    sig.formal_param_types = reg.formal_param_types.clone();
                    sig.forwarding_borrow_params = reg.forwarding_borrow_params.clone();
                }
            }
            sig
        } else if let Some(reg) = registry_sig {
            reg.clone()
        } else {
            return false;
        };
        let pidx = sig.arg_param_index(arg_index);
        // WJ source formals (`key: Key`) are authoritative for mixed-forwarder detection.
        // Converged registry `param_types` may already be `&Key` for borrow-only delegates
        // (has_key) — using those would falsely reject owned siblings (patch_put/hot_put).
        if sig
            .forwarding_borrow_params
            .as_ref()
            .and_then(|flags| flags.get(pidx))
            .copied()
            .unwrap_or(false)
        {
            return false;
        }
        if sig.emitted_rust_ref_params.as_ref().is_some_and(|flags| {
            flags.get(pidx).copied().unwrap_or(false)
        }) {
            return false;
        }
        sig.formal_param_types
            .get(pidx)
            .or_else(|| sig.formal_param_type(pidx))
            .is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && !type_analysis::is_copy_type(t)
            })
    }

    /// True when `param_name` is passed to a method on a receiver other than `self` / `self.field`.
    pub(in crate::codegen::rust) fn param_passed_to_non_self_receiver_method_arg(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_to_non_self_receiver_method_arg(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    /// True when `param_name` is passed to a method whose receiver is `self` or `self.field`.
    pub(in crate::codegen::rust) fn param_passed_to_self_or_field_receiver_method_arg(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_to_self_or_field_receiver_method_arg(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    /// True when `param_name` is passed to a method whose receiver is another formal
    /// parameter (`items.push(item)` — `items` is a sibling param).
    pub(in crate::codegen::rust) fn param_passed_to_other_param_receiver_method_arg(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_to_other_param_receiver_method_arg(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    fn statement_passes_to_other_param_receiver_method_arg(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(expr, param_name, func)
            }
            Statement::Return { .. } => false,
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    condition, param_name, func,
                ) || self.param_passed_to_other_param_receiver_method_arg(
                    then_block.as_slice(),
                    param_name,
                    func,
                ) || else_block.as_ref().is_some_and(|b| {
                    self.param_passed_to_other_param_receiver_method_arg(
                        b.as_slice(),
                        param_name,
                        func,
                    )
                })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    condition, param_name, func,
                ) || self.param_passed_to_other_param_receiver_method_arg(
                    body.as_slice(),
                    param_name,
                    func,
                )
            }
            Statement::For { body, iterable, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    iterable, param_name, func,
                ) || self.param_passed_to_other_param_receiver_method_arg(
                    body.as_slice(),
                    param_name,
                    func,
                )
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    value, param_name, func,
                ) || else_block.as_ref().is_some_and(|b| {
                    self.param_passed_to_other_param_receiver_method_arg(
                        b.as_slice(),
                        param_name,
                        func,
                    )
                })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self
                .param_passed_to_other_param_receiver_method_arg(
                    body.as_slice(),
                    param_name,
                    func,
                ),
            Statement::Match { value, arms, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    value, param_name, func,
                ) || arms.iter().any(|arm| {
                    self.expression_passes_to_other_param_receiver_method_arg(
                        &arm.body, param_name, func,
                    )
                })
            }
            Statement::Assignment { value, .. } => self
                .expression_passes_to_other_param_receiver_method_arg(value, param_name, func),
            Statement::Defer { statement, .. } => self
                .statement_passes_to_other_param_receiver_method_arg(
                    statement, param_name, func,
                ),
            _ => false,
        }
    }

    fn expression_passes_to_other_param_receiver_method_arg(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: recv, .. } = &**object {
                    if recv != param_name
                        && func.parameters.iter().any(|p| p.name == *recv)
                        && arguments.iter().any(|(_, arg)| {
                            matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        })
                    {
                        return true;
                    }
                }
                self.expression_passes_to_other_param_receiver_method_arg(
                    object, param_name, func,
                ) || arguments.iter().any(|(_, arg)| {
                    self.expression_passes_to_other_param_receiver_method_arg(
                        arg, param_name, func,
                    )
                })
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    function, param_name, func,
                ) || arguments.iter().any(|(_, arg)| {
                    self.expression_passes_to_other_param_receiver_method_arg(
                        arg, param_name, func,
                    )
                })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(left, param_name, func)
                    || self.expression_passes_to_other_param_receiver_method_arg(
                        right, param_name, func,
                    )
            }
            Expression::Unary { operand, .. } => self
                .expression_passes_to_other_param_receiver_method_arg(operand, param_name, func),
            Expression::FieldAccess { object, .. } => self
                .expression_passes_to_other_param_receiver_method_arg(object, param_name, func),
            Expression::Index { object, index, .. } => {
                self.expression_passes_to_other_param_receiver_method_arg(
                    object, param_name, func,
                ) || self.expression_passes_to_other_param_receiver_method_arg(
                    index, param_name, func,
                )
            }
            Expression::Block { statements, .. } => self
                .param_passed_to_other_param_receiver_method_arg(
                    statements.as_slice(),
                    param_name,
                    func,
                ),
            _ => false,
        }
    }

    fn statement_passes_to_self_or_field_receiver_method_arg(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(expr, param_name, func)
            }
            Statement::Return { .. } => false,
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    condition, param_name, func,
                ) || self.param_passed_to_self_or_field_receiver_method_arg(
                    then_block.as_slice(),
                    param_name,
                    func,
                ) || else_block.as_ref().is_some_and(|b| {
                    self.param_passed_to_self_or_field_receiver_method_arg(
                        b.as_slice(),
                        param_name,
                        func,
                    )
                })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    condition, param_name, func,
                ) || self.param_passed_to_self_or_field_receiver_method_arg(
                    body.as_slice(),
                    param_name,
                    func,
                )
            }
            Statement::For { body, iterable, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    iterable, param_name, func,
                ) || self.param_passed_to_self_or_field_receiver_method_arg(
                    body.as_slice(),
                    param_name,
                    func,
                )
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    value, param_name, func,
                ) || else_block.as_ref().is_some_and(|b| {
                    self.param_passed_to_self_or_field_receiver_method_arg(
                        b.as_slice(),
                        param_name,
                        func,
                    )
                })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self
                .param_passed_to_self_or_field_receiver_method_arg(
                    body.as_slice(),
                    param_name,
                    func,
                ),
            Statement::Match { value, arms, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    value, param_name, func,
                ) || arms.iter().any(|arm| {
                    self.expression_passes_to_self_or_field_receiver_method_arg(
                        &arm.body, param_name, func,
                    )
                })
            }
            Statement::Assignment { value, .. } => self
                .expression_passes_to_self_or_field_receiver_method_arg(value, param_name, func),
            Statement::Defer { statement, .. } => self
                .statement_passes_to_self_or_field_receiver_method_arg(
                    statement, param_name, func,
                ),
            _ => false,
        }
    }

    fn expression_passes_to_self_or_field_receiver_method_arg(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                if crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(object)
                {
                    for (_, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        {
                            return true;
                        }
                    }
                }
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    object, param_name, func,
                ) || arguments.iter().any(|(_, arg)| {
                    self.expression_passes_to_self_or_field_receiver_method_arg(
                        arg, param_name, func,
                    )
                })
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    function, param_name, func,
                ) || arguments.iter().any(|(_, arg)| {
                    self.expression_passes_to_self_or_field_receiver_method_arg(
                        arg, param_name, func,
                    )
                })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(left, param_name, func)
                    || self.expression_passes_to_self_or_field_receiver_method_arg(
                        right, param_name, func,
                    )
            }
            Expression::Unary { operand, .. } => self
                .expression_passes_to_self_or_field_receiver_method_arg(operand, param_name, func),
            Expression::FieldAccess { object, .. } => self
                .expression_passes_to_self_or_field_receiver_method_arg(object, param_name, func),
            Expression::Index { object, index, .. } => {
                self.expression_passes_to_self_or_field_receiver_method_arg(
                    object, param_name, func,
                ) || self.expression_passes_to_self_or_field_receiver_method_arg(
                    index, param_name, func,
                )
            }
            Expression::Block { statements, .. } => self
                .param_passed_to_self_or_field_receiver_method_arg(
                    statements.as_slice(),
                    param_name,
                    func,
                ),
            _ => false,
        }
    }

    fn statement_passes_to_non_self_receiver_method_arg(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(expr, param_name, func)
            }
            Statement::Return { .. } => false,
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_to_non_self_receiver_method_arg(condition, param_name, func)
                    || self.param_passed_to_non_self_receiver_method_arg(
                        then_block.as_slice(),
                        param_name,
                        func,
                    )
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_to_non_self_receiver_method_arg(
                            b.as_slice(),
                            param_name,
                            func,
                        )
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(condition, param_name, func)
                    || self.param_passed_to_non_self_receiver_method_arg(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::For { body, iterable, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(iterable, param_name, func)
                    || self.param_passed_to_non_self_receiver_method_arg(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(value, param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_to_non_self_receiver_method_arg(
                            b.as_slice(),
                            param_name,
                            func,
                        )
                    })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.param_passed_to_non_self_receiver_method_arg(
                body.as_slice(),
                param_name,
                func,
            ),
            Statement::Match { value, arms, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(value, param_name, func)
                    || arms.iter().any(|arm| {
                        self.expression_passes_to_non_self_receiver_method_arg(
                            &arm.body,
                            param_name,
                            func,
                        )
                    })
            }
            Statement::Assignment { value, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(value, param_name, func)
            }
            Statement::Defer { statement, .. } => {
                self.statement_passes_to_non_self_receiver_method_arg(statement, param_name, func)
            }
            _ => false,
        }
    }

    fn expression_passes_to_non_self_receiver_method_arg(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                if !crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                    object,
                ) {
                    for (_, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                            return true;
                        }
                    }
                }
                self.expression_passes_to_non_self_receiver_method_arg(object, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_passes_to_non_self_receiver_method_arg(
                            arg, param_name, func,
                        )
                    })
            }
            Expression::Call { function, arguments, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(function, param_name, func)
                    || arguments.iter().any(|(_, arg)| {
                        self.expression_passes_to_non_self_receiver_method_arg(
                            arg, param_name, func,
                        )
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(left, param_name, func)
                    || self.expression_passes_to_non_self_receiver_method_arg(
                        right, param_name, func,
                    )
            }
            Expression::Unary { operand, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(operand, param_name, func)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(object, param_name, func)
            }
            Expression::Index { object, index, .. } => {
                self.expression_passes_to_non_self_receiver_method_arg(object, param_name, func)
                    || self.expression_passes_to_non_self_receiver_method_arg(
                        index, param_name, func,
                    )
            }
            Expression::Block { statements, .. } => {
                self.param_passed_to_non_self_receiver_method_arg(
                    statements.as_slice(),
                    param_name,
                    func,
                )
            }
            _ => false,
        }
    }

    /// True when `param_name` is passed to `self.some_method(...)` and the callee's WJ
    /// formal for that argument is an owned non-Copy type (patch_put/hot_put).
    pub(in crate::codegen::rust) fn param_passed_to_owned_self_method_arg(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_to_owned_self_method_arg(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    fn statement_passes_to_owned_self_method_arg(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_passes_to_owned_self_method_arg(expr, param_name, func)
            }
            Statement::Return { .. } => false,
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_to_owned_self_method_arg(condition, param_name, func)
                    || self.param_passed_to_owned_self_method_arg(
                        then_block.as_slice(),
                        param_name,
                        func,
                    )
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_to_owned_self_method_arg(
                            b.as_slice(),
                            param_name,
                            func,
                        )
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_to_owned_self_method_arg(condition, param_name, func)
                    || self.param_passed_to_owned_self_method_arg(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::For { body, iterable, .. } => {
                self.expression_passes_to_owned_self_method_arg(iterable, param_name, func)
                    || self.param_passed_to_owned_self_method_arg(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_to_owned_self_method_arg(value, param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passed_to_owned_self_method_arg(
                            b.as_slice(),
                            param_name,
                            func,
                        )
                    })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.param_passed_to_owned_self_method_arg(
                body.as_slice(),
                param_name,
                func,
            ),
            Statement::Match { value, arms, .. } => {
                self.expression_passes_to_owned_self_method_arg(value, param_name, func)
                    || arms.iter().any(|arm| {
                        self.expression_passes_to_owned_self_method_arg(
                            &arm.body,
                            param_name,
                            func,
                        )
                    })
            }
            _ => false,
        }
    }

    fn expression_passes_to_owned_self_method_arg(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if !matches!(object, Expression::Identifier { name, .. } if name == "self") {
                    return false;
                }
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        && self.method_call_sibling_ast_expects_owned_arg(object, method, i, func)
                    {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// True when the body forwards `param_name` to a sibling method whose WJ formal is
    /// owned (patch_put/hot_put), excluding converged borrow-only callees (has_key).
    pub(in crate::codegen::rust) fn param_passes_to_wj_owned_sibling_call(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        for stmt in body {
            if self.statement_passes_to_wj_owned_sibling_call(stmt, param_name, func) {
                return true;
            }
        }
        false
    }

    fn statement_passes_to_wj_owned_sibling_call(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_passes_to_wj_owned_sibling_call(expr, param_name, func)
            }
            Statement::Return { .. } => false,
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_passes_to_wj_owned_sibling_call(condition, param_name, func)
                    || self.param_passes_to_wj_owned_sibling_call(
                        then_block.as_slice(),
                        param_name,
                        func,
                    )
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passes_to_wj_owned_sibling_call(
                            b.as_slice(),
                            param_name,
                            func,
                        )
                    })
            }
            Statement::While { body, condition, .. } => {
                self.expression_passes_to_wj_owned_sibling_call(condition, param_name, func)
                    || self.param_passes_to_wj_owned_sibling_call(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::For { body, iterable, .. } => {
                self.expression_passes_to_wj_owned_sibling_call(iterable, param_name, func)
                    || self.param_passes_to_wj_owned_sibling_call(
                        body.as_slice(),
                        param_name,
                        func,
                    )
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_to_wj_owned_sibling_call(value, param_name, func)
                    || else_block.as_ref().is_some_and(|b| {
                        self.param_passes_to_wj_owned_sibling_call(
                            b.as_slice(),
                            param_name,
                            func,
                        )
                    })
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.param_passes_to_wj_owned_sibling_call(
                body.as_slice(),
                param_name,
                func,
            ),
            Statement::Match { value, arms, .. } => {
                self.expression_passes_to_wj_owned_sibling_call(value, param_name, func)
                    || arms.iter().any(|arm| {
                        self.expression_passes_to_wj_owned_sibling_call(
                            &arm.body,
                            param_name,
                            func,
                        )
                    })
            }
            _ => false,
        }
    }

    fn expression_passes_to_wj_owned_sibling_call(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let is_self_sibling = matches!(
                    &**object,
                    Expression::Identifier { name, .. } if name == "self"
                );
                if is_self_sibling {
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                            && self.method_call_sibling_ast_expects_owned_arg(
                                object, method, i, func,
                            )
                        {
                            return true;
                        }
                    }
                }
                false
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_to_wj_owned_sibling_call(left, param_name, func)
                    || self.expression_passes_to_wj_owned_sibling_call(right, param_name, func)
            }
            Expression::Unary { operand, .. } => {
                self.expression_passes_to_wj_owned_sibling_call(operand, param_name, func)
            }
            Expression::Block { statements, .. } => {
                self.param_passes_to_wj_owned_sibling_call(statements.as_slice(), param_name, func)
            }
            _ => false,
        }
    }

    fn method_call_arg_expects_borrow(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if let Some(sig) = self.method_call_signature_for_arg(object, method, arg_index, func) {
            let pidx = sig.arg_param_index(arg_index);
            // WJ AST owned non-copy formals are not borrowing callees for delegation
            // decisions unless codegen already recorded an emitted `&T` formal.
            if sig.formal_param_type(pidx).is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && !type_analysis::is_copy_type(t)
            }) {
                return false;
            }
            if sig
                .forwarding_borrow_params
                .as_ref()
                .and_then(|flags| flags.get(pidx))
                .copied()
                .unwrap_or(false)
            {
                return true;
            }
            if self.signature_param_expects_borrow(&sig, arg_index) {
                return true;
            }
            return false;
        }
        self.method_has_forwarding_borrow_param(method, arg_index)
    }

    /// Fallback when receiver type inference fails (e.g. `latest.has_key(key)` after vec index).
    fn method_has_forwarding_borrow_param(&self, method: &str, arg_index: usize) -> bool {
        self.method_signatures_by_type.values().any(|methods| {
            methods.get(method).is_some_and(|ms| {
                ms.forwarding_borrow_params
                    .get(arg_index)
                    .copied()
                    .unwrap_or(false)
            })
        })
    }

    /// Multi-method store/lookup facades keep owned non-Copy custom formals (WDB-046).
    ///
    /// Signature-driven: if this impl declares the same owned non-Copy custom type on
    /// ≥2 methods' formals, callers borrow at the outer edge and callees keep owned.
    /// No hardcoded method-name lists.
    pub(in crate::codegen::rust) fn struct_is_owned_engine_key_facade(
        &self,
        struct_name: &str,
        param: &crate::parser::Parameter,
    ) -> bool {
        if param.name == "self" || self.is_type_copy(&param.type_) {
            return false;
        }
        let Type::Custom(type_name) = &param.type_ else {
            return false;
        };
        let Some(methods) = self.struct_method_ast_formal_param_types.get(struct_name) else {
            // Fallback while AST formals are not yet registered for this impl.
            let method_names = &self.current_impl_methods;
            return method_names.len() >= 2
                && method_names.iter().any(|m| {
                    self.signature_registry
                        .get_signature(&format!("{struct_name}::{m}"))
                        .or_else(|| self.signature_registry.lookup_method(m))
                        .is_some_and(|sig| {
                            sig.formal_param_types.iter().any(|t| {
                                matches!(t, Type::Custom(n) if n == type_name)
                                    && !self.is_type_copy(t)
                            }) || sig.param_types.iter().any(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                matches!(bare, Type::Custom(n) if n == type_name)
                            })
                        })
                });
        };
        let methods_with_owned_same_type = methods
            .values()
            .filter(|formals| {
                formals.iter().any(|t| {
                    matches!(t, Type::Custom(n) if n == type_name)
                        && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && !self.is_type_copy(t)
                })
            })
            .count();
        methods_with_owned_same_type >= 2
            && self.struct_has_owned_key_field_lookup.contains(struct_name)
            && (self
                .struct_has_owned_key_sibling_wrapper
                .contains(struct_name)
                || methods_with_owned_same_type >= 3)
    }

    /// Facade keeps owned formals except non-self owned-forward delegates (`latest.has_key(key)`).
    pub(in crate::codegen::rust) fn param_keeps_owned_engine_key_facade(
        &self,
        struct_name: &str,
        param: &crate::parser::Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        self.struct_is_owned_engine_key_facade(struct_name, param)
            && !self.param_should_emit_borrowed_delegation_formal(param, func)
    }

    /// True when `param_name` appears in an `if` condition expression in `func`.
    pub(in crate::codegen::rust) fn body_forwards_param_in_if_condition(
        &self,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        func.body.iter().any(|stmt| {
            matches!(
                stmt,
                Statement::If { condition, .. }
                    if self.expr_mentions_param_name(param_name, condition)
            )
        })
    }

    /// True when a non-`self` param should emit `&T`/`&str` because the body only forwards to borrowing callees.
    ///
    /// Also true for single-arg helpers that only forward to an **owned** method on a non-`self`
    /// receiver (`latest.has_key(key)`): emit `&Key` and clone at the owned call site so outer
    /// forward-ref callers can borrow (`key_in_latest_base(&key)`). Self-sibling owned wrappers
    /// (`has_key` → `self.get`) stay owned via `param_passes_to_wj_owned_sibling_call`.
    pub(in crate::codegen::rust) fn param_should_emit_borrowed_delegation_formal(
        &self,
        param: &crate::parser::Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        let non_self_facade = self.param_is_non_self_forward_facade_borrow_candidate(param, func);
        let only_as_call_arg =
            self.param_only_used_as_call_argument(func.body.as_slice(), &param.name, func);
        let to_owned_sibling =
            self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func);
        let to_owned_method = self.param_passed_to_owned_non_copy_method_arg(
            func.body.as_slice(),
            &param.name,
            func,
        );
        // Multi-param helpers that only forward into owned store methods (`apply_patch_put`
        // → `apply_put` / `put_value`): emit `&T` + clone so outer forward-ref callers can
        // borrow (`put_value` → `apply_patch_put(&key, value)`). Single-param mixed
        // forwards (`apply_patch_delete`) keep owned (count < 2).
        // Note: do not require `non_self_facade` here — that helper gates on
        // `!to_owned_sibling`, which is true for store forwards and would never fire.
        let multiparam_store_forward_borrow = only_as_call_arg
            && to_owned_method
            && self.count_non_self_params(func) >= 2
            && func.body.len() > 1
            && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
            && !self.is_type_copy(&param.type_);
        param.name != "self"
            // Registry-aware Copy (Vec3, etc.) — not the primitive-only `type_analysis` helper.
            && !self.is_type_copy(&param.type_)
            && !matches!(
                &param.type_,
                Type::Reference(_) | Type::MutableReference(_)
            )
            && (!self.param_passed_from_multiple_statements(func.body.as_slice(), &param.name, func)
                || non_self_facade)
            && !self.param_used_in_if_with_condition_and_branches(
                func.body.as_slice(),
                &param.name,
            )
            && !self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
            && (!self.current_fn_mixed_forwarder_params.contains(&param.name) || non_self_facade)
            && (!to_owned_sibling || multiparam_store_forward_borrow)
            && !self.param_stored_in_owned_payload(func.body.as_slice(), &param.name)
            && !self.param_moves_via_struct_literal_init(func.body.as_slice(), &param.name)
            && {
                let single_stmt_multi_param_forward = only_as_call_arg
                    && self.count_non_self_params(func) >= 2
                    && func.body.len() == 1
                    && !self.func_is_pure_forwarding_delegate(func)
                    && !self.param_passed_from_multiple_statements(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    );
                // Exclusive non-self owned forward to a local receiver
                // (`latest.has_key(key)` for non-text Key): emit `&T` + clone so
                // outer forward-ref callers can borrow. Text params (`string`) and
                // forwards onto sibling params (`items.push(item)`) keep owned.
                let non_self_owned_forward = only_as_call_arg
                    && to_owned_method
                    && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                    && self.param_passed_to_non_self_receiver_method_arg(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_passed_to_self_or_field_receiver_method_arg(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_passed_to_other_param_receiver_method_arg(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    );
                // Single-stmt `merge(remote)` is both facade and multi-param forward; do not
                // borrow when the method consumes an owned non-copy arg. Multi-stmt helpers
                // like `put_value` keep ungated facade borrowing.
                // Mixed owned+borrowed callees (`apply_patch_delete` → owned apply_delete +
                // borrowed HotPart::delete_key) keep owned — do not demote solely because one
                // callee borrows.
                (non_self_facade && !(single_stmt_multi_param_forward && to_owned_method))
                    || (single_stmt_multi_param_forward && !to_owned_method)
                    || non_self_owned_forward
                    || multiparam_store_forward_borrow
                    || (self.param_passed_to_borrowing_callee(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) && !to_owned_method
                        && !self.param_passed_to_self_or_field_receiver_method_arg(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        ))
            }
    }

    /// True when `param_name` is passed to a method argument that emits owned non-copy
    /// (e.g. `local.merge(remote)` with `merge(other: LwwRegister)`).
    fn param_passed_to_owned_non_copy_method_arg(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        body.iter().any(|stmt| {
            self.statement_passes_param_to_owned_non_copy_method_arg(stmt, param_name, func)
        })
    }

    fn statement_passes_param_to_owned_non_copy_method_arg(
        &self,
        stmt: &Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_passes_param_to_owned_non_copy_method_arg(expr, param_name, func)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_passes_param_to_owned_non_copy_method_arg(
                    condition, param_name, func,
                ) || then_block.iter().any(|s| {
                    self.statement_passes_param_to_owned_non_copy_method_arg(s, param_name, func)
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.statement_passes_param_to_owned_non_copy_method_arg(
                            s, param_name, func,
                        )
                    })
                })
            }
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::For { body, .. } => body.iter().any(|s| {
                self.statement_passes_param_to_owned_non_copy_method_arg(s, param_name, func)
            }),
            _ => false,
        }
    }

    fn expression_passes_param_to_owned_non_copy_method_arg(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        && self.method_call_arg_formal_is_owned_non_copy(object, method, i, func)
                    {
                        return true;
                    }
                }
                self.expression_passes_param_to_owned_non_copy_method_arg(
                    object, param_name, func,
                ) || arguments.iter().any(|(_, a)| {
                    self.expression_passes_param_to_owned_non_copy_method_arg(a, param_name, func)
                })
            }
            Expression::Call { arguments, function, .. } => {
                arguments.iter().any(|(_, a)| {
                    self.expression_passes_param_to_owned_non_copy_method_arg(a, param_name, func)
                }) || self.expression_passes_param_to_owned_non_copy_method_arg(
                    function, param_name, func,
                )
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_param_to_owned_non_copy_method_arg(left, param_name, func)
                    || self.expression_passes_param_to_owned_non_copy_method_arg(
                        right, param_name, func,
                    )
            }
            Expression::FieldAccess { object, .. }
            | Expression::Unary { operand: object, .. } => self
                .expression_passes_param_to_owned_non_copy_method_arg(object, param_name, func),
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.statement_passes_param_to_owned_non_copy_method_arg(s, param_name, func)
            }),
            _ => false,
        }
    }

    /// True when `param_name` is forwarded as a call argument from more than one statement.
    pub(in crate::codegen::rust) fn param_passed_from_multiple_statements(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        let mut sites = 0usize;
        for stmt in body {
            if self.statement_passes_param_as_call_argument(stmt, param_name, func) {
                sites += 1;
                if sites > 1 {
                    return true;
                }
            }
        }
        false
    }

    /// Single-expression body that forwards all params to one callee (e.g. `self.engine.put(k, v)`).
    pub(in crate::codegen::rust) fn func_is_pure_forwarding_delegate(&self, func: &FunctionDecl<'ast>) -> bool {
        if func.body.len() != 1 {
            return false;
        }
        let stmt = func.body[0];
        let expr = match stmt {
            Statement::Expression { expr, .. } => expr,
            Statement::Return { value: Some(expr), .. } => expr,
            _ => return false,
        };
        self.expression_is_pure_forwarding_delegate(expr, func)
    }

    /// Re-derive pure-forwarding from the active function body (prepare flag can stale).
    pub(in crate::codegen::rust) fn refresh_pure_forwarding_delegate_flag(&mut self) {
        // Nested blocks (if arms, loops) temporarily swap `current_function_body` to a
        // single forwarding call — do not treat that as the whole function (put_value if-arm).
        if self.current_function_body != self.full_function_body_snapshot {
            return;
        }
        if self.current_function_body.len() != 1 {
            return;
        }
        let stmt = self.current_function_body[0];
        let expr = match stmt {
            Statement::Expression { expr, .. } => expr,
            Statement::Return { value: Some(expr), .. } => expr,
            _ => return,
        };
        let pseudo = FunctionDecl {
            name: self
                .current_function_name
                .clone()
                .unwrap_or_else(|| "anon".to_string()),
            is_pub: false,
            is_extern: false,
            type_params: Vec::new(),
            where_clause: Vec::new(),
            decorators: Vec::new(),
            is_async: false,
            parameters: self.current_function_params.clone(),
            return_type: self.current_function_return_type.clone(),
            return_decorators: Vec::new(),
            body: self.current_function_body.clone(),
            parent_type: self.current_struct_name.clone(),
            impl_trait: None,
            doc_comment: None,
        };
        self.current_func_is_pure_forwarding_delegate =
            self.expression_is_pure_forwarding_delegate(expr, &pseudo);
    }

    fn expression_is_pure_forwarding_delegate(
        &self,
        expr: &'ast Expression<'ast>,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        let arguments = match expr {
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                let receiver_is_self = matches!(object, Expression::Identifier { name, .. } if name == "self")
                    || matches!(
                        object,
                        Expression::FieldAccess {
                            object: obj,
                            ..
                        } if matches!(obj, Expression::Identifier { name, .. } if name == "self")
                    );
                if !receiver_is_self {
                    return false;
                }
                arguments.as_slice()
            }
            Expression::Call { arguments, .. } => arguments.as_slice(),
            _ => return false,
        };
        let non_self: Vec<_> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .map(|p| p.name.as_str())
            .collect();
        if non_self.is_empty() {
            return !arguments.is_empty()
                && arguments.iter().all(|(_, arg)| {
                    matches!(
                        arg,
                        Expression::FieldAccess { object, .. }
                            if crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                                object,
                            )
                    ) || matches!(arg, Expression::Identifier { name, .. } if name == "self")
                });
        }
        let forwarded: Vec<_> = arguments
            .iter()
            .filter_map(|(_, arg)| match arg {
                Expression::Identifier { name, .. } if name != "self" => Some(name.as_str()),
                Expression::FieldAccess { object, .. }
                    if crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                        object,
                    ) =>
                {
                    None
                }
                _ => None,
            })
            .collect();
        forwarded.len() == non_self.len()
            && non_self.iter().all(|n| forwarded.contains(n))
    }

    fn count_non_self_params(&self, func: &FunctionDecl<'ast>) -> usize {
        func.parameters
            .iter()
            .filter(|p| p.name != "self")
            .count()
    }

    /// Single-parameter helper that only forwards its arg to callees (e.g. `apply_patch_delete`).
    pub(in crate::codegen::rust) fn param_is_single_arg_call_only_delegate(
        &self,
        param: &crate::parser::Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        param.name != "self"
            && self.count_non_self_params(func) == 1
            && self.param_only_used_as_call_argument(func.body.as_slice(), &param.name, func)
    }

    pub(in crate::codegen::rust) fn param_type_is_vec_container(ty: &Type) -> bool {
        match ty {
            Type::Vec(_) => true,
            Type::Parameterized(name, _) if name == "Vec" => true,
            _ => false,
        }
    }

    /// Rust type for a borrowed formal (`&str` when Phase 2 applies, else `&T`).
    pub(in crate::codegen::rust) fn borrowed_formal_rust_type_for_param(
        &self,
        param: &crate::parser::Parameter,
        func: &FunctionDecl<'ast>,
        param_idx: usize,
    ) -> String {
        let is_string = matches!(param.type_, Type::String)
            || matches!(param.type_, Type::Custom(ref name) if name == "string");
        if is_string {
            let registry_str_ref = self
                .get_signature_with_global(&func.name)
                .and_then(|sig| sig.param_types.get(param_idx))
                .is_some_and(|ty| {
                    matches!(
                        ty,
                        Type::Reference(inner)
                            if matches!(&**inner, Type::Custom(n) if n == "str")
                    )
                });
            if self.str_ref_optimized_params.contains(&param.name) || registry_str_ref {
                if self.param_only_forwarded_to_qualified_collection_key_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) && func.parent_type.is_none()
                {
                    return format!("&{}", self.type_to_rust(&param.type_));
                }
                return "&str".to_string();
            }
            return "&String".to_string();
        }
        format!("&{}", self.type_to_rust(&param.type_))
    }

    /// True when the body only forwards `param_name` to callees that emit owned non-Copy formals.
    pub(in crate::codegen::rust) fn param_only_forwards_to_emitted_owned_callees(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if !self.param_only_used_as_call_argument(body, param_name, func) {
            return false;
        }
        let mut saw_site = false;
        let mut all_emitted_owned = true;
        let mut visit = |sig: &crate::analyzer::FunctionSignature, arg_index: usize| {
            saw_site = true;
            let pidx = sig.arg_param_index(arg_index);
            let forwarding = sig
                .forwarding_borrow_params
                .as_ref()
                .and_then(|flags| flags.get(pidx))
                .copied()
                .unwrap_or(false);
            let callee_keeps_wj_owned = sig.formal_param_type(pidx).is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && !type_analysis::is_copy_type(t)
            }) && matches!(
                sig.param_ownership.get(pidx),
                Some(crate::analyzer::OwnershipMode::Owned)
            );
            let emitted_owned = callee_keeps_wj_owned
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                || sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(pidx))
                    .copied()
                    == Some(false);
            if forwarding || !emitted_owned {
                all_emitted_owned = false;
            }
        };
        self.for_each_param_call_argument_site(body, param_name, func, &mut visit);
        saw_site && all_emitted_owned
    }

    fn for_each_param_call_argument_site<F>(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visit: &mut F,
    ) where
        F: FnMut(&crate::analyzer::FunctionSignature, usize),
    {
        for stmt in body {
            self.statement_for_each_param_call_argument_site(stmt, param_name, func, visit);
        }
    }

    fn statement_for_each_param_call_argument_site<F>(
        &self,
        stmt: &'ast Statement<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visit: &mut F,
    ) where
        F: FnMut(&crate::analyzer::FunctionSignature, usize),
    {
        match stmt {
            Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => {
                self.expression_for_each_param_call_argument_site(expr, param_name, func, visit);
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_for_each_param_call_argument_site(value, param_name, func, visit);
                if let Some(b) = else_block {
                    self.for_each_param_call_argument_site(b.as_slice(), param_name, func, visit);
                }
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                self.expression_for_each_param_call_argument_site(condition, param_name, func, visit);
                self.for_each_param_call_argument_site(then_block.as_slice(), param_name, func, visit);
                if let Some(b) = else_block {
                    self.for_each_param_call_argument_site(b.as_slice(), param_name, func, visit);
                }
            }
            Statement::While { body, condition, .. } => {
                self.expression_for_each_param_call_argument_site(condition, param_name, func, visit);
                self.for_each_param_call_argument_site(body.as_slice(), param_name, func, visit);
            }
            Statement::For { body, iterable, .. } => {
                self.expression_for_each_param_call_argument_site(iterable, param_name, func, visit);
                self.for_each_param_call_argument_site(body.as_slice(), param_name, func, visit);
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                self.for_each_param_call_argument_site(body.as_slice(), param_name, func, visit);
            }
            Statement::Match { value, arms, .. } => {
                self.expression_for_each_param_call_argument_site(value, param_name, func, visit);
                for arm in arms {
                    self.expression_for_each_param_call_argument_site(
                        &arm.body,
                        param_name,
                        func,
                        visit,
                    );
                }
            }
            _ => {}
        }
    }

    fn expression_for_each_param_call_argument_site<F>(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
        visit: &mut F,
    ) where
        F: FnMut(&crate::analyzer::FunctionSignature, usize),
    {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                        if let Some(sig) =
                            self.method_call_signature_for_arg(object, method, i, func)
                        {
                            visit(&sig, i);
                        }
                    }
                }
                self.expression_for_each_param_call_argument_site(object, param_name, func, visit);
                for (_, arg) in arguments {
                    self.expression_for_each_param_call_argument_site(arg, param_name, func, visit);
                }
            }
            Expression::Call { function, arguments, .. } => {
                if let Some(callee_name) = self.callee_name_from_call_function(function) {
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                            let simple = callee_name.rsplit("::").next().unwrap_or(&callee_name);
                            let sig = self
                                .signature_registry
                                .get_signature(&callee_name)
                                .or_else(|| self.signature_registry.lookup_method(&callee_name))
                                .or_else(|| {
                                    self.signature_registry.find_signature_ending_with(simple)
                                })
                                .or_else(|| {
                                    self.global_signature_registry.as_ref().and_then(|g| {
                                        g.get_signature(&callee_name)
                                            .or_else(|| g.lookup_method(&callee_name))
                                            .or_else(|| g.find_signature_ending_with(simple))
                                    })
                                });
                            if let Some(sig) = sig {
                                visit(sig, i);
                            }
                        }
                    }
                }
                self.expression_for_each_param_call_argument_site(function, param_name, func, visit);
                for (_, arg) in arguments {
                    self.expression_for_each_param_call_argument_site(arg, param_name, func, visit);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.expression_for_each_param_call_argument_site(left, param_name, func, visit);
                self.expression_for_each_param_call_argument_site(right, param_name, func, visit);
            }
            Expression::Unary { operand, .. } => {
                self.expression_for_each_param_call_argument_site(operand, param_name, func, visit);
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_for_each_param_call_argument_site(object, param_name, func, visit);
            }
            Expression::Index { object, index, .. } => {
                self.expression_for_each_param_call_argument_site(object, param_name, func, visit);
                self.expression_for_each_param_call_argument_site(index, param_name, func, visit);
            }
            Expression::Block { statements, .. } => {
                self.for_each_param_call_argument_site(statements.as_slice(), param_name, func, visit);
            }
            _ => {}
        }
    }

    fn free_call_arg_expects_borrow(&self, callee_name: &str, arg_index: usize) -> bool {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        if let Some(g) = self.global_signature_registry.as_ref() {
            if let Some(sig) = g
                .get_signature(callee_name)
                .or_else(|| g.lookup_method(callee_name))
                .or_else(|| g.find_signature_ending_with(simple))
            {
                return self.signature_param_expects_borrow(sig, arg_index);
            }
        }
        if let Some(sig) = self
            .signature_registry
            .get_signature(callee_name)
            .or_else(|| self.signature_registry.lookup_method(callee_name))
            .or_else(|| self.signature_registry.find_signature_ending_with(simple))
        {
            return self.signature_param_expects_borrow(sig, arg_index);
        }
        false
    }

    fn signature_param_expects_borrow(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
        if sig.param_types.get(pidx).is_some_and(|t| matches!(t, Type::Reference(_))) {
            return true;
        }
        if sig
            .formal_param_types
            .get(pidx)
            .is_some_and(|t| matches!(t, Type::Reference(_)))
        {
            return true;
        }
        matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                sig, pidx,
            ),
            crate::analyzer::OwnershipMode::Borrowed
        )
    }

    fn callee_name_from_call_function(&self, function: &Expression<'ast>) -> Option<String> {
        match function {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                if let Some(prefix) = self.callee_name_from_call_function(object) {
                    Some(format!("{}::{}", prefix, field))
                } else {
                    Some(field.clone())
                }
            }
            _ => None,
        }
    }

    fn method_call_arg_formal_is_owned_non_copy(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        let Some(sig) = self.method_call_signature_for_arg(object, method, arg_index, func) else {
            return false;
        };
        let pidx = sig.arg_param_index(arg_index);
        let converged_ref = sig.param_types.get(pidx).is_some_and(|t| {
            matches!(t, Type::Reference(_) | Type::MutableReference(_))
        }) || matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                &sig, pidx,
            ),
            crate::analyzer::OwnershipMode::Borrowed | crate::analyzer::OwnershipMode::MutBorrowed
        );
        if converged_ref {
            return false;
        }
        sig.formal_param_type(pidx).is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                && !type_analysis::is_copy_type(t)
        })
    }

    fn method_call_signature_for_arg(
        &self,
        object: &Expression<'ast>,
        method: &str,
        _arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> Option<crate::analyzer::FunctionSignature> {
        let receiver_type = if let Expression::Identifier { name, .. } = object {
            if name == "self" && self.in_impl_block {
                self.current_struct_name.clone()
            } else {
                self.infer_local_binding_type_name(func.body.as_slice(), name)
                    .or_else(|| self.infer_type_name(object))
            }
        } else {
            self.mc_infer_method_receiver_type_name(object)
                .or_else(|| self.infer_type_name(object))
        };
        let rt = receiver_type?;
        let qualified = format!("{rt}::{method}");
        let mut sig = self
            .lookup_method_signature(&rt, method)
            .map(|ms| ms.to_function_signature())
            .or_else(|| self.signature_registry.get_signature(&qualified).cloned())
            .or_else(|| {
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(&qualified).cloned())
            })?;
        if let Some(reg) = self
            .signature_registry
            .get_signature(&qualified)
            .or_else(|| self.get_signature_with_global(&qualified))
        {
            if reg.emitted_rust_ref_params.is_some() {
                sig.emitted_rust_ref_params = reg.emitted_rust_ref_params.clone();
                sig.param_types = reg.param_types.clone();
                sig.param_ownership = reg.param_ownership.clone();
                sig.formal_param_types = reg.formal_param_types.clone();
                sig.forwarding_borrow_params = reg.forwarding_borrow_params.clone();
            }
        }
        Some(sig)
    }

    /// Align method registry with emitted Rust formals (`&Key`, `&str`, etc.) so later
    /// call sites in the same crate see converged borrow signatures.
    pub(in crate::codegen::rust) fn refresh_method_registry_from_emitted_formals(
        &mut self,
        func: &FunctionDecl<'ast>,
    ) {
        let Some(impl_type) = self.current_struct_name.clone() else {
            return;
        };
        let qualified = format!("{impl_type}::{}", func.name);

        if let Some(methods) = self.method_signatures_by_type.get_mut(&impl_type) {
            if let Some(sig) = methods.get_mut(func.name.as_str()) {
                let mut param_idx = 0;
                for param in &func.parameters {
                    if param.name == "self" {
                        continue;
                    }
                    if param_idx >= sig.param_types.len() {
                        break;
                    }
                    if self.inferred_mut_borrowed_params.contains(&param.name) {
                        if !matches!(sig.param_types[param_idx], Type::MutableReference(_)) {
                            sig.param_types[param_idx] =
                                Type::MutableReference(Box::new(param.type_.clone()));
                        }
                        if param_idx < sig.param_ownership.len() {
                            sig.param_ownership[param_idx] =
                                crate::analyzer::OwnershipMode::MutBorrowed;
                        }
                    } else if self.emitted_rust_ref_formals.contains(&param.name)
                        || self.str_ref_optimized_params.contains(&param.name)
                    {
                        if !matches!(sig.param_types[param_idx], Type::Reference(_)) {
                            sig.param_types[param_idx] =
                                Type::Reference(Box::new(param.type_.clone()));
                        }
                        if param_idx < sig.param_ownership.len() {
                            sig.param_ownership[param_idx] =
                                crate::analyzer::OwnershipMode::Borrowed;
                        }
                    } else if matches!(sig.param_types[param_idx], Type::Reference(_))
                        && !self.emitted_rust_ref_formals.contains(&param.name)
                    {
                        // Emitted Rust uses owned `T` (including Copy structs like KeyRange/Key).
                        sig.param_types[param_idx] = param.type_.clone();
                        if param_idx < sig.param_ownership.len() {
                            sig.param_ownership[param_idx] = crate::analyzer::OwnershipMode::Owned;
                        }
                    }
                    param_idx += 1;
                }

                let mut ms_emitted = vec![false; sig.param_types.len()];
                let mut user_param_idx = 0;
                for param in &func.parameters {
                    if param.name == "self" {
                        continue;
                    }
                    if user_param_idx < ms_emitted.len() {
                        ms_emitted[user_param_idx] = self.emitted_rust_ref_formals.contains(&param.name)
                            || self.str_ref_optimized_params.contains(&param.name);
                        ms_emitted[user_param_idx] = ms_emitted[user_param_idx]
                            && !self.inferred_mut_borrowed_params.contains(&param.name);
                    }
                    user_param_idx += 1;
                }
                sig.emitted_rust_ref_params = Some(ms_emitted);
            }
        }

        let base_sig = self
            .method_signatures_by_type
            .get(&impl_type)
            .and_then(|methods| methods.get(func.name.as_str()))
            .map(|sig| sig.to_function_signature());
        let mut updated = self
            .signature_registry
            .get_signature(&qualified)
            .cloned()
            .or_else(|| base_sig.clone())
            .unwrap_or_else(|| {
                let mut fs = crate::analyzer::FunctionSignature::default();
                fs.name = qualified.clone();
                fs.has_self_receiver = func.parameters.iter().any(|p| p.name == "self");
                fs
            });
        if let Some(ref ms) = base_sig {
            for (idx, pt) in ms.param_types.iter().enumerate() {
                let reg_idx = if ms.has_self_receiver { idx + 1 } else { idx };
                if reg_idx < updated.param_types.len() {
                    updated.param_types[reg_idx] = pt.clone();
                }
                if reg_idx < updated.param_ownership.len() {
                    if let Some(own) = ms.param_ownership.get(idx) {
                        updated.param_ownership[reg_idx] = *own;
                    }
                }
            }
        } else {
            for param in &func.parameters {
                if param.name == "self" {
                    continue;
                }
                let reg_idx = updated.param_types.len();
                updated.param_types.push(if self.emitted_rust_ref_formals.contains(&param.name)
                    || self.str_ref_optimized_params.contains(&param.name)
                {
                    Type::Reference(Box::new(param.type_.clone()))
                } else {
                    param.type_.clone()
                });
                updated.param_ownership.push(
                    if self.inferred_mut_borrowed_params.contains(&param.name) {
                        crate::analyzer::OwnershipMode::MutBorrowed
                    } else if self.emitted_rust_ref_formals.contains(&param.name)
                        || self.str_ref_optimized_params.contains(&param.name)
                    {
                        crate::analyzer::OwnershipMode::Borrowed
                    } else {
                        crate::analyzer::OwnershipMode::Owned
                    },
                );
                if updated.formal_param_types.len() <= reg_idx {
                    updated.formal_param_types.push(param.type_.clone());
                }
            }
        }
        let mut emitted = vec![false; updated.param_ownership.len()];
        let mut user_param_idx = 0;
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            let reg_idx = if updated.has_self_receiver {
                user_param_idx + 1
            } else {
                user_param_idx
            };
            if reg_idx < emitted.len() {
                // Owned WJ formals that emit `&mut T` in Rust are not shared-ref slots.
                let emits_shared_ref = self.emitted_rust_ref_formals.contains(&param.name)
                    || self.str_ref_optimized_params.contains(&param.name);
                emitted[reg_idx] =
                    emits_shared_ref && !self.inferred_mut_borrowed_params.contains(&param.name);
            }
            user_param_idx += 1;
        }
        updated.emitted_rust_ref_params = Some(emitted);
        self.signature_registry.add_function(qualified, updated);
    }

    /// Align free-function registry entries with emitted Rust formals so cross-file call
    /// sites see converged borrow signatures (e.g. `add_items(items: &mut Vec<i32>)`).
    pub(in crate::codegen::rust) fn refresh_free_function_registry_from_emitted_formals(
        &mut self,
        func: &FunctionDecl<'ast>,
        emitted_param_strings: &[String],
    ) {
        if self.current_struct_name.is_some() {
            return;
        }

        let mut keys = vec![func.name.clone()];
        if let Some(stem) = self.current_wj_file.file_stem() {
            let stem = stem.to_string_lossy();
            if !stem.is_empty() {
                keys.push(format!("{stem}::{}", func.name));
            }
        }

        keys.sort();
        keys.dedup();

        for key in keys {
            let mut updated = if let Some(existing) = self.signature_registry.get_signature(&key).cloned() {
                existing
            } else {
                let mut sig = crate::analyzer::FunctionSignature {
                    name: key.clone(),
                    param_types: func
                        .parameters
                        .iter()
                        .filter(|p| p.name != "self")
                        .map(|p| p.type_.clone())
                        .collect(),
                    formal_param_types: func
                        .parameters
                        .iter()
                        .filter(|p| p.name != "self")
                        .map(|p| p.type_.clone())
                        .collect(),
                    param_ownership: func
                        .parameters
                        .iter()
                        .filter(|p| p.name != "self")
                        .map(|_| crate::analyzer::OwnershipMode::Owned)
                        .collect(),
                    return_type: func.return_type.clone(),
                    ..Default::default()
                };
                if func.parameters.iter().any(|p| p.name == "self") {
                    sig.has_self_receiver = true;
                    sig.param_types.insert(0, Type::Custom("Self".to_string()));
                    sig.formal_param_types.insert(0, Type::Custom("Self".to_string()));
                    sig.param_ownership.insert(0, crate::analyzer::OwnershipMode::Borrowed);
                }
                sig
            };
            self.sync_registry_signature_from_emitted_formals(
                func,
                &mut updated,
                emitted_param_strings,
            );
            self.signature_registry.add_function(key.clone(), updated);
            // Rebuild mut-arg indices from this emission only — do not retain stale
            // `&mut` slots from a prior multipass when the formal is now owned.
            let mut mut_arg_indices = std::collections::HashSet::new();
            mut_arg_indices.extend(self.current_fn_emitted_mut_arg_indices.iter().copied());
            for (idx, param_str) in emitted_param_strings.iter().enumerate() {
                if (param_str.contains(": &mut ")
                    || param_str.contains(": &'a mut ")
                    || param_str.starts_with("&mut self")
                    || param_str.starts_with("&'a mut self"))
                    && param_str != "&mut self"
                    && param_str != "&'a mut self"
                {
                    let user_idx = if emitted_param_strings.first().is_some_and(|s| {
                        s.starts_with("&self") || s.starts_with("&mut self") || s.starts_with("mut self") || s == "self"
                    }) {
                        idx.saturating_sub(1)
                    } else {
                        idx
                    };
                    if user_idx < func.parameters.len() {
                        mut_arg_indices.insert(user_idx);
                    }
                }
            }
            self.function_emitted_mut_arg_indices
                .insert(key.clone(), mut_arg_indices);
        }
    }

    fn sync_registry_signature_from_emitted_formals(
        &self,
        func: &FunctionDecl<'ast>,
        updated: &mut crate::analyzer::FunctionSignature,
        emitted_param_strings: &[String],
    ) {
        let mut user_param_idx = 0;
        let mut emitted_idx = 0;
        for param in &func.parameters {
            if param.name == "self" {
                if emitted_idx < emitted_param_strings.len() {
                    emitted_idx += 1;
                }
                continue;
            }
            let reg_idx = if updated.has_self_receiver {
                user_param_idx + 1
            } else {
                user_param_idx
            };
            if reg_idx >= updated.param_types.len() {
                break;
            }
            let emitted_shared = emitted_param_strings.get(emitted_idx).is_some_and(|s| {
                (s.contains(": &") || s.contains(": &'a ") || s.starts_with("&self") || s.starts_with("&'a self"))
                    && !s.contains(": &mut ")
                    && !s.contains(": &'a mut ")
                    && !s.starts_with("&mut self")
                    && !s.starts_with("&'a mut self")
            });
            let emitted_mut = emitted_param_strings.get(emitted_idx).is_some_and(|s| {
                s.contains(": &mut ")
                    || s.contains(": &'a mut ")
                    || s.starts_with("&mut self")
            });
            if emitted_idx < emitted_param_strings.len() {
                emitted_idx += 1;
            }
            // Prefer the emitted formal string over stale inference:
            // `&T` beats MutBorrowed inference; `&mut T` beats shared-ref bookkeeping
            // (`emitted_rust_ref_formals` includes both `&` and `&mut` formals).
            if emitted_mut {
                if !matches!(updated.param_types[reg_idx], Type::MutableReference(_)) {
                    updated.param_types[reg_idx] =
                        Type::MutableReference(Box::new(param.type_.clone()));
                }
                if reg_idx < updated.param_ownership.len() {
                    updated.param_ownership[reg_idx] =
                        crate::analyzer::OwnershipMode::MutBorrowed;
                }
            } else if emitted_shared
                || (self.emitted_rust_ref_formals.contains(&param.name)
                    && !self.inferred_mut_borrowed_params.contains(&param.name))
            {
                if !matches!(updated.param_types[reg_idx], Type::Reference(_)) {
                    updated.param_types[reg_idx] = Type::Reference(Box::new(param.type_.clone()));
                }
                if reg_idx < updated.param_ownership.len() {
                    updated.param_ownership[reg_idx] =
                        crate::analyzer::OwnershipMode::Borrowed;
                }
            } else if self.inferred_mut_borrowed_params.contains(&param.name)
                && self.emitted_rust_ref_formals.contains(&param.name)
            {
                // Only keep MutBorrowed registry metadata when the emitted formal is `&mut T`.
                // Owned Copy aggregates (`mut deps: AppDeps`) clear inferred_mut before sync;
                // if metadata is stale, do not re-promote to MutableReference.
                if !matches!(updated.param_types[reg_idx], Type::MutableReference(_)) {
                    updated.param_types[reg_idx] =
                        Type::MutableReference(Box::new(param.type_.clone()));
                }
                if reg_idx < updated.param_ownership.len() {
                    updated.param_ownership[reg_idx] =
                        crate::analyzer::OwnershipMode::MutBorrowed;
                }
            } else if self.emitted_rust_ref_formals.contains(&param.name) {
                if !matches!(updated.param_types[reg_idx], Type::Reference(_)) {
                    updated.param_types[reg_idx] = Type::Reference(Box::new(param.type_.clone()));
                }
                if reg_idx < updated.param_ownership.len() {
                    updated.param_ownership[reg_idx] =
                        crate::analyzer::OwnershipMode::Borrowed;
                }
            } else if matches!(
                updated.param_types[reg_idx],
                Type::Reference(_) | Type::MutableReference(_)
            ) && !self.emitted_rust_ref_formals.contains(&param.name)
            {
                // Owned emitted formal: clear stale shared/mut-ref analyzer metadata so
                // call sites pass by value (`mut deps: AppDeps`, not `&mut deps`).
                updated.param_types[reg_idx] = param.type_.clone();
                if reg_idx < updated.param_ownership.len() {
                    updated.param_ownership[reg_idx] = crate::analyzer::OwnershipMode::Owned;
                }
            } else if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                updated, reg_idx,
            ) && !self.emitted_rust_ref_formals.contains(&param.name)
            {
                updated.param_types[reg_idx] = param.type_.clone();
                if reg_idx < updated.param_ownership.len() {
                    updated.param_ownership[reg_idx] = crate::analyzer::OwnershipMode::Owned;
                }
            }
            user_param_idx += 1;
        }

        let mut emitted = vec![false; updated.param_ownership.len()];
        let mut user_param_idx = 0;
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            let reg_idx = if updated.has_self_receiver {
                user_param_idx + 1
            } else {
                user_param_idx
            };
            if reg_idx < emitted.len() {
                emitted[reg_idx] = self.emitted_rust_ref_formals.contains(&param.name);
            }
            user_param_idx += 1;
        }
        updated.emitted_rust_ref_params = Some(emitted);
    }
}
