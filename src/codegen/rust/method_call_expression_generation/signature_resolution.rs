//! Qualified and unqualified method signature resolution during codegen.

use crate::analyzer::FunctionSignature;
use crate::parser::Expression;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn mc_resolve_method_call_signature(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> Option<FunctionSignature> {
        let type_name = self.infer_type_name(object);
        let out = if let Some(ref type_name) = type_name {
            let qualified_name = format!("{}::{}", type_name, method);
            let mut sig = self
                .signature_registry
                .get_signature(&qualified_name)
                .cloned();
            // Validate argument count vs signature to dodge name collisions.
            if let Some(ref found_sig) = sig {
                let expected_args = if found_sig.has_self_receiver {
                    found_sig.param_ownership.len().saturating_sub(1)
                } else {
                    found_sig.param_ownership.len()
                };
                if expected_args != arguments.len() {
                    sig = None;
                    for (key, alt_sig) in &self.signature_registry.signatures {
                        if key.ends_with(&format!("::{}", qualified_name))
                            && key != &qualified_name
                        {
                            let alt_args = if alt_sig.has_self_receiver {
                                alt_sig.param_ownership.len().saturating_sub(1)
                            } else {
                                alt_sig.param_ownership.len()
                            };
                            if alt_args == arguments.len() {
                                sig = Some(alt_sig.clone());
                                break;
                            }
                        }
                    }
                }
            }
            sig
        } else {
            if crate::codegen::rust::stdlib_method_traits::is_common_stdlib_method(method) {
                None
            } else {
                self.signature_registry
                    .get_signature(method)
                    .cloned()
                    .or_else(|| {
                        let suffix_sig = self
                            .signature_registry
                            .find_signature_ending_with(method)
                            .cloned();
                        if let Some(ref sig) = suffix_sig {
                            let expected_args = if sig.has_self_receiver {
                                sig.param_ownership.len().saturating_sub(1)
                            } else {
                                sig.param_ownership.len()
                            };
                            if expected_args == arguments.len() {
                                return suffix_sig;
                            }
                        }
                        None
                    })
            }
        };

        out
    }
}
