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
const ENUMERABLE: &str = include_str!("../../std/enumerable.wj");

/// One declared method: the generics of its type and of the method, the parameter types (no receiver), the result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Entry {
    type_generics: Vec<String>,
    method_generics: Vec<String>,
    params: Vec<Type>,
    ret: Option<Type>,
    /// Declared without `self`: an associated function (`Vec::new()`).
    is_static: bool,
    /// Declared `&mut self`: the method changes its receiver (`push`, `insert`, `next`).
    changes_receiver: bool,
}

/// An instantiated signature. A type the receiver or the turbofish does not fix stays `Type::Generic`.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    /// Parameter types, without the receiver.
    pub params: Vec<Type>,
    pub ret: Option<Type>,
}

/// The parsed surface, keyed by (type, method).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "SurfaceRepr", into = "SurfaceRepr")]
pub struct CoreSurface {
    table: BTreeMap<(String, String), Entry>,
    /// The enumerable methods every sequence has (`std/enumerable.wj`, trait `Enumerable<T>`), by method name.
    enumerable: BTreeMap<String, Entry>,
}

/// The stored form of a [`CoreSurface`]: JSON keys are strings, so the (type, method) table is a list of pairs.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SurfaceRepr {
    table: Vec<((String, String), Entry)>,
    enumerable: BTreeMap<String, Entry>,
}

impl From<CoreSurface> for SurfaceRepr {
    fn from(s: CoreSurface) -> Self {
        SurfaceRepr { table: s.table.into_iter().collect(), enumerable: s.enumerable }
    }
}

impl From<SurfaceRepr> for CoreSurface {
    fn from(r: SurfaceRepr) -> Self {
        CoreSurface { table: r.table.into_iter().collect(), enumerable: r.enumerable }
    }
}

impl CoreSurface {
    /// Parse `std/rust_core.wji` (embedded in the compiler).
    pub fn parse() -> Self {
        let mut table = BTreeMap::new();
        collect_entries(SURFACE, "std/rust_core.wji", &mut table);
        let mut enumerable_table = BTreeMap::new();
        collect_entries(ENUMERABLE, "std/enumerable.wj", &mut enumerable_table);
        let enumerable = enumerable_table.into_iter().map(|((_, m), e)| (m, e)).collect();
        Self { table, enumerable }
    }

    /// The item type of the iterator a method is declared to return, when every type that declares the method
    /// agrees and the item names no type parameter (`split_whitespace` -> `&str`; `map` -> none: its item is `U`).
    /// For a receiver whose type is not known: the declaration still says what the call yields.
    pub fn unique_iterator_item(&self, method: &str) -> Option<Type> {
        let mut found: Option<Type> = None;
        for ((_, m), e) in &self.table {
            if m != method || e.is_static {
                continue;
            }
            let ret = e.ret.as_ref()?;
            let (name, args) = head(ret)?;
            if name != ITERATOR || args.len() != 1 || mentions_type_parameter(&args[0]) {
                return None;
            }
            match &found {
                Some(prev) if *prev != args[0] => return None,
                _ => found = Some(args[0].clone()),
            }
        }
        found
    }

    /// Every declared `(type, method)` pair, sorted (for the rustc agreement gate and the docs).
    pub fn entries(&self) -> Vec<(String, String)> {
        self.table.keys().cloned().collect()
    }
}

/// The trait methods declared in a surface file, keyed by (trait, method).
fn collect_entries(source: &str, path: &str, table: &mut BTreeMap<(String, String), Entry>) {
    let tokens = crate::lexer::Lexer::new(source).tokenize_with_locations();
    let program = crate::parser_impl::Parser::new(tokens).parse().unwrap_or_else(|e| panic!("{path} parses: {e}"));
    for item in &program.items {
        if let Item::Trait { decl, .. } = item {
            for m in &decl.methods {
                let entry = Entry {
                    type_generics: decl.generics.clone(),
                    method_generics: m.type_params.iter().map(|t| t.name.clone()).collect(),
                    params: m.parameters.iter().filter(|p| p.name != "self").map(|p| p.type_.clone()).collect(),
                    ret: m.return_type.clone(),
                    is_static: !m.parameters.iter().any(|p| p.name == "self"),
                    changes_receiver: m
                        .parameters
                        .iter()
                        .any(|p| p.name == "self" && p.ownership == crate::parser::ast::ownership::OwnershipHint::Mut),
                };
                table.insert((decl.name.clone(), m.name.clone()), entry);
            }
        }
    }
}

impl CoreSurface {
    /// The declared signature of `type_name::method`, uninstantiated: (type generics, method generics, params, ret).
    #[allow(clippy::type_complexity)]
    pub fn declared(&self, type_name: &str, method: &str) -> Option<(Vec<String>, Vec<String>, Vec<Type>, Option<Type>)> {
        // A sequence also has the enumerable methods (`unique`, `sum_by`, ...), declared on its element `T`.
        let e = self
            .table
            .get(&(type_name.to_string(), method.to_string()))
            .or_else(|| is_element_container(type_name).then(|| self.enumerable.get(method)).flatten())
            // Direct iteration: `v.skip(n)` on a sequence is `v.iter().skip(n)` (ADR-0014), so a sequence also has the
            // iterator adapters, over the same element `T` (`skip` / `take` counts are `usize` slots).
            .or_else(|| {
                (is_element_container(type_name) && type_name != ITERATOR)
                    .then(|| self.table.get(&(ITERATOR.to_string(), method.to_string())))
                    .flatten()
            })?;
        if e.is_static {
            return None;
        }
        Some((e.type_generics.clone(), e.method_generics.clone(), e.params.clone(), e.ret.clone()))
    }

    /// The declared signature of an associated function `type_name::function` (no `self`), uninstantiated.
    #[allow(clippy::type_complexity)]
    pub fn declared_static(&self, type_name: &str, function: &str) -> Option<(Vec<String>, Vec<String>, Vec<Type>, Option<Type>)> {
        let e = self.table.get(&(type_name.to_string(), function.to_string())).filter(|e| e.is_static)?;
        Some((e.type_generics.clone(), e.method_generics.clone(), e.params.clone(), e.ret.clone()))
    }

    /// Whether `receiver.method(..)` changes its receiver: `Some(true)` for a method declared `&mut self`,
    /// `Some(false)` for any other declared method, `None` when the surface does not declare it for this receiver.
    /// ADR-0002: a local binding changed this way must be declared `let mut`.
    pub fn changes_receiver(&self, receiver: &Type, method: &str) -> Option<bool> {
        let (name, _) = head(receiver)?;
        let e = self
            .table
            .get(&(name.to_string(), method.to_string()))
            .or_else(|| is_element_container(name).then(|| self.enumerable.get(method)).flatten())?;
        (!e.is_static).then_some(e.changes_receiver)
    }

    /// Whether `receiver.method(..)` is an iterator method called directly on a sequence: it iterates the sequence
    /// by reference (`v.any(f)` is `v.iter().any(f)`), so the sequence itself is read, not changed, whatever the
    /// iterator method's own `&mut self` says.
    pub fn iterates_by_reference(&self, receiver: &Type, method: &str) -> bool {
        let Some((name, _)) = head(receiver) else { return false };
        name != ITERATOR && !self.table.contains_key(&(name.to_string(), method.to_string())) && !self.enumerable.contains_key(method)
            && self.table.contains_key(&(ITERATOR.to_string(), method.to_string()))
    }

    /// Whether `receiver.method(..)` on a collection is an iterator adapter: the collection does not declare the
    /// method, `Iterator` does (`names.any(..)`, `names.position(..)`); codegen's collection detection asks by name.
    pub fn is_adapter_on_collection(&self, receiver_name: &str, method: &str) -> bool {
        let name = crate::type_classification::type_name_leaf(receiver_name);
        is_element_container(name)
            && name != ITERATOR
            && !self.table.contains_key(&(name.to_string(), method.to_string()))
            && !self.enumerable.contains_key(method)
            && self.table.contains_key(&(ITERATOR.to_string(), method.to_string()))
    }

    /// [`Self::is_adapter_on_collection`] for a receiver given as a type.
    pub fn is_adapter_on_type(&self, receiver: &Type, method: &str) -> bool {
        surface_type_name(receiver).is_some_and(|name| self.is_adapter_on_collection(name, method))
    }

    /// Whether `type_name::name` is declared without `self` (an associated function).
    pub fn is_static(&self, type_name: &str, name: &str) -> bool {
        self.table.get(&(type_name.to_string(), name.to_string())).is_some_and(|e| e.is_static)
    }

    /// The signature of `receiver.method::<turbofish>(..)`, instantiated, or `None` when the receiver is not a
    /// surface type or the method is not declared for it. Generics left unfixed come back as `Type::Generic`.
    pub fn method_signature(&self, receiver: &Type, method: &str, turbofish: &[Type]) -> Option<MethodSig> {
        let (name, type_args) = head(receiver)?;
        let (type_generics, method_generics, params, ret) = self.declared(name, method)?;
        // An iterator adapter called directly on a sequence iterates it by reference (`v.filter(f)` is
        // `v.iter().filter(f)`): its element is `&T`.
        let by_ref = name != ITERATOR && !self.table.contains_key(&(name.to_string(), method.to_string())) && !self.enumerable.contains_key(method)
            && self.table.contains_key(&(ITERATOR.to_string(), method.to_string()));
        let type_args = if by_ref { type_args.into_iter().map(|t| Type::Reference(Box::new(t))).collect() } else { type_args };
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

/// The surface type a receiver type names (`&Vec<int>` -> `Vec`), or `None` for a type the surface does not declare.
pub fn surface_type_name(receiver: &Type) -> Option<&'static str> {
    head(receiver).map(|(n, _)| n)
}

/// The surface type whose methods are the iterator adapters (`map`, `filter`, ..).
pub const ITERATOR: &str = "Iterator";

/// The surface types whose one type parameter is an element (`Vec<T>`, `Iterator<T>`, the sets).
const ELEMENT_CONTAINERS: [&str; 6] = ["Vec", "VecDeque", "HashSet", "BTreeSet", "Iterator", "List"];

/// Whether a surface type name's type parameter is an element (the solver's `Part::Elem`).
pub fn is_element_container(name: &str) -> bool {
    ELEMENT_CONTAINERS.contains(&crate::type_classification::type_name_leaf(name))
}

/// The surface types whose one type parameter is the value they hand back (`JoinHandle<T>`: `join` returns it); the
/// solver relates it as an element, but they have no sequence methods.
const HANDLE_TYPES: [&str; 4] = ["JoinHandle", "Task", "Sender", "Receiver"];

/// Whether a surface type name holds one value of its type parameter (the solver's `Part::Elem`).
pub fn has_element_part(name: &str) -> bool {
    is_element_container(name) || HANDLE_TYPES.contains(&crate::type_classification::type_name_leaf(name))
}

/// The surface types whose two type parameters are a key and a value (the solver's `Part::Key` / `Part::Value`).
const MAP_CONTAINERS: [&str; 2] = ["HashMap", "BTreeMap"];

/// Whether a surface type name is a map (`HashMap<K, V>`, `BTreeMap<K, V>`).
pub fn is_map_container(name: &str) -> bool {
    MAP_CONTAINERS.contains(&crate::type_classification::type_name_leaf(name))
}

/// The receiver type of a surface type's methods, in the parser's own forms (`Vec<T>`, `Option<T>`, `Result<T, E>`).
pub fn self_type(type_name: &str, generics: &[String]) -> Type {
    let g: Vec<Type> = generics.iter().map(|n| Type::Generic(n.clone())).collect();
    match (type_name, g.as_slice()) {
        ("Vec", [t]) => Type::Vec(Box::new(t.clone())),
        ("Option", [t]) => Type::Option(Box::new(t.clone())),
        ("Result", [t, e]) => Type::Result(Box::new(t.clone()), Box::new(e.clone())),
        _ => Type::Parameterized(type_name.to_string(), g),
    }
}

/// The surface type a receiver type names, with its type arguments (`&Vec<int>` -> `Vec`, `[int]`).
fn head(receiver: &Type) -> Option<(&'static str, Vec<Type>)> {
    const NAMES: [&str; 19] = [
        "Vec", "VecDeque", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Option", "Result", "String", "Iterator", "List", "JoinHandle", "Task",
        "Sender", "Receiver", "thread", "sync", "Duration", "Instant",
    ];
    // Windjammer's `List<T>` (the enumerable methods' result) is the surface's `Vec<T>`; the `thread` module's
    // functions are declared on the surface trait `Thread` (`thread` is a keyword, so it cannot name a trait).
    const SYNONYMS: [(&str, &str); 3] = [("List", "Vec"), ("thread", "Thread"), ("sync", "Sync")];
    let known = |n: &str| {
        let k = NAMES.iter().copied().find(|k| *k == crate::type_classification::type_name_leaf(n))?;
        Some(SYNONYMS.iter().find(|(s, _)| *s == k).map_or(k, |(_, v)| *v))
    };
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
        // Rust's own `std::sync::mpsc` ends are not the ADR-0004 channel ends (their `send` returns a `Result`)
        Type::Parameterized(n, _) if crate::type_classification::is_rust_channel_path(n) => None,
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
        Type::FunctionPointer { params, return_type } => Type::FunctionPointer {
            params: params.iter().map(|x| substitute(x, names, args)).collect(),
            return_type: return_type.as_ref().map(|r| sub(r)),
        },
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

/// Whether a surface type mentions one of its own type parameters (a single capital letter, or `Generic`).
fn mentions_type_parameter(t: &Type) -> bool {
    match t {
        Type::Generic(_) => true,
        Type::Custom(n) => n.len() == 1 && n.chars().all(|c| c.is_ascii_uppercase()),
        Type::Reference(i) | Type::MutableReference(i) | Type::Vec(i) | Type::Option(i) | Type::Array(i, _) => mentions_type_parameter(i),
        Type::Result(a, b) => mentions_type_parameter(a) || mentions_type_parameter(b),
        Type::Tuple(v) => v.iter().any(mentions_type_parameter),
        Type::Parameterized(_, v) => v.iter().any(mentions_type_parameter),
        _ => false,
    }
}
