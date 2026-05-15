//! Function Generation Module
//!
//! Handles generation of Rust code for function declarations, including:
//! - Regular functions and methods
//! - Extern/FFI function declarations
//! - Functions with decorator wrapping (timeout, bench, requires, ensures, etc.)
//! - Parameterized tests (@test_cases)
//! - Self parameter inference and builder pattern detection

use crate::analyzer::*;
use crate::parser::*;

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

        output.push_str(&params.join(", "));
        output.push(')');

        self.append_regular_function_return_where_open_brace(func, &mut output, needs_lifetime);
        self.append_regular_function_body_and_close(func, &mut output);

        // LOCAL VARIABLE TRACKING: Pop scope when exiting function
        self.local_variable_scopes.pop();

        output
    }
}
