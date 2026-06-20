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

        // AUTO-CLONE: Load auto-clone analysis for this function
        self.auto_clone_analysis = Some(analyzed.auto_clone_analysis.clone());
        self.auto_clone_counter = 0;

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        self.current_function_params = func.parameters.clone();
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

        // Track parameters inferred as borrowed/mut-borrowed for codegen decisions
        self.inferred_borrowed_params.clear();
        self.inferred_mut_borrowed_params.clear();
        self.str_ref_optimized_params.clear();
        for (param_name, ownership) in &analyzed.inferred_ownership {
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

        // Track Phase 2 string-optimized parameters (string type params that become &str)
        for param_name in &analyzed.str_ref_optimizable_params {
            self.str_ref_optimized_params.insert(param_name.clone());
        }

        // Any parameter the analyzer lowered to `Reference(str)` generates as `&str` in Rust.
        // Call-site borrow helpers must treat these as already referenced (map.get(key) not get(&key)).
        for (idx, param) in func.parameters.iter().enumerate() {
            if let Some(Type::Reference(inner)) = analyzed.inferred_param_types.get(idx) {
                if matches!(&**inner, Type::Custom(s) if s == "str") {
                    self.str_ref_optimized_params.insert(param.name.clone());
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
            let ownership_vec: Vec<(String, crate::analyzer::OwnershipMode)> = analyzed
                .inferred_ownership
                .iter()
                .filter(|(name, _)| name.as_str() != "self")
                .map(|(name, mode)| (name.clone(), *mode))
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
        
        // Skip method registry for static methods - always use global registry for consistency.
        // Body-inferred Borrowed from double-use shouldn't override converged Owned formals.
        // The global registry has the converged ownership after multipass analysis.
        if !has_self_receiver {
            if let Some(global_sig) = self.global_signature_registry() {
                if global_sig.get_signature(&qualified).is_some() {
                    return;
                }
            }
        }
        
        let registry_sig = self.signature_registry.get_signature(&qualified);
        let mut param_types = Vec::new();
        let mut param_ownership = Vec::new();

        for (idx, param) in func.parameters.iter().enumerate() {
            if param.name != "self" {
                let mut p_type = analyzed
                    .inferred_param_types
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| param.type_.clone());

                let ownership = if has_self_receiver {
                    analyzed
                        .inferred_ownership
                        .get(&param.name)
                        .copied()
                        .unwrap_or(crate::analyzer::OwnershipMode::Borrowed)
                } else {
                    registry_sig
                        .and_then(|sig| sig.param_ownership.get(idx).copied())
                        .or_else(|| analyzed.inferred_ownership.get(&param.name).copied())
                        .unwrap_or(crate::analyzer::OwnershipMode::Borrowed)
                };

                if !has_self_receiver {
                    if let Some(reg) = registry_sig {
                        if let Some(formal_ty) = reg.param_types.get(idx) {
                            if matches!(ownership, crate::analyzer::OwnershipMode::Owned) {
                                p_type = formal_ty.clone();
                            }
                        }
                    }
                }

                let stored_type = match ownership {
                    crate::analyzer::OwnershipMode::Borrowed
                        if !matches!(
                            &p_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) && !type_analysis::is_copy_type(&p_type) =>
                    {
                        Type::Reference(Box::new(p_type))
                    }
                    crate::analyzer::OwnershipMode::MutBorrowed
                        if !matches!(&p_type, Type::MutableReference(_)) =>
                    {
                        Type::MutableReference(Box::new(p_type))
                    }
                    _ => p_type,
                };
                param_types.push(stored_type);
                param_ownership.push(ownership);
            }
        }

        let signature = crate::codegen::rust::generator::MethodSignature::new(
            impl_type.to_string(),
            func.name.clone(),
            param_types,
            param_ownership,
            func.return_type.clone(),
            has_self_receiver,
        );

        self.register_method_signature(signature);
    }
}
