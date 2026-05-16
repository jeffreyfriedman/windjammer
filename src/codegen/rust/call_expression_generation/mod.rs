//! Call expression generation
//!
//! Handles code generation for function calls including:
//! - Regular function calls
//! - Test macros (assert_*, property_test, etc.)
//! - Print/println macros
//! - Type casting and auto-clone insertion
//! - Parameter type balancing

mod argument_generation;
mod function_call_generation;
mod stdlib_call_generation;
mod trait_call_generation;

use crate::parser::*;

use super::{ast_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a function call expression
    pub(in crate::codegen::rust) fn generate_call_expression(
        &mut self,
        function: &Expression<'ast>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> String {
        let func_name = ast_utilities::extract_function_name(function);

        if let Some(done) =
            stdlib_call_generation::try_early_stdio_and_test_dispatch(self, function, arguments)
        {
            return done;
        }

        if let Expression::FieldAccess {
            object: call_obj,
            field: ref call_method,
            ..
        } = function
        {
            return trait_call_generation::generate_call_on_field_access(
                self,
                call_obj,
                call_method,
                arguments,
            );
        }

        function_call_generation::generate_plain_function_call(
            self,
            func_name.as_str(),
            function,
            arguments,
        )
    }
}
