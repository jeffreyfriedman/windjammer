//! FFI extern function codegen for `extern fn` declarations.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Generate extern "C" function declaration for FFI
    pub(super) fn generate_extern_function(&self, func: &FunctionDecl) -> String {
        let mut output = String::new();

        output.push_str("    pub fn ");
        output.push_str(&func.name);

        // Add type parameters if present
        if !func.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&func.type_params));
            output.push('>');
        }

        output.push('(');

        // Generate parameters - use FFI-safe types for extern "C"
        // string/String -> windjammer_runtime::ffi::FfiString (Rust String/&str are not C-compatible)
        let mut params: Vec<String> = Vec::new();
        for param in &func.parameters {
            if matches!(&param.type_, Type::Custom(name) if name == "str") {
                params.push(format!("{}_ptr: *const u8", param.name));
                params.push(format!("{}_len: usize", param.name));
            } else if matches!(&param.type_, Type::String)
                || matches!(&param.type_, Type::Custom(name) if name == "string" || name == "String")
            {
                params.push(format!(
                    "{}: windjammer_runtime::ffi::FfiString",
                    param.name
                ));
            } else {
                params.push(format!(
                    "{}: {}",
                    param.name,
                    self.type_to_rust(&param.type_)
                ));
            }
        }

        output.push_str(&params.join(", "));
        output.push(')');

        // Add return type - use FfiString for string returns
        if let Some(ret_type) = &func.return_type {
            output.push_str(" -> ");
            let rust_ret = if matches!(ret_type, Type::String)
                || matches!(ret_type, Type::Custom(name) if name == "string" || name == "String")
            {
                "windjammer_runtime::ffi::FfiString".to_string()
            } else {
                self.type_to_rust(ret_type)
            };
            output.push_str(&rust_ret);
        }

        output.push_str(";\n");
        output
    }
}
