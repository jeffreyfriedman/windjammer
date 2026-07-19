//! Setup and per-function analyzer state loaded before emitting a regular function.

use crate::analyzer::*;
use crate::codegen::rust::type_analysis;
use crate::parser::*;

use super::CodeGenerator;

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
        for param in &func.parameters {
            if param.name != "self"
                && (self.param_passes_to_wj_owned_sibling_call(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) || self.param_passed_to_owned_self_method_arg(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ))
            {
                self.current_fn_mixed_forwarder_params
                    .insert(param.name.clone());
            }
        }
        for (param_name, ownership) in self.get_all_param_ownership(analyzed) {
            if param_name != "self" {
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
                self.inferred_borrowed_params.remove(&param.name);
                self.inferred_mut_borrowed_params.remove(&param.name);
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
                match self.get_param_ownership(&param.name, analyzed) {
                    Some(crate::analyzer::OwnershipMode::Borrowed) => {
                        self.inferred_borrowed_params.insert(param.name.clone());
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

    /// Map/set key params keep owned formals in impl methods and free functions.
    pub(in crate::codegen::rust) fn is_collection_key_owned_param(
        &self,
        param: &Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        if self.collection_key_owned_params.contains(&param.name) {
            return true;
        }
        if crate::codegen::rust::types::is_windjammer_text_type(&param.type_) {
            if self.str_ref_optimized_params.contains(&param.name)
                || self.inferred_borrowed_params.contains(&param.name)
            {
                return false;
            }
            // Text keys: owned formals for HashMap forwarding only; HashSet::contains keeps &str.
            return self.param_only_forwarded_to_map_key_callee(
                func.body.as_slice(),
                &param.name,
                func,
            ) || (func.parent_type.is_none()
                && self.param_only_forwarded_to_qualified_map_key_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ));
        }
        self.param_only_forwarded_to_collection_key_callee(
            func.body.as_slice(),
            &param.name,
            func,
        ) || (func.parent_type.is_none()
            && self.param_only_forwarded_to_qualified_collection_key_callee(
                func.body.as_slice(),
                &param.name,
                func,
            ))
    }

    /// Map/set key helpers keep owned `String` formals; call sites add `&` via collection-key finalization.
    fn preserve_owned_formals_for_collection_key_only_params(
        &mut self,
        func: &FunctionDecl<'ast>,
    ) {
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            let preserve = if crate::codegen::rust::types::is_windjammer_text_type(&param.type_) {
                if self.str_ref_optimized_params.contains(&param.name)
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
                }
            } else {
                self.param_only_forwarded_to_qualified_collection_key_callee(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) || self.param_only_forwarded_to_collection_key_callee(
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

    /// When one `Vec` param is borrow-inferred, unused sibling `Vec` params should match
    /// (`register(deps: &Vec, outs: Vec)` → both `&Vec` at call sites).
    fn promote_unused_readonly_vec_params_to_borrowed(&mut self, func: &FunctionDecl<'ast>) {
        let has_borrowed_vec = func.parameters.iter().any(|p| {
            p.name != "self"
                && matches!(p.type_, Type::Vec(_))
                && self.inferred_borrowed_params.contains(&p.name)
        });
        if !has_borrowed_vec {
            return;
        }
        let unused = self.compute_unused_formal_parameter_names(func);
        for param in &func.parameters {
            if param.name == "self" || !matches!(param.type_, Type::Vec(_)) {
                continue;
            }
            if self.inferred_borrowed_params.contains(&param.name)
                || self.inferred_mut_borrowed_params.contains(&param.name)
            {
                continue;
            }
            if unused.contains(&param.name)
                && !self.param_has_owning_method_use(func.body.as_slice(), &param.name, func)
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
                || self
                    .current_struct_name
                    .as_ref()
                    .is_some_and(|sn| self.struct_is_owned_engine_key_facade(sn, param))
            {
                self.inferred_borrowed_params.remove(&param.name);
                self.inferred_mut_borrowed_params.remove(&param.name);
            }
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

                // Module-level `string` formals stay owned; impl methods may converge to &str.
                if ast_owned_string && is_module_level {
                    p_type = param.type_.clone();
                    ownership = crate::analyzer::OwnershipMode::Owned;
                } else if let Some(reg) = registry_sig {
                    if let Some(formal) = reg.formal_param_type(idx) {
                        if crate::codegen::rust::types::is_windjammer_text_type(formal)
                            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                            && is_module_level
                        {
                            p_type = formal.clone();
                            ownership = crate::analyzer::OwnershipMode::Owned;
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

                // Emitted Rust formals follow body-inferred ownership; IR param_types can
                // lag as Owned for non-Copy user types (e.g. wdb `Key` → `&Key` at call sites).
                if let Some(analyzed_own) = analyzed.inferred_ownership.get(&param.name) {
                    if matches!(
                        analyzed_own,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    ) && matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                        && !type_analysis::is_copy_type(&p_type)
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
                if analyzed.str_ref_optimizable_params.contains(&param.name) {
                    ownership = crate::analyzer::OwnershipMode::Borrowed;
                }

                if matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                    && !type_analysis::is_copy_type(&p_type)
                    && self.param_passed_to_borrowing_callee(func.body.as_slice(), &param.name, func)
                    && !self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
                    && !self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
                {
                    ownership = crate::analyzer::OwnershipMode::Borrowed;
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
                {
                    return false;
                }
                if !forwarded
                    && !self.param_used_as_read_operand(func.body.as_slice(), &param.name)
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
                let discard = matches!(
                    pattern,
                    Pattern::Identifier(name) if name == "_"
                );
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
                self.expression_uses_param_as_read_operand(iterable, param_name)
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
                        if self.method_call_sibling_ast_expects_owned_arg(
                            object, method, i, func,
                        ) {
                            return true;
                        }
                        if self.method_call_arg_formal_is_owned_non_copy(object, method, i, func) {
                            return true;
                        }
                        if self.method_call_arg_expects_borrow(object, method, i, func) {
                            continue;
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

    /// True when the body forwards `param_name` to a borrowing callee and also reads
    /// the param afterward (forward-ref pattern: keep owned formal, borrow at call site).
    pub(in crate::codegen::rust) fn param_has_forward_ref_keep_owned(
        &self,
        body: &[&'ast Statement<'ast>],
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
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
                self.expression_passes_param_as_call_argument(expr, param_name, func)
            }
            Statement::Let { value, else_block, .. } => {
                self.expression_passes_param_as_call_argument(value, param_name, func)
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
                self.expression_passes_param_as_call_argument(iterable, param_name, func)
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

    fn expression_passes_param_as_call_argument(
        &self,
        expr: &Expression<'ast>,
        param_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        match expr {
            Expression::MethodCall { arguments, .. } => arguments.iter().any(|(_, arg)| {
                matches!(arg, Expression::Identifier { name, .. } if name == param_name)
            }),
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                matches!(arg, Expression::Identifier { name, .. } if name == param_name)
            }),
            Expression::Binary { left, right, .. } => {
                self.expression_passes_param_as_call_argument(left, param_name, func)
                    || self.expression_passes_param_as_call_argument(right, param_name, func)
            }
            Expression::Unary { operand, .. } => {
                self.expression_passes_param_as_call_argument(operand, param_name, func)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_passes_param_as_call_argument(object, param_name, func)
            }
            Expression::Index { object, index, .. } => {
                self.expression_passes_param_as_call_argument(object, param_name, func)
                    || self.expression_passes_param_as_call_argument(index, param_name, func)
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
                if matches!(*arg, Expression::Identifier { name, .. } if name == param_name) {
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
                    if matches!(*arg, Expression::Identifier { name, .. } if name == param_name) {
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

    fn method_call_sibling_ast_expects_owned_arg(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        func: &FunctionDecl<'ast>,
    ) -> bool {
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
        let Some(ms) = self.lookup_method_signature(&rt, method) else {
            return false;
        };
        let mut sig = ms.to_function_signature();
        let qualified = format!("{rt}::{method}");
        if let Some(reg) = self
            .signature_registry
            .get_signature(&qualified)
            .or_else(|| self.get_signature_with_global(&qualified))
        {
            if reg.emitted_rust_ref_params.is_some() {
                sig.emitted_rust_ref_params = reg.emitted_rust_ref_params.clone();
                sig.formal_param_types = reg.formal_param_types.clone();
                sig.forwarding_borrow_params = reg.forwarding_borrow_params.clone();
            }
        }
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

    /// Multi-method engine facade (get/put/delete) keeps owned `Key`/`Value` formals (WDB-046).
    pub(in crate::codegen::rust) fn struct_is_owned_engine_key_facade(
        &self,
        struct_name: &str,
        param: &crate::parser::Parameter,
    ) -> bool {
        if param.name == "self" || type_analysis::is_copy_type(&param.type_) {
            return false;
        }
        if !matches!(&param.type_, Type::Custom(name) if name == "Key" || name == "Value") {
            return false;
        }
        ["get", "put", "delete"]
            .iter()
            .all(|name| self.current_impl_methods.contains(*name))
    }

    /// True when a non-`self` param should emit `&T`/`&str` because the body only forwards to borrowing callees.
    pub(in crate::codegen::rust) fn param_should_emit_borrowed_delegation_formal(
        &self,
        param: &crate::parser::Parameter,
        func: &FunctionDecl<'ast>,
    ) -> bool {
        param.name != "self"
            && !type_analysis::is_copy_type(&param.type_)
            && !matches!(
                &param.type_,
                Type::Reference(_) | Type::MutableReference(_)
            )
            && !self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
            && !self.current_fn_mixed_forwarder_params.contains(&param.name)
            && !self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
            && !self.current_struct_name.as_ref().is_some_and(|sn| {
                self.struct_is_owned_engine_key_facade(sn, param)
            })
            && self.param_passed_to_borrowing_callee(func.body.as_slice(), &param.name, func)
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
            if self.inferred_mut_borrowed_params.contains(&param.name) {
                if !matches!(sig.param_types[param_idx], Type::MutableReference(_)) {
                    sig.param_types[param_idx] =
                        Type::MutableReference(Box::new(param.type_.clone()));
                }
                if param_idx < sig.param_ownership.len() {
                    sig.param_ownership[param_idx] = crate::analyzer::OwnershipMode::MutBorrowed;
                }
            } else if self.emitted_rust_ref_formals.contains(&param.name)
                || self.str_ref_optimized_params.contains(&param.name)
            {
                if !matches!(sig.param_types[param_idx], Type::Reference(_)) {
                    sig.param_types[param_idx] = Type::Reference(Box::new(param.type_.clone()));
                }
                if param_idx < sig.param_ownership.len() {
                    sig.param_ownership[param_idx] = crate::analyzer::OwnershipMode::Borrowed;
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

        let qualified = format!("{impl_type}::{}", func.name);
        let mut updated = self
            .signature_registry
            .get_signature(&qualified)
            .cloned()
            .unwrap_or_else(|| {
                let mut fs = sig.to_function_signature();
                fs.name = qualified.clone();
                fs
            });
        for (idx, pt) in sig.param_types.iter().enumerate() {
            let reg_idx = if sig.has_self_receiver { idx + 1 } else { idx };
            if reg_idx < updated.param_types.len() {
                updated.param_types[reg_idx] = pt.clone();
            }
            if reg_idx < updated.param_ownership.len() {
                if let Some(own) = sig.param_ownership.get(idx) {
                    updated.param_ownership[reg_idx] = *own;
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
            let mut mut_arg_indices = self
                .function_emitted_mut_arg_indices
                .entry(key.clone())
                .or_default();
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
            let emitted_mut = emitted_param_strings.get(emitted_idx).is_some_and(|s| {
                s.contains(": &mut ")
                    || s.contains(": &'a mut ")
                    || s.starts_with("&mut self")
            });
            if emitted_idx < emitted_param_strings.len() {
                emitted_idx += 1;
            }
            if emitted_mut || self.inferred_mut_borrowed_params.contains(&param.name) {
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
            } else if matches!(updated.param_types[reg_idx], Type::Reference(_))
                && !self.emitted_rust_ref_formals.contains(&param.name)
            {
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
