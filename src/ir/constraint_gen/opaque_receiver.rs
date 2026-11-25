//! OWN-OPAQUE-RECEIVER: a by-value parameter receiving a method no declaration describes keeps its declared owned mode.

use super::*;

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// OWN-OPAQUE-RECEIVER: a by-value parameter that receives a method no declaration describes (a type from a Rust
    /// crate: `base.active(on)` on `base: RailItem`) keeps its declared owned mode: the method may consume it, and
    /// demoting it to `&T` would make a consuming call fail to compile.
    pub(super) fn emit_owned_receiver_of_unknown_method(&mut self, object: &Expression<'ast>, method: &str) {
        let Expression::Identifier { name, location } = object else { return };
        let Some(param) = self.analyzed.decl.parameters.iter().find(|p| &p.name == name) else { return };
        // Only a nominal type Windjammer has no declaration for: not a std or runtime type (`db::Connection`), not a
        // struct of this crate or of a dependency, not a type parameter of the function.
        let undeclared = match &param.type_ {
            Type::Custom(ty) => {
                matches!(crate::ir::node::parser_type_to_base_type(&param.type_), BaseType::Custom(_))
                    && !self.struct_field_types.is_some_and(|m| m.contains_key(ty))
                    && !self.analyzed.decl.type_params.iter().any(|tp| &tp.name == ty)
                    && !self.registry.is_some_and(|r| r.runtime_module_for_type(ty).is_some())
            }
            _ => false,
        };
        if !undeclared {
            return;
        }
        if let Some(var) = self.param_var(name) {
            self.cs.add_because(
                Constraint::OwnershipIs(var, OwnedType::Owned),
                origin("OWN-OPAQUE-RECEIVER", format!("`{name}.{method}(..)` calls a method no declaration describes"), location),
            );
        }
    }
}
