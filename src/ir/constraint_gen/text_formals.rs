//! Stage 1 of "ownership decided once" (docs/design/ownership-decided-once.md), class 1 (DRAFT, unbuilt, not for
//! merge): rules that the Rust backend applied to `string` formals after the solver are decided here, so callers read
//! the formal the callee emits (D-OWN-STAGES `ir=& emitted=owned` disappears for the class).
//!
//! Class 1, OWN-PUB-TEXT-UNUSED: a `string` parameter of a `pub` free function that the body never reads is owned.
//! Its formal is part of the module's API and a sibling that consumes the same value shares the call site's
//! `.to_string()`; the backend held it owned in `pub_module_api_keeps_owned_string_formal`
//! (src/codegen/rust/function_formal_parameter_generation.rs, first branch).

use super::*;

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    pub(super) fn emit_owned_for_unused_pub_text_formals(&mut self) {
        let decl = &self.analyzed.decl;
        if !decl.is_pub || decl.parent_type.is_some() || decl.is_extern {
            return;
        }
        for param in &decl.parameters {
            let text = crate::ir::formal_predicates::is_windjammer_text_type(&param.type_);
            if !text || param.pattern.is_some() || crate::analyzer::task_structure::block_mentions(&decl.body, &param.name) {
                continue;
            }
            if let Some(&var) = self.param_vars.get(&param.name) {
                self.cs.add_because(
                    Constraint::OwnershipIs(var, OwnedType::Owned),
                    origin("OWN-PUB-TEXT-UNUSED", format!("`{}` is an unused `string` formal of a `pub` function: owned API", param.name), &None),
                );
            }
        }
    }
}
