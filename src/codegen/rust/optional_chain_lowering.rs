//! Optional chaining on the Rust backend (ADR-0018 feature 5, spec area OPT).
//!
//! The parser desugars `a?.b` / `a?.m(args)` to `a.__opt_chain(|_oc| _oc.b)`. For an `Option<T>` receiver this
//! becomes `a.as_ref().map(|_oc| ..)`, or `.and_then(..)` when the member is itself an option (so `a?.b?.c` is
//! one optional). The choice is made from the member's static type: a field of the payload struct, or the return
//! type of a method of the payload type. `as_ref` keeps the receiver usable afterwards; a non-`Copy` field is
//! cloned out of the borrow. Any other receiver (a `Result`, an unknown type) is reported, not guessed.

use super::CodeGenerator;
use crate::parser::optional_chain::OPT_CHAIN_PARAM;
use crate::parser::{Expression, Type};

impl<'ast> CodeGenerator<'ast> {
    /// The `T` of an `Option<T>` receiver (references peeled).
    fn opt_chain_payload(&self, object: &Expression<'ast>) -> Option<Type> {
        let mut ty = self.infer_expression_type(object)?;
        while let Type::Reference(inner) | Type::MutableReference(inner) = ty {
            ty = *inner;
        }
        match ty {
            Type::Option(payload) => Some(*payload),
            _ => None,
        }
    }

    /// Static type of the member the closure body reads from a payload of type `payload`.
    fn opt_chain_member_type(&self, payload: &Type, body: &Expression<'ast>) -> Option<Type> {
        let name = Self::type_to_name(payload)?;
        let base = name.split('<').next().unwrap_or(&name);
        let base = base.rsplit("::").next().unwrap_or(base);
        match body {
            Expression::FieldAccess { field, .. } => self.lookup_struct_field_types(base)?.get(field.as_str()).cloned(),
            Expression::MethodCall { method, .. } => {
                self.get_signature_with_global(&format!("{base}::{method}"))?.return_type.clone()
            }
            _ => None,
        }
    }

    /// Type of `object?.member`: an option of the member, flattened (`T??` is `T?`).
    pub(in crate::codegen::rust) fn opt_chain_type(
        &self,
        object: &Expression<'ast>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> Option<Type> {
        let (_, Expression::Closure { body, .. }) = arguments.first()? else { return None };
        let member = self.opt_chain_member_type(&self.opt_chain_payload(object)?, body)?;
        Some(Type::Option(Box::new(match member {
            Type::Option(inner) => *inner,
            other => other,
        })))
    }

    /// `object?.member` as Rust.
    pub(in crate::codegen::rust) fn lower_optional_chain(
        &mut self,
        object: &Expression<'ast>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> String {
        let Some((_, Expression::Closure { body, .. })) = arguments.first() else { return self.unsupported_optional_chain() };
        let Some(payload) = self.opt_chain_payload(object) else { return self.unsupported_optional_chain() };
        let Some(member) = self.opt_chain_member_type(&payload, body) else { return self.unsupported_optional_chain() };
        let receiver = self.generate_expression(object);
        let previous = self.local_var_types.insert(OPT_CHAIN_PARAM.to_string(), payload);
        let value = self.generate_expression(body);
        match previous {
            Some(t) => self.local_var_types.insert(OPT_CHAIN_PARAM.to_string(), t),
            None => self.local_var_types.remove(OPT_CHAIN_PARAM),
        };
        let value = if matches!(body, Expression::FieldAccess { .. }) && !self.is_type_copy(&member) { format!("{value}.clone()") } else { value };
        let adapter = if matches!(member, Type::Option(_)) { "and_then" } else { "map" };
        format!("{receiver}.as_ref().{adapter}(|{OPT_CHAIN_PARAM}| {value})")
    }
}
