//! Parameter types of an iterator adapter's closure, from the adapter's declaration in `std/rust_core.wji`.
//!
//! `v.map(|x| ..)` on a list becomes `v.iter().map(..)`: `Vec::iter` is declared `-> Iterator<&T>`, so the
//! iterator's item is `&T`, and `Iterator::map` declares its closure `fn(T) -> U` over that item: `x: &T`.
//! `Iterator::filter` and `find` declare `fn(&T) -> bool`, one reference more: `n: &&T`. An explicit chain
//! (`v.iter().filter(..).find(..)`) is typed call by call through the declared results. The depth is recorded
//! so that the operators in the closure's body dereference the parameter down to its value (`**n % 2`).

use crate::ir::stdlib_tables::{surface_type_name, CoreSurface, ITERATOR};
use crate::parser::{Expression, Type};

use crate::codegen::rust::CodeGenerator;

const COLLECTION: &str = "Vec";
/// The borrowing iteration codegen inserts before an adapter on a collection (`v.map(f)` is `v.iter().map(f)`).
const INSERTED_ITERATION: &str = "iter";

/// What `bind_element_param_types` replaced, for `release_element_param_types`.
pub(in crate::codegen::rust) struct BoundParams {
    types: Vec<(String, Option<Type>)>,
    depths: Vec<(String, Option<usize>)>,
}

fn is_iterator(t: &Type) -> bool {
    surface_type_name(t).is_some_and(|head| head.eq(ITERATOR))
}

/// A type with no generic left open (`Iterator<U>` after `map` is open: the closure's result decides `U`).
fn is_closed(t: &Type) -> bool {
    match t {
        Type::Generic(_) | Type::Infer => false,
        Type::Reference(x) | Type::MutableReference(x) | Type::Option(x) | Type::Vec(x) | Type::Array(x, _) => is_closed(x),
        Type::Result(a, b) => is_closed(a) && is_closed(b),
        Type::Parameterized(_, xs) | Type::Tuple(xs) => xs.iter().all(is_closed),
        _ => true,
    }
}

/// How many shared references wrap a type (`&&i64`: 2), and the type under them.
pub(in crate::codegen::rust) fn reference_depth(t: &Type) -> (usize, &Type) {
    match t {
        Type::Reference(inner) => {
            let (d, base) = reference_depth(inner);
            (d + 1, base)
        }
        other => (0, other),
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// True when `receiver.method(|x| ..)` gets `.iter()` inserted (a list receiver, an iterator adapter).
    fn adapter_gets_inserted_iter(&self, object: &Expression<'ast>, method: &str, receiver: &Option<Type>, surface: &CoreSurface) -> bool {
        let iterates_by_reference = Self::is_collection_literal_expr(object) || receiver.as_ref().is_some_and(Self::type_is_iterable_collection);
        // `xs.into_iter().map(..)` / `xs.iter().map(..)`: the receiver already is an iterator, nothing is inserted
        let already_iterator = matches!(object, Expression::MethodCall { method: producer, .. }
            if surface.declared(COLLECTION, producer).and_then(|(_, _, _, ret)| ret).is_some_and(|ret| is_iterator(&ret)));
        iterates_by_reference && !already_iterator && Self::is_direct_iterator_adapter(method)
    }

    /// The declared iterator over a collection that the inserted borrowing iteration gives (`Iterator<&T>`).
    fn inserted_iterator_type(collection: &Type, surface: &CoreSurface) -> Option<Type> {
        surface.method_signature(collection, INSERTED_ITERATION, &[])?.ret.filter(|t| is_iterator(t) && is_closed(t))
    }

    /// The `Iterator<Item>` type an expression evaluates to, typed from the declarations call by call.
    fn iterator_type_of(&self, expr: &Expression<'ast>, surface: &CoreSurface) -> Option<Type> {
        let Expression::MethodCall { object, method, type_args, .. } = expr else { return None };
        let turbofish = type_args.clone().unwrap_or_default();
        let recv = match self.iterator_type_of(object, surface) {
            Some(it) => it,
            None => {
                let collection = self.infer_expression_type(object)?;
                if let Some(ret) = surface.method_signature(&collection, method, &turbofish).and_then(|s| s.ret) {
                    if is_iterator(&ret) {
                        return is_closed(&ret).then_some(ret);
                    }
                }
                if !self.adapter_gets_inserted_iter(object, method, &Some(collection.clone()), surface) {
                    return None;
                }
                Self::inserted_iterator_type(&collection, surface)?
            }
        };
        let ret = surface.method_signature(&recv, method, &turbofish)?.ret?;
        (is_iterator(&ret) && is_closed(&ret)).then_some(ret)
    }

    /// The declared parameter types of the closure passed as argument `arg_index` of `object.method(..)`, when the
    /// call is an iterator adapter whose item type is known.
    fn adapter_closure_param_types(&self, object: &Expression<'ast>, method: &str, arg_index: usize, receiver: &Option<Type>) -> Option<Vec<Type>> {
        let surface = CoreSurface::parse();
        // A receiver the declarations type as an iterator (`s.chars()`, `v.iter()`) gets nothing inserted.
        let iterator = match self.iterator_type_of(object, &surface) {
            Some(it) => it,
            None if self.adapter_gets_inserted_iter(object, method, receiver, &surface) => {
                Self::inserted_iterator_type(receiver.as_ref()?, &surface)?
            }
            None => return None,
        };
        let sig = surface.method_signature(&iterator, method, &[])?;
        match sig.params.get(arg_index)? {
            Type::FunctionPointer { params, .. } if params.iter().all(is_closed) => Some(params.clone()),
            _ => None,
        }
    }

    /// True when `receiver.method(|x| ..)` gets `.iter()` inserted and the adapter's declared closure takes the element only.
    pub(super) fn closure_receives_element_ref(&self, object: &Expression<'ast>, method: &str, receiver: &Option<Type>) -> bool {
        let surface = CoreSurface::parse();
        if !self.adapter_gets_inserted_iter(object, method, receiver, &surface) {
            return false;
        }
        surface.declared(ITERATOR, method).is_some_and(|(type_generics, _, params, _)| {
            matches!(params.as_slice(), [Type::FunctionPointer { params: closure, .. }]
                if matches!(closure.as_slice(), [Type::Custom(t) | Type::Generic(t)] if type_generics.first() == Some(t)))
        })
    }

    /// The closure's parameters take the declared types of the adapter's closure: record them (`x: &T`,
    /// `n: &&T`) so field reads have a type and operators dereference to the value.
    pub(super) fn bind_element_param_types(&mut self, object: &Expression<'ast>, method: &str, arg_index: usize, arg: &Expression<'ast>, receiver: &Option<Type>) -> BoundParams {
        let mut bound = BoundParams { types: Vec::new(), depths: Vec::new() };
        let Expression::Closure { parameters, .. } = arg else { return bound };
        let Some(declared) = self.adapter_closure_param_types(object, method, arg_index, receiver) else { return bound };
        if declared.len() != parameters.len() {
            return bound;
        }
        for (p, ty) in parameters.iter().zip(declared) {
            // A written parameter type (`|x: int|`) is the user's; only untyped parameters take the declaration.
            if p.contains(": ") {
                continue;
            }
            let name = Self::closure_param_binding_name(p);
            let (depth, _) = reference_depth(&ty);
            let before_depth = self.idiom.adapter_closure_param_depth.insert(name.clone(), depth);
            bound.depths.push((name.clone(), before_depth));
            let before = self.local_var_types.insert(name.clone(), ty);
            bound.types.push((name, before));
        }
        bound
    }

    pub(super) fn release_element_param_types(&mut self, bound: BoundParams) {
        for (name, before) in bound.types.into_iter().rev() {
            match before {
                Some(t) => self.local_var_types.insert(name, t),
                None => self.local_var_types.remove(&name),
            };
        }
        for (name, before) in bound.depths.into_iter().rev() {
            match before {
                Some(d) => self.idiom.adapter_closure_param_depth.insert(name, d),
                None => self.idiom.adapter_closure_param_depth.remove(&name),
            };
        }
    }
}
