//! Function Generation Module
//!
//! Handles generation of Rust code for function declarations, including:
//! - Regular functions and methods
//! - Extern/FFI function declarations
//! - Functions with decorator wrapping (timeout, bench, requires, ensures, etc.)
//! - Parameterized tests (@test_cases)
//! - Self parameter inference and builder pattern detection

use crate::analyzer::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(crate) fn generate_function(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        let func = &analyzed.decl;

        // PARAMETERIZED TESTS: Check for @test_cases decorator
        // If present, generate multiple test functions instead of one
        if let Some(test_cases_decorator) = func.decorators.iter().find(|d| d.name == "test_cases")
        {
            return self.generate_parameterized_tests(analyzed, test_cases_decorator);
        }

        // TESTING DECORATORS: Check for decorators that need to wrap the function body
        // These include: @timeout, @bench, @requires, @ensures, @property_test, @test(setup/teardown)
        if self.has_wrapping_decorator(func) {
            return self.generate_function_with_wrapping(analyzed);
        }

        let mut output = String::new();

        self.push_auto_test_attribute_if_needed(func, &mut output);
        self.select_ir_function_for(&func.name);
        self.prepare_codegen_environment_for_regular_function(analyzed);

        let needs_lifetime =
            self.append_regular_function_signature_prefix(analyzed, func, &mut output);

        let mut params = Vec::new();
        self.extend_implicit_self_parameters(analyzed, func, &mut params);

        let unused_params = self.compute_unused_formal_parameter_names(func);
        self.refresh_unused_let_bindings_for_function_body(&func.body);

        params.extend(self.collect_additional_formal_parameter_strings(
            analyzed,
            func,
            needs_lifetime,
            &unused_params,
        ));

        self.refresh_method_registry_from_emitted_formals(func);
        self.refresh_free_function_registry_from_emitted_formals(func, &params);

        output.push_str(&params.join(", "));
        output.push(')');

        self.append_regular_function_return_where_open_brace(func, &mut output, needs_lifetime);
        self.append_regular_function_body_and_close(func, &mut output);

        // LOCAL VARIABLE TRACKING: Pop scope when exiting function
        self.local_variable_scopes.pop();

        output
    }

    /// Populate `signature_registry` / `function_emitted_mut_arg_indices` from emitted
    /// formals before any function bodies run (so `main` call sites see callee metadata).
    pub(in crate::codegen::rust) fn preregister_function_formals_in_registry(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
    ) {
        let func = &analyzed.decl;
        if func.decorators.iter().any(|d| d.name == "test_cases") || self.has_wrapping_decorator(func)
        {
            return;
        }

        self.select_ir_function_for(&func.name);
        self.prepare_codegen_environment_for_regular_function(analyzed);

        let mut signature_scratch = String::new();
        let needs_lifetime =
            self.append_regular_function_signature_prefix(analyzed, func, &mut signature_scratch);

        let mut params = Vec::new();
        self.extend_implicit_self_parameters(analyzed, func, &mut params);

        let unused_params = self.compute_unused_formal_parameter_names(func);
        self.refresh_unused_let_bindings_for_function_body(&func.body);

        params.extend(self.collect_additional_formal_parameter_strings(
            analyzed,
            func,
            needs_lifetime,
            &unused_params,
        ));

        self.refresh_method_registry_from_emitted_formals(func);
        self.refresh_free_function_registry_from_emitted_formals(func, &params);
    }

    /// Preregister every method on `impl_type` (all impl blocks) once so forward refs like
    /// `put_value` → `apply_patch_put` see converged `&Key` formals before any body emits.
    pub(in crate::codegen::rust) fn preregister_all_impl_siblings_for_type_if_needed(
        &mut self,
        impl_type: &str,
        impl_trait: Option<&str>,
        analyzed: &[AnalyzedFunction<'ast>],
    ) {
        let prev_struct = self.current_struct_name.clone();
        let prev_in_impl = self.in_impl_block;
        self.current_struct_name = Some(impl_type.to_string());
        self.in_impl_block = true;

        let mut siblings: Vec<&AnalyzedFunction<'_>> = analyzed
            .iter()
            .filter(|af| {
                af.decl.parent_type.as_deref() == Some(impl_type)
                    && af.decl.impl_trait.as_deref() == impl_trait
                    && !af.decl.is_extern
            })
            .collect();

        if !self.preregistered_impl_sibling_types.contains(impl_type) {
            for af in analyzed.iter().filter(|af| {
                af.decl.parent_type.as_deref() == Some(impl_type)
                    && af.decl.impl_trait.as_deref() == impl_trait
                    && !af.decl.is_extern
            }) {
                self.register_impl_ast_method_formals(impl_type, &af.decl);
            }

            siblings.sort_by_key(|af| {
                af.decl.parameters.iter().any(|p| {
                    p.name != "self"
                        && self.param_passed_to_borrowing_callee(
                            af.decl.body.as_slice(),
                            &p.name,
                            &af.decl,
                        )
                }) as u8
            });

            for _ in 0..4 {
                for af in &siblings {
                    self.select_ir_function_for(&af.decl.name);
                    self.prepare_codegen_environment_for_regular_function(af);
                }
            }

            self.preregistered_impl_sibling_types
                .insert(impl_type.to_string());
        }

        for af in siblings {
            self.select_ir_function_for(&af.decl.name);
            self.preregister_function_formals_in_registry(af);
        }

        self.current_struct_name = prev_struct;
        self.in_impl_block = prev_in_impl;
    }
}
