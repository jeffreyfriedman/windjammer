//! Method call signature resolution — delegates to unified resolver.

use crate::analyzer::FunctionSignature;
use crate::parser::Expression;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Resolve the receiver type for method signature lookup (`self.field` → field type, not owner struct).
    pub(in crate::codegen::rust) fn mc_infer_method_receiver_type_name(
        &self,
        object: &Expression<'ast>,
    ) -> Option<String> {
        if let Expression::FieldAccess {
            object: obj,
            field,
            ..
        } = object
        {
            if let Expression::Identifier { name, .. } = &**obj {
                if name == "self" {
                    if let Some(sn) = &self.current_struct_name {
                        if let Some(fields) = self.lookup_struct_field_types(sn) {
                            if let Some(ft) = fields.get(field.as_str()) {
                                if let Some(tn) = Self::type_to_name(ft) {
                                    return Some(tn);
                                }
                            }
                        }
                    }
                }
            }
            if let Some(field_type) = self.infer_expression_type(object) {
                if let Some(name) = Self::type_to_name(&field_type) {
                    return Some(name);
                }
            }
        }
        self.infer_type_name(object)
    }

    pub(in crate::codegen::rust) fn mc_resolve_method_call_signature(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> Option<FunctionSignature> {
        let type_name = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object))
            .or_else(|| {
                if let Expression::Identifier { name, .. } = object {
                    if (name == "Self" || name == "self") && self.in_impl_block {
                        return self.current_struct_name.clone();
                    }
                }
                None
            });

        if let Some(ref tn) = type_name {
            return self.lookup_method_signature_on_receiver_type(tn, method, arguments.len());
        }

        // Never homonym-guess `Type::method` when the receiver is a field access — e.g.
        // `self.quest_manager.is_quest_active` must not resolve to `DialogueState::is_quest_active`.
        if matches!(object, Expression::FieldAccess { .. }) {
            return None;
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
