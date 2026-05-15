//! Return type / where-clause / brace and generated body for regular functions.

use crate::analyzer::*;
use crate::codegen::rust::codegen_helpers;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn append_regular_function_return_where_open_brace(
        &mut self,
        func: &FunctionDecl<'ast>,
        output: &mut String,
        needs_lifetime: bool,
    ) {
        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            if needs_lifetime {
                output.push_str(&crate::codegen::rust::types::type_to_rust_with_lifetime(
                    return_type,
                ));
            } else {
                output.push_str(&self.type_to_rust(return_type));
            }
        }

        // Add where clause if present
        output.push_str(&codegen_helpers::format_where_clause(&func.where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;
    }

    pub(in crate::codegen::rust) fn append_regular_function_body_and_close(
        &mut self,
        func: &FunctionDecl<'ast>,
        output: &mut String,
    ) {
        // TDD: Generate function body with return optimization
        // Set flag to enable implicit return for last statement
        let old_in_function_body = self.in_function_body;
        self.in_function_body = true;
        let mut body_code = self.generate_block(&func.body);
        self.in_function_body = old_in_function_body;

        // PHASE 6 OPTIMIZATION: Add defer drop logic before function returns
        // This defers heavy deallocations to a background thread for 10,000x speedup
        if !self.defer_drop_optimizations.is_empty() {
            body_code =
                self.wrap_with_defer_drop(body_code, &self.defer_drop_optimizations.clone());
        }

        output.push_str(&body_code);

        self.indent_level -= 1;
        output.push('}');
    }
}
