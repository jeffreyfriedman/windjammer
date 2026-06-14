//! Method call signature resolution — delegates to unified resolver.

use crate::analyzer::FunctionSignature;
use crate::codegen::rust::call_signature_resolution::{self, ResolutionMethod};
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

        if let Some(ref tn) = type_name {
            let qualified_name = format!("{}::{}", tn, method);
            let resolved = call_signature_resolution::resolve_call_signature(
                &self.signature_registry,
                &qualified_name,
                Some(tn.as_str()),
                arguments.len(),
                &self.module_alias_map,
            );
            // Only accept resolutions that matched our type (exact, receiver, or
            // module-alias). Arg-count-validated suffix matches could pick a
            // completely different type's method (e.g. str::contains when we
            // asked for String::contains), which causes wrong coercion.
            let result = resolved
                .filter(|r| !matches!(r.resolution_method, ResolutionMethod::ArgCountValidated))
                .map(|r| r.sig);
            return result;
        }

        // No receiver type known: only suffix-match with arg-count validation.
        // Never do bare `get_signature(method)` — it could pick any type's method.
        // Skip `remove` specifically because it has incompatible semantics across types:
        // Vec::remove(usize) takes owned index, HashMap::remove(&K) takes borrowed key.
        if method == "remove" {
            return None;
        }
        self.signature_registry
            .find_signature_by_name_and_arg_count(method, arguments.len())
            .cloned()
    }
}
