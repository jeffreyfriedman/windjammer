//! `pairs.map(|p| p.1)` on a list: the call becomes `pairs.iter().map(..)`, so the closure receives each element by
//! reference. The declaration (`std/rust_core.wji`) says which adapters take the element alone: `fn(T) -> U`.

use crate::parser::{Expression, Type};
use crate::ir::stdlib_tables::{surface_type_name, CoreSurface, ITERATOR};

use crate::codegen::rust::CodeGenerator;

const COLLECTION: &str = "Vec";

impl<'ast> CodeGenerator<'ast> {
    /// True when `receiver.method(|x| ..)` gets `.iter()` inserted and the adapter's declared closure takes the element only.
    pub(super) fn closure_receives_element_ref(&self, object: &Expression<'ast>, method: &str, receiver: &Option<Type>) -> bool {
        let iterates_by_reference = Self::is_collection_literal_expr(object) || receiver.as_ref().is_some_and(Self::type_is_iterable_collection);
        // `xs.into_iter().map(..)` / `xs.iter().map(..)`: the receiver already is an iterator, nothing is inserted
        let already_iterator = matches!(object, Expression::MethodCall { method: producer, .. }
            if CoreSurface::parse().declared(COLLECTION, producer).and_then(|(_, _, _, ret)| ret).is_some_and(|ret| surface_type_name(&ret).is_some_and(|head| head.eq(ITERATOR))));
        if !iterates_by_reference || already_iterator || !Self::is_direct_iterator_adapter(method) {
            return false;
        }
        CoreSurface::parse().declared(ITERATOR, method).is_some_and(|(type_generics, _, params, _)| {
            matches!(params.as_slice(), [Type::FunctionPointer { params: closure, .. }]
                if matches!(closure.as_slice(), [Type::Custom(t) | Type::Generic(t)] if type_generics.first() == Some(t)))
        })
    }

    /// The closure's single parameter takes the list's element by reference: record `&Elem` so `p.1` has a type.
    /// Returns what the names held before, for `release_element_param_types`.
    pub(super) fn bind_element_param_types(&mut self, object: &Expression<'ast>, method: &str, arg: &Expression<'ast>, receiver: &Option<Type>) -> Vec<(String, Option<Type>)> {
        let Expression::Closure { parameters, .. } = arg else { return Vec::new() };
        let Some(elem) = receiver.as_ref().and_then(Self::peeled_collection_element_type).cloned() else { return Vec::new() };
        if parameters.len() != 1 || !self.closure_receives_element_ref(object, method, receiver) {
            return Vec::new();
        }
        let name = Self::closure_param_binding_name(&parameters[0]);
        let before = self.local_var_types.insert(name.clone(), Type::Reference(Box::new(elem)));
        vec![(name, before)]
    }

    pub(super) fn release_element_param_types(&mut self, bound: Vec<(String, Option<Type>)>) {
        for (name, before) in bound {
            match before {
                Some(t) => self.local_var_types.insert(name, t),
                None => self.local_var_types.remove(&name),
            };
        }
    }
}
