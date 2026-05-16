//! Closure expression generation
//!
//! Handles generation of:
//! - Closure expressions (|params| body)
//! - Smart `move` inference for compiler-generated closures

use crate::parser::Expression;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for closure expression |params| body
    /// THE WINDJAMMER WAY: Smart `move` inference for closures.
    /// - Compiler-generated closures (params start with __) → add `move`
    /// - User-written closures → preserve as-is (respect explicit intent)
    /// - Closures capturing `self` → don't add `move` (UI callbacks need to borrow)
    pub(in crate::codegen::rust) fn generate_closure(
        &mut self,
        parameters: &[String],
        body: &Expression<'ast>,
    ) -> String {
        let params = parameters.join(", ");

        // Check if this is a compiler-generated closure (params start with __)
        let is_compiler_generated = parameters.iter().any(|p| p.starts_with("__"));

        // Check if the closure body references `self`
        let captures_self = self.expression_references_self(body);

        // For user-written closures, set flag and track params to suppress transformations
        let prev_in_user_closure = self.in_user_written_closure;
        let mut prev_closure_params = None;
        if !is_compiler_generated {
            self.in_user_written_closure = true;
            prev_closure_params = Some(std::mem::take(&mut self.user_closure_params));
            for param in parameters {
                self.user_closure_params.insert(param.clone());
            }
        }

        // Generate closure body with context flags set
        let body_str = self.generate_expression(body);

        // Restore previous state
        if !is_compiler_generated {
            self.in_user_written_closure = prev_in_user_closure;
            if let Some(prev_params) = prev_closure_params {
                self.user_closure_params = prev_params;
            }
        }

        if is_compiler_generated && !captures_self {
            // Compiler-generated closure that doesn't capture self → add `move`
            format!("move |{}| {}", params, body_str)
        } else {
            // User-written closure or captures self → preserve as-is
            format!("|{}| {}", params, body_str)
        }
    }
}
