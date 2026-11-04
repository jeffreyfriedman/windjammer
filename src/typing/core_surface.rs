//! The typed surface of the Rust core types (`std/rust_core.wji`): method signatures of `Vec`, `VecDeque`,
//! `HashMap`, `BTreeMap`, `HashSet`, `BTreeSet`, `Option`, `Result`, `String` and the `f32` / `f64` math
//! methods, in Windjammer types.
//!
//! Option C, stage C1 (docs/design/numeric-width-redesign.md section 7): the surface is data, parsed with the
//! Windjammer parser into a [`CoreSurface`] that a compilation session owns (no global state), and
//! [`CoreSurface::method_signature`] instantiates an entry for a receiver's static type (the type's generics from
//! the receiver, a method's own generics from a turbofish). The numeric solver reads it for std method arguments
//! and results instead of matching method names.

use crate::parser::ast::core::Item;
use crate::parser::Type;
use std::collections::BTreeMap;

const SURFACE: &str = include_str!("../../std/rust_core.wji");

/// One declared method: the generics of its type and of the method, the parameter types (no receiver), the result.
#[derive(Debug, Clone)]
struct Entry {
    type_generics: Vec<String>,
    method_generics: Vec<String>,
    params: Vec<Type>,
    ret: Option<Type>,
}

/// An instantiated signature. A type the receiver or the turbofish does not fix stays `Type::Generic`.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    /// Parameter types, without the receiver.
    pub params: Vec<Type>,
    pub ret: Option<Type>,
}

/// The parsed surface, keyed by (type, method).
#[derive(Debug, Clone)]
pub struct CoreSurface {
    table: BTreeMap<(String, String), Entry>,
}

impl CoreSurface {
    /// Parse `std/rust_core.wji` (embedded in the compiler).
    pub fn parse() -> Self {
        let tokens = crate::lexer::Lexer::new(SURFACE).tokenize_with_locations();
        let program = crate::parser_impl::Parser::new(tokens).parse().expect("std/rust_core.wji parses");
        let mut table = BTreeMap::new();
        for item in &program.items {
            if let Item::Trait { decl, .. } = item {
                for m in &decl.methods {
                    let entry = Entry {
                        type_generics: decl.generics.clone(),
                        method_generics: m.type_params.iter().map(|t| t.name.clone()).collect(),
                        params: m.parameters.iter().filter(|p| p.name != "self").map(|p| p.type_.clone()).collect(),
                        ret: m.return_type.clone(),
                    };
                    table.insert((decl.name.clone(), m.name.clone()), entry);
                }
            }
        }
        Self { table }
    }

    /// Every declared `(type, method)` pair, sorted (for the rustc agreement gate and the docs).
    pub fn entries(&self) -> Vec<(String, String)> {
        self.table.keys().cloned().collect()
    }

    /// The declared signature of `type_name::method`, uninstantiated: (type generics, method generics, params, ret).
    #[allow(clippy::type_complexity)]
    pub fn declared(&self, type_name: &str, method: &str) -> Option<(Vec<String>, Vec<String>, Vec<Type>, Option<Type>)> {
        let e = self.table.get(&(type_name.to_string(), method.to_string()))?;
        Some((e.type_generics.clone(), e.method_generics.clone(), e.params.clone(), e.ret.clone()))
    }

    /// The signature of `receiver.method::<turbofish>(..)`, instantiated, or `None` when the receiver is not a
    /// surface type or the method is not declared for it. Generics left unfixed come back as `Type::Generic`.
    pub fn method_signature(&self, receiver: &Type, method: &str, turbofish: &[Type]) -> Option<MethodSig> {
        let (name, type_args) = head(receiver)?;
        let (type_generics, method_generics, params, ret) = self.declared(name, method)?;
        let mut names = type_generics;
        let mut args = type_args;
        args.resize(names.len(), Type::Infer);
        for (i, g) in method_generics.iter().enumerate() {
            names.push(g.clone());
            args.push(turbofish.get(i).cloned().unwrap_or(Type::Infer));
        }
        // An unfixed generic (no receiver argument, no turbofish) stays generic.
        let args: Vec<Type> = args.into_iter().zip(&names).map(|(a, n)| if a == Type::Infer { Type::Generic(n.clone()) } else { a }).collect();
        Some(MethodSig {
            params: params.iter().map(|p| substitute(p, &names, &args)).collect(),
            ret: ret.map(|r| substitute(&r, &names, &args)),
        })
    }
}

/// The surface type a receiver type names, with its type arguments (`&Vec<int>` -> `Vec`, `[int]`).
fn head(receiver: &Type) -> Option<(&'static str, Vec<Type>)> {
    const NAMES: [&str; 9] = ["Vec", "VecDeque", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Option", "Result", "String"];
    let known = |n: &str| NAMES.iter().copied().find(|k| *k == crate::type_classification::type_name_leaf(n));
    match receiver {
        Type::Reference(t) | Type::MutableReference(t) => head(t),
        Type::Vec(t) | Type::Array(t, _) => Some(("Vec", vec![(**t).clone()])),
        Type::Option(t) => Some(("Option", vec![(**t).clone()])),
        Type::Result(t, e) => Some(("Result", vec![(**t).clone(), (**e).clone()])),
        Type::String => Some(("String", Vec::new())),
        Type::Custom(n) if matches!(n.as_str(), "String" | "string" | "str") => Some(("String", Vec::new())),
        Type::Float => Some(("Float", vec![receiver.clone()])),
        Type::Custom(n) if matches!(n.as_str(), "f64" | "f32" | "float") => Some(("Float", vec![receiver.clone()])),
        Type::Int | Type::Int32 | Type::Uint => Some(("Integer", vec![receiver.clone()])),
        Type::Custom(n) if is_integer_name(n) => Some(("Integer", vec![receiver.clone()])),
        Type::Parameterized(n, args) => known(n).map(|k| (k, args.clone())),
        _ => None,
    }
}

fn is_integer_name(n: &str) -> bool {
    matches!(n, "int" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize")
}

/// Whether a numeric method's parameter or result is the receiver's own type (`T` of `Integer<T>` / `Float<T>`).
pub fn is_receiver_type(t: &Type) -> bool {
    matches!(t, Type::Custom(n) | Type::Generic(n) if n == "T")
}

impl CoreSurface {
    /// A method of the integer (`float == false`) or float numeric surface, uninstantiated: (params, result), where
    /// a type `T` is the receiver's own type. Used when the receiver's width is a class (a literal), not a type.
    pub fn numeric_method(&self, float: bool, method: &str) -> Option<(Vec<Type>, Option<Type>)> {
        let (_, _, params, ret) = self.declared(if float { "Float" } else { "Integer" }, method)?;
        Some((params, ret))
    }
}

fn substitute(t: &Type, names: &[String], args: &[Type]) -> Type {
    let lookup = |n: &str| names.iter().position(|g| g == n).and_then(|i| args.get(i).cloned());
    let sub = |x: &Type| Box::new(substitute(x, names, args));
    match t {
        Type::Custom(n) | Type::Generic(n) => lookup(n).unwrap_or_else(|| t.clone()),
        Type::Reference(x) => Type::Reference(sub(x)),
        Type::MutableReference(x) => Type::MutableReference(sub(x)),
        Type::Option(x) => Type::Option(sub(x)),
        Type::Vec(x) => Type::Vec(sub(x)),
        Type::Result(a, b) => Type::Result(sub(a), sub(b)),
        Type::Parameterized(n, xs) => Type::Parameterized(n.clone(), xs.iter().map(|x| substitute(x, names, args)).collect()),
        Type::Tuple(xs) => Type::Tuple(xs.iter().map(|x| substitute(x, names, args)).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method_signature(receiver: &Type, method: &str, turbofish: &[Type]) -> Option<MethodSig> {
        CoreSurface::parse().method_signature(receiver, method, turbofish)
    }

    fn entries() -> Vec<(String, String)> {
        CoreSurface::parse().entries()
    }

    fn vec_of(t: Type) -> Type {
        Type::Vec(Box::new(t))
    }

    #[test]
    fn surface_parses_and_declares_the_core_types() {
        let e = entries();
        assert!(e.len() > 80, "{}", e.len());
        for t in ["Vec", "VecDeque", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Option", "Result", "String", "Integer", "Float"] {
            assert!(e.iter().any(|(n, _)| n == t), "{t} missing");
        }
    }

    #[test]
    fn vec_methods_take_and_give_their_element_and_usize_indices() {
        let s = method_signature(&vec_of(Type::Int), "get", &[]).unwrap();
        assert_eq!(s.params, vec![Type::Custom("usize".into())]);
        assert_eq!(s.ret, Some(Type::Option(Box::new(Type::Reference(Box::new(Type::Int))))));
        let push = method_signature(&Type::Reference(Box::new(vec_of(Type::Int32))), "push", &[]).unwrap();
        assert_eq!(push.params, vec![Type::Int32]);
        assert_eq!(method_signature(&vec_of(Type::Int), "len", &[]).unwrap().ret, Some(Type::Custom("usize".into())));
    }

    #[test]
    fn map_get_then_unwrap_or_is_the_value_type() {
        let map = Type::Parameterized("HashMap".into(), vec![Type::String, Type::Int]);
        let got = method_signature(&map, "get", &[]).unwrap().ret.unwrap();
        let unwrap_or = method_signature(&got, "unwrap_or", &[]).unwrap();
        assert_eq!(unwrap_or.params, vec![Type::Reference(Box::new(Type::Int))]);
    }

    #[test]
    fn parse_takes_its_result_from_the_turbofish() {
        let parsed = method_signature(&Type::String, "parse", &[Type::Int]).unwrap().ret.unwrap();
        let s = method_signature(&parsed, "unwrap_or", &[]).unwrap();
        assert_eq!(s.params, vec![Type::Int]);
        assert_eq!(s.ret, Some(Type::Int));
        // No turbofish: the result's payload stays generic.
        let open = method_signature(&Type::String, "parse", &[]).unwrap().ret.unwrap();
        assert!(matches!(open, Type::Result(ref t, _) if matches!(**t, Type::Generic(_))), "{open:?}");
    }

    #[test]
    fn float_methods_keep_the_receiver_width() {
        let s = method_signature(&Type::Custom("f32".into()), "min", &[]).unwrap();
        assert_eq!(s.params, vec![Type::Custom("f32".into())]);
        assert_eq!(s.ret, Some(Type::Custom("f32".into())));
        assert_eq!(method_signature(&Type::Float, "sqrt", &[]).unwrap().ret, Some(Type::Float));
        let pow = method_signature(&Type::Int, "pow", &[]).unwrap();
        assert_eq!(pow.params, vec![Type::Custom("u32".into())]);
        assert_eq!(pow.ret, Some(Type::Int));
    }

    #[test]
    fn unknown_receivers_and_methods_have_no_signature() {
        assert!(method_signature(&Type::Custom("Grid".into()), "get", &[]).is_none());
        assert!(method_signature(&vec_of(Type::Int), "frobnicate", &[]).is_none());
    }
}
