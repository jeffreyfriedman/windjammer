//! Built-in / standard-library style call dispatch before general call lowering.

use crate::parser::*;

use super::super::{ast_utilities, CodeGenerator};

/// Print/println macros, test macros/runtime qualification, and `assert` lowering.
pub(in crate::codegen::rust) fn try_early_stdio_and_test_dispatch<'ast>(
    gen: &mut CodeGenerator<'ast>,
    function: &Expression<'ast>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Option<String> {
    let func_name = ast_utilities::extract_function_name(function);

    if let Some(print_macro) = gen.try_generate_print_macro(&func_name, arguments) {
        return Some(print_macro);
    }

    let is_user_defined = gen
        .signature_registry
        .get_signature(&func_name)
        .map(|sig| !sig.is_extern)
        .unwrap_or(false);

    if !is_user_defined {
        if let Some(macro_call) = gen.try_generate_test_macro(&func_name, arguments) {
            return Some(macro_call);
        }

        if let Some(qualified_call) = gen.try_qualify_test_function(&func_name, arguments) {
            return Some(qualified_call);
        }
    }

    if func_name == "assert" {
        let args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| gen.generate_expression(arg))
            .collect();
        return Some(format!("assert!({})", args.join(", ")));
    }

    None
}
