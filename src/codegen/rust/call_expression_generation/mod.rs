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
        // Match scrutinees parse `fs.write(...)` as Call(FieldAccess), not MethodCall.
        // Lower FieldAccess before macro dispatch so `write` is not mistaken for `write!`.
        if let Expression::FieldAccess {
            object: call_obj,
            field: ref call_method,
            ..
        } = function
        {
            // `reader.read_line(line)` / `read_to_string(buf)` (match scrutinee form): std fills a `&mut String`;
            // pass the buffer by mutable borrow, never a clone (a filled clone leaves `line` empty: endless loops).
            if matches!(call_method.as_str(), "read_line" | "read_to_string") && arguments.len() == 1 {
                if let Expression::Identifier { name, .. } = arguments[0].1 {
                    let recv = self.generate_expression(call_obj);
                    return format!("{recv}.{call_method}(&mut {name})");
                }
            }
            return trait_call_generation::generate_call_on_field_access(
                self,
                call_obj,
                call_method,
                arguments,
            );
        }

        let func_name = ast_utilities::extract_function_name(function);

        if let Some(done) =
            stdlib_call_generation::try_early_stdio_and_test_dispatch(self, function, arguments)
        {
            return done;
        }

        function_call_generation::generate_plain_function_call(
            self,
            func_name.as_str(),
            function,
            arguments,
        )
    }
}
