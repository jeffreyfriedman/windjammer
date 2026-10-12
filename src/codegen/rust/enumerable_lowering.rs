//! Rust lowering of the enumerable collection surface (ADR-0018 feature 1).
//!
//! The surface table (`crate::stdlib_surface::enumerable`) names the runtime method; this module only renames the
//! call and inlines the std-only runtime source (`crates/windjammer-runtime/src/enumerable.rs`) as
//! `mod wj_enumerable`, so generated crates need no dependency. Ownership of the arguments is decided upstream
//! (the closure is an ordinary closure); nothing here inspects argument types.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Type};
use crate::stdlib_surface::enumerable::{self, Ret};

const RUNTIME_SOURCE: &str = include_str!("../../../crates/windjammer-runtime/src/enumerable.rs");

/// `mod wj_enumerable { .. }`: the runtime source without its unit tests.
pub(super) fn inline_module() -> String {
    let src = RUNTIME_SOURCE
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(RUNTIME_SOURCE);
    format!(
        "#[allow(dead_code, clippy::all)]\nmod {} {{\n{}}}\n",
        enumerable::RUST_MODULE,
        src
    )
}

fn element_type(recv: &Type) -> Option<Type> {
    match recv {
        Type::Vec(t) | Type::Array(t, _) => Some((**t).clone()),
        Type::Reference(t) | Type::MutableReference(t) => element_type(t),
        Type::Parameterized(n, args) if matches!(n.as_str(), "Vec" | "HashSet" | "BTreeSet" | "VecDeque") => args.first().cloned(),
        _ => None,
    }
}

fn map_of(k: Type, v: Type) -> Type {
    Type::Parameterized("HashMap".to_string(), vec![k, v])
}

/// Result type of a surface method on a receiver of type `recv` (`None` when it depends on a closure's result
/// type, which the solver/rustc infer).
pub(super) fn surface_return_type(m: &enumerable::Method, recv: &Type) -> Option<Type> {
    let t = element_type(recv)?;
    let list = Type::Vec(Box::new(t.clone()));
    let int = Type::Custom("i64".to_string());
    Some(match m.ret {
        Ret::GroupMap => map_of(Type::Infer, list),
        Ret::CountMap => map_of(Type::Infer, int),
        Ret::TallyMap => map_of(t, int),
        Ret::List => list,
        Ret::ListOfLists => Type::Vec(Box::new(list)),
        Ret::ListPair => Type::Tuple(vec![list.clone(), list]),
        Ret::IndexOpt => Type::Option(Box::new(Type::Custom("usize".to_string()))),
        Ret::Unit => Type::Tuple(vec![]),
        Ret::ClosureResult | Ret::ClosureList => return None,
    })
}

impl<'ast> CodeGenerator<'ast> {
    /// The Rust method name for `recv.method(args)`: the runtime name when `method` is a surface method the
    /// receiver's own type does not define.
    pub(in crate::codegen::rust) fn enumerable_rust_name(
        &mut self,
        receiver_type: Option<&str>,
        method: &str,
        argc: usize,
    ) -> Option<&'static str> {
        let m = enumerable::lookup(method, argc)?;
        if self.receiver_defines_method(receiver_type, method) {
            return None;
        }
        self.uses_enumerable = true;
        Some(m.rust)
    }

    /// Static type of `object.method(args)` when `method` is a surface method (for map indexing, `len`, ...).
    pub(in crate::codegen::rust) fn enumerable_call_type(
        &self,
        object: &Expression<'ast>,
        method: &str,
        argc: usize,
    ) -> Option<Type> {
        let m = enumerable::lookup(method, argc)?;
        let recv_name = self.infer_type_name(object);
        if self.receiver_defines_method(recv_name.as_deref(), method) {
            return None;
        }
        surface_return_type(m, &self.infer_expression_type(object)?)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn inlined_module_has_no_tests_and_both_traits() {
        let m = super::inline_module();
        assert!(m.starts_with("#[allow(dead_code, clippy::all)]\nmod wj_enumerable {"));
        assert!(m.contains("pub trait WjIterExt") && m.contains("pub trait WjCollExt"));
        assert!(!m.contains("mod tests"));
    }
}
