//! Stage 1 of "ownership decided once" (docs/design/ownership-decided-once.md), classes 1 and 2: rules that the Rust backend applied to `string` formals after the solver are decided here, so callers read
//! the formal the callee emits (D-OWN-STAGES `ir=& emitted=owned` disappears for the class).
//!
//! Class 1, OWN-PUB-TEXT-UNUSED: a `string` parameter of a `pub` free function that the body never reads is owned.
//! Its formal is part of the module's API and a sibling that consumes the same value shares the call site's
//! `.to_string()`; the backend held it owned in `pub_module_api_keeps_owned_string_formal`
//! (src/codegen/rust/function_formal_parameter_generation.rs, first branch).
//!
//! Class 2, OWN-TEXT-API-UNUSED: the same for an unused text formal of a method or function returning text (the
//! backend's text-API branch emitted `String`); a split `impl` in another file is covered because the published
//! signature carries the solved mode before the callee's file is generated.

use super::*;
use crate::parser::FunctionDecl;

/// The rule that makes `param` an owned API formal, when one does: a `string` formal the body never reads, of
/// a `pub` free function (class 1) or of a function or method returning text, a split `impl` included (class 2).
/// The solver's constraint and the published signature both read this one declaration-level fact.
pub(crate) fn unused_text_api_formal(decl: &FunctionDecl<'_>, param: &crate::parser::Parameter<'_>) -> Option<&'static str> {
    use crate::ir::formal_predicates::is_windjammer_text_type as text;
    if decl.is_extern || decl.impl_trait.is_some() || param.pattern.is_some() || !text(&param.type_) {
        return None;
    }
    let returns_text = decl.return_type.as_ref().is_some_and(|t| match t {
        Type::Result(ok, _) => text(ok),
        other => text(other),
    });
    let class1 = decl.is_pub && decl.parent_type.is_none();
    if !(class1 || returns_text) || crate::analyzer::task_structure::block_mentions(&decl.body, &param.name) {
        return None;
    }
    Some(if class1 { "OWN-PUB-TEXT-UNUSED" } else { "OWN-TEXT-API-UNUSED" })
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    pub(super) fn emit_owned_for_unused_pub_text_formals(&mut self) {
        let decl = &self.analyzed.decl;
        for param in &decl.parameters {
            let Some(rule) = unused_text_api_formal(decl, param) else { continue };
            if let Some(&var) = self.param_vars.get(&param.name) {
                self.cs.add_because(
                    Constraint::OwnershipIs(var, OwnedType::Owned),
                    origin(rule, format!("`{}` is an unused `string` formal of an API: owned", param.name), &None),
                );
            }
        }
    }
}
