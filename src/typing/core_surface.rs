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
use crate::stdlib_surface::lowering::{LoweringTable, RawDecorator, RawMethod, RawTrait};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

const SURFACE: &str = include_str!("../../std/rust_core.wji");
const ENUMERABLE: &str = include_str!("../../std/enumerable.wj");
const SURFACE_PATH: &str = "std/rust_core.wji";

/// One declared method: the generics of its type and of the method, the parameter types (no receiver), the result.
#[derive(Debug, Clone)]
struct Entry {
    type_generics: Vec<String>,
    method_generics: Vec<String>,
    params: Vec<Type>,
    ret: Option<Type>,
    /// Declared without `self`: an associated function (`Vec::new()`).
    is_static: bool,
    /// Declared `&mut self`: the method changes its receiver (`push`, `insert`, `next`).
    changes_receiver: bool,
    /// The parameter names, without `self` (the names a lowering template's `{name}` refers to).
    param_names: Vec<String>,
}

/// An instantiated signature. A type the receiver or the turbofish does not fix stays `Type::Generic`.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    /// Parameter types, without the receiver.
    pub params: Vec<Type>,
    pub ret: Option<Type>,
}

/// The decorators of one trait: its own and each method's, parsed from the declaration's attribute lines.
#[derive(Debug, Default)]
struct ChunkDecorators {
    of_trait: Vec<RawDecorator>,
    by_method: BTreeMap<String, Vec<RawDecorator>>,
}

/// One `trait` of a surface file. A compilation touches a handful of the traits, so each is read on first use: the text is
/// split by trait when the surface is loaded (a line scan), the typed signatures are parsed with the Windjammer parser from the
/// declaration without its attribute lines, and the attributes are read by a small reader of `@name(key = "text", ..)`.
/// What a process never asks about is never parsed.
#[derive(Debug)]
struct TraitChunk {
    /// The attribute lines directly above `trait Name`.
    trait_lines: Vec<String>,
    /// The `trait Name { .. }` declaration, method attribute lines included.
    text: String,
    entries: OnceLock<BTreeMap<String, Entry>>,
    decorators: OnceLock<ChunkDecorators>,
}

impl TraitChunk {
    fn entries(&self, name: &str) -> &BTreeMap<String, Entry> {
        self.entries.get_or_init(|| {
            let mut bare = String::with_capacity(self.text.len());
            for line in self.text.split_inclusive('\n') {
                if !line.trim_start().starts_with('@') {
                    bare.push_str(line);
                }
            }
            let mut by_trait = BTreeMap::new();
            collect_entries(&bare, SURFACE_PATH, &mut by_trait);
            by_trait.remove(name).unwrap_or_default()
        })
    }

    fn decorators(&self, name: &str) -> &ChunkDecorators {
        self.decorators.get_or_init(|| {
            let mut out = ChunkDecorators { of_trait: self.trait_decorators(name), ..Default::default() };
            let mut pending: Vec<RawDecorator> = Vec::new();
            for line in self.text.lines() {
                let line = line.trim();
                if line.starts_with('@') {
                    pending.push(read_decorator(SURFACE_PATH, name, line));
                } else if let Some(rest) = line.strip_prefix("fn ") {
                    let method: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    out.by_method.insert(method, std::mem::take(&mut pending));
                } else if !line.is_empty() && !line.starts_with("//") && !line.starts_with("pub trait ") && !line.starts_with("trait ") {
                    pending.clear();
                }
            }
            out
        })
    }

    /// The attributes of the trait itself (the lines above it).
    fn trait_decorators(&self, name: &str) -> Vec<RawDecorator> {
        self.trait_lines.iter().map(|l| read_decorator(SURFACE_PATH, name, l)).collect()
    }
}

/// The parsed surface: the traits of `std/rust_core.wji`, each read on first use.
#[derive(Debug)]
struct Inner {
    traits: BTreeMap<String, TraitChunk>,
    /// The enumerable methods every sequence has (`std/enumerable.wj`, trait `Enumerable<T>`), by method name.
    enumerable: OnceLock<BTreeMap<String, Entry>>,
}

/// The parsed surface, keyed by (type, method). Cloning shares the parse.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "SurfaceRepr", into = "SurfaceRepr")]
pub struct CoreSurface {
    inner: Arc<Inner>,
}

/// The stored form of a [`CoreSurface`]: the text of each trait (what is parsed is a function of it).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SurfaceRepr {
    traits: Vec<(String, Vec<String>, String)>,
}

impl From<CoreSurface> for SurfaceRepr {
    fn from(s: CoreSurface) -> Self {
        SurfaceRepr { traits: s.inner.traits.iter().map(|(n, c)| (n.clone(), c.trait_lines.clone(), c.text.clone())).collect() }
    }
}

impl From<SurfaceRepr> for CoreSurface {
    fn from(r: SurfaceRepr) -> Self {
        let traits = r.traits.into_iter().map(|(n, trait_lines, text)| (n, TraitChunk { trait_lines, text, entries: OnceLock::new(), decorators: OnceLock::new() })).collect();
        CoreSurface { inner: Arc::new(Inner { traits, enumerable: OnceLock::new() }) }
    }
}

/// Split a surface file into its traits (a trait starts at a `trait` line in column 0 and ends at the next `}` in column 0;
/// attribute lines in column 0 directly above it belong to it). `examples` keeps the `@example(..)` lines (the gates read them).
fn split_traits(source: &str, examples: bool) -> BTreeMap<String, TraitChunk> {
    let mut traits = BTreeMap::new();
    let mut above: Vec<String> = Vec::new();
    let mut current: Option<(String, Vec<String>, String)> = None;
    for line in source.split_inclusive('\n') {
        if let Some((name, lines, text)) = &mut current {
            if !examples && line.trim_start().starts_with("@example(") {
                continue;
            }
            text.push_str(line);
            if line.starts_with('}') {
                let (name, lines, text) = (std::mem::take(name), std::mem::take(lines), std::mem::take(text));
                traits.insert(name, TraitChunk { trait_lines: lines, text, entries: OnceLock::new(), decorators: OnceLock::new() });
                current = None;
            }
            continue;
        }
        let head = line.strip_prefix("pub trait ").or_else(|| line.strip_prefix("trait "));
        if let Some(rest) = head {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            current = Some((name, std::mem::take(&mut above), line.to_string()));
            if line.starts_with('}') {
                continue;
            }
        } else if line.starts_with('@') {
            above.push(line.trim().to_string());
        } else if !line.trim().is_empty() && !line.starts_with("//") {
            above.clear();
        }
    }
    traits
}

impl CoreSurface {
    /// Parse `std/rust_core.wji` (embedded in the compiler).
    pub fn parse() -> Self {
        Self::from_source(SURFACE, false)
    }

    /// [`Self::parse`] over the text of a `std/rust_core.wji` (the gates parse the file on disk, so a planted
    /// mistake in the file is seen without rebuilding the compiler). The examples are kept.
    pub fn parse_source(surface: &str) -> Self {
        Self::from_source(surface, true)
    }

    fn from_source(surface: &str, examples: bool) -> Self {
        CoreSurface { inner: Arc::new(Inner { traits: split_traits(surface, examples), enumerable: OnceLock::new() }) }
    }

    fn enumerable(&self) -> &BTreeMap<String, Entry> {
        self.inner.enumerable.get_or_init(|| {
            let mut table = BTreeMap::new();
            collect_entries(ENUMERABLE, "std/enumerable.wj", &mut table);
            table.into_iter().flat_map(|(_, methods)| methods).collect()
        })
    }

    /// The declared entry of `type_name::method`.
    fn get(&self, type_name: &str, method: &str) -> Option<&Entry> {
        self.inner.traits.get(type_name)?.entries(type_name).get(method)
    }

    fn has(&self, type_name: &str, method: &str) -> bool {
        self.get(type_name, method).is_some()
    }

    /// The lowering table, every row read (the gates, which check each row; a malformed decorator is an error).
    pub fn lowering(&self) -> LoweringTable {
        LoweringTable::from_declarations(&self.declared_traits()).unwrap_or_else(|e| panic!("std/rust_core.wji lowering decorators: {e}"))
    }

    /// The lowering table a compilation reads: the decorators of each trait (the owners, `@adapts`, `requires`, `widen`) are read
    /// now, the rows of a trait when a call on it is first lowered. A malformed row is a bug in the compiler's own source, so
    /// it fails like a parse error does, when it is first read (the gates read every row through [`Self::lowering`]).
    pub fn lowering_on_demand(&self) -> LoweringTable {
        let names: Vec<(String, Vec<RawDecorator>)> = self.inner.traits.iter().map(|(n, c)| (n.clone(), c.trait_decorators(n))).collect();
        let this = self.clone();
        LoweringTable::on_demand(names, Arc::new(move |name| this.raw_trait(name))).unwrap_or_else(|e| panic!("std/rust_core.wji lowering decorators: {e}"))
    }

    /// One declared trait with its decorators and methods (the input of [`LoweringTable`]).
    pub fn raw_trait(&self, name: &str) -> RawTrait {
        let chunk = &self.inner.traits[name];
        let decorators = chunk.decorators(name);
        let methods = chunk
            .entries(name)
            .iter()
            .map(|(m, e)| RawMethod {
                name: m.clone(),
                params: e.param_names.clone(),
                returns_receiver: e.ret.as_ref().is_some_and(is_receiver_type),
                changes_receiver: e.changes_receiver,
                is_static: e.is_static,
                returns_usize: matches!(&e.ret, Some(Type::Custom(n)) if n == "usize"),
                decorators: decorators.by_method.get(m).cloned().unwrap_or_default(),
            })
            .collect();
        RawTrait { name: name.to_string(), decorators: decorators.of_trait.clone(), methods }
    }

    /// Every declared trait with its decorators and methods, in name order (the input of [`LoweringTable`]).
    pub fn declared_traits(&self) -> Vec<RawTrait> {
        self.inner.traits.keys().filter(|n| !self.inner.traits[*n].entries(n).is_empty()).map(|n| self.raw_trait(n)).collect()
    }

    /// The attributes written on `type_name::method`'s declaration.
    pub fn decorators(&self, type_name: &str, method: &str) -> &[RawDecorator] {
        self.inner.traits.get(type_name).and_then(|c| c.decorators(type_name).by_method.get(method)).map_or(&[], Vec::as_slice)
    }

    /// The item type of the iterator a method is declared to return, when every type that declares the method
    /// agrees and the item names no type parameter (`split_whitespace` -> `&str`; `map` -> none: its item is `U`).
    /// For a receiver whose type is not known: the declaration still says what the call yields.
    pub fn unique_iterator_item(&self, method: &str) -> Option<Type> {
        let mut found: Option<Type> = None;
        for (name, chunk) in &self.inner.traits {
            let Some(e) = chunk.entries(name).get(method) else { continue };
            if e.is_static {
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
        self.inner.traits.iter().flat_map(|(t, c)| c.entries(t).keys().map(move |m| (t.clone(), m.clone()))).collect()
    }
}

/// The trait methods declared in a surface file (attribute lines removed), keyed by trait, then method.
fn collect_entries(source: &str, path: &str, table: &mut BTreeMap<String, BTreeMap<String, Entry>>) {
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
                    param_names: m.parameters.iter().filter(|p| p.name != "self").map(|p| p.name.clone()).collect(),
                };
                table.entry(decl.name.clone()).or_default().insert(m.name.clone(), entry);
            }
        }
    }
}

/// Read an attribute line `@name(key = "text", ..)` (or a bare `@name`). The surface files only use string literals, written
/// as the lexer reads them (`\\`, `\"`, `\n`, `$$`); anything else is a bug in them. A gate compares this reader with the
/// parser over the whole file.
fn read_decorator(path: &str, owner: &str, line: &str) -> RawDecorator {
    let bad = |what: &str| -> ! { panic!("{path}: `{owner}`: attribute `{line}`: {what}") };
    let body = line.strip_prefix('@').unwrap_or_else(|| bad("does not start with `@`"));
    let (name, rest) = match body.find('(') {
        Some(i) => (&body[..i], &body[i + 1..]),
        None => return RawDecorator { name: body.trim().to_string(), args: Vec::new() },
    };
    let mut args = Vec::new();
    let mut chars = rest.chars().peekable();
    loop {
        while matches!(chars.peek(), Some(c) if c.is_whitespace() || *c == ',') {
            chars.next();
        }
        match chars.peek() {
            Some(')') => break,
            None => bad("is not closed"),
            _ => {}
        }
        let mut key = String::new();
        while let Some(c) = chars.next_if(|c| c.is_alphanumeric() || *c == '_') {
            key.push(c);
        }
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        if chars.next() != Some('=') {
            bad("an argument is `key = \"text\"`");
        }
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        if chars.next() != Some('"') {
            bad("an argument must be a string literal");
        }
        let mut value = String::new();
        loop {
            match chars.next() {
                Some('"') => break,
                Some('\\') => value.push(match chars.next() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('0') => '\0',
                    Some(c) => c,
                    None => bad("ends inside a string"),
                }),
                Some('$') if chars.peek() == Some(&'$') => {
                    chars.next();
                    value.push('$');
                }
                Some('$') if chars.peek() == Some(&'{') => bad("`${` interpolates in a string literal"),
                Some(c) => value.push(c),
                None => bad("ends inside a string"),
            }
        }
        args.push((key, value));
    }
    RawDecorator { name: name.trim().to_string(), args }
}

impl CoreSurface {
    /// The declared signature of `type_name::method`, uninstantiated: (type generics, method generics, params, ret).
    #[allow(clippy::type_complexity)]
    pub fn declared(&self, type_name: &str, method: &str) -> Option<(Vec<String>, Vec<String>, Vec<Type>, Option<Type>)> {
        // A sequence also has the enumerable methods (`unique`, `sum_by`, ...), declared on its element `T`.
        let e = self.get(type_name, method)
            .or_else(|| is_element_container(type_name).then(|| self.enumerable().get(method)).flatten())
            // Direct iteration: `v.skip(n)` on a sequence is `v.iter().skip(n)` (ADR-0014), so a sequence also has the
            // iterator adapters, over the same element `T` (`skip` / `take` counts are `usize` slots).
            .or_else(|| {
                (is_element_container(type_name) && type_name != ITERATOR)
                    .then(|| self.get(ITERATOR, method))
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
        let e = self.get(type_name, function).filter(|e| e.is_static)?;
        Some((e.type_generics.clone(), e.method_generics.clone(), e.params.clone(), e.ret.clone()))
    }

    /// Whether `receiver.method(..)` changes its receiver: `Some(true)` for a method declared `&mut self`,
    /// `Some(false)` for any other declared method, `None` when the surface does not declare it for this receiver.
    /// ADR-0002: a local binding changed this way must be declared `let mut`.
    pub fn changes_receiver(&self, receiver: &Type, method: &str) -> Option<bool> {
        let (name, _) = head(receiver)?;
        let e = self.get(name, method)
            .or_else(|| is_element_container(name).then(|| self.enumerable().get(method)).flatten())?;
        (!e.is_static).then_some(e.changes_receiver)
    }

    /// Whether `receiver.method(..)` is an iterator method called directly on a sequence: it iterates the sequence
    /// by reference (`v.any(f)` is `v.iter().any(f)`), so the sequence itself is read, not changed, whatever the
    /// iterator method's own `&mut self` says.
    pub fn iterates_by_reference(&self, receiver: &Type, method: &str) -> bool {
        let Some((name, _)) = head(receiver) else { return false };
        name != ITERATOR && !self.has(name, method) && !self.enumerable().contains_key(method)
            && self.has(ITERATOR, method)
    }

    /// Whether `receiver.method(..)` on a collection is an iterator adapter: the collection does not declare the
    /// method, `Iterator` does (`names.any(..)`, `names.position(..)`); codegen's collection detection asks by name.
    pub fn is_adapter_on_collection(&self, receiver_name: &str, method: &str) -> bool {
        let name = crate::type_classification::type_name_leaf(receiver_name);
        is_element_container(name)
            && name != ITERATOR
            && !self.has(name, method)
            && !self.enumerable().contains_key(method)
            && self.has(ITERATOR, method)
    }

    /// [`Self::is_adapter_on_collection`] for a receiver given as a type.
    pub fn is_adapter_on_type(&self, receiver: &Type, method: &str) -> bool {
        surface_type_name(receiver).is_some_and(|name| self.is_adapter_on_collection(name, method))
    }

    /// Whether `type_name::name` is declared without `self` (an associated function).
    pub fn is_static(&self, type_name: &str, name: &str) -> bool {
        self.get(type_name, name).is_some_and(|e| e.is_static)
    }

    /// The signature of `receiver.method::<turbofish>(..)`, instantiated, or `None` when the receiver is not a
    /// surface type or the method is not declared for it. Generics left unfixed come back as `Type::Generic`.
    pub fn method_signature(&self, receiver: &Type, method: &str, turbofish: &[Type]) -> Option<MethodSig> {
        if let Some(width_trait) = float_width_trait(receiver) {
            if let Some((_, _, params, ret)) = self.declared(width_trait, method) {
                return Some(MethodSig { params, ret });
            }
        }
        let (name, type_args) = head(receiver)?;
        let (type_generics, method_generics, params, ret) = self.declared(name, method)?;
        // An iterator adapter called directly on a sequence iterates it by reference (`v.filter(f)` is
        // `v.iter().filter(f)`): its element is `&T`.
        let by_ref = name != ITERATOR && !self.has(name, method) && !self.enumerable().contains_key(method)
            && self.has(ITERATOR, method);
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

/// The surface trait declaring the width-dependent methods of a float receiver (`f32` -> `F32`).
pub fn float_width_trait(receiver: &Type) -> Option<&'static str> {
    match receiver {
        Type::Reference(t) | Type::MutableReference(t) => float_width_trait(t),
        Type::Float => Some("F64"),
        Type::Custom(n) if n == "f64" || n == "float" => Some("F64"),
        Type::Custom(n) if n == "f32" => Some("F32"),
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

    /// The per-process cost of the typed surface (parse, lowering table, stored copy): `wj build` pays it once (the budget
    /// of docs/design/table-driven-std-surface.md section 6 is +2% on the conformance corpus). Report only:
    /// `cargo test --release --lib core_surface::tests::parse_cost_report -- --ignored --nocapture`.
    #[test]
    #[ignore = "a cost report, not a check"]
    fn parse_cost_report() {
        let time = |label: &str, f: &mut dyn FnMut()| {
            let n = 200;
            let mut best = std::time::Duration::MAX;
            for _ in 0..n {
                let t = std::time::Instant::now();
                f();
                best = best.min(t.elapsed());
            }
            eprintln!("{label}: {:?}", best);
        };
        time("CoreSurface::parse", &mut || {
            std::hint::black_box(CoreSurface::parse());
        });
        let s = CoreSurface::parse();
        time("declared_traits", &mut || {
            std::hint::black_box(s.declared_traits());
        });
        time("lowering()", &mut || {
            std::hint::black_box(s.lowering());
        });
        time("per-process: parse + on-demand table + Vec/String/Integer/Iterator rows", &mut || {
            use crate::stdlib_surface::lowering::CoreType;
            let s = CoreSurface::parse();
            let t = s.lowering_on_demand();
            for ty in [CoreType::Vec, CoreType::String, CoreType::Integer, CoreType::Iter] {
                std::hint::black_box(t.lookup(Some(ty), "len"));
            }
            std::hint::black_box(s.declared("Vec", "len"));
        });
        time("clone", &mut || {
            std::hint::black_box(s.clone());
        });
        let json = serde_json::to_string(&s).unwrap();
        eprintln!("stored form: {} bytes", json.len());
        time("to_string", &mut || {
            std::hint::black_box(serde_json::to_string(&s).unwrap());
        });
        time("from_str", &mut || {
            std::hint::black_box(serde_json::from_str::<CoreSurface>(&json).unwrap());
        });
    }

    /// The on-demand reader (split by trait, attribute lines read by `read_decorator`, signatures parsed per trait) gives exactly what
    /// the Windjammer parser gives over the whole file: the same methods, signatures and attributes.
    #[test]
    fn the_on_demand_reader_agrees_with_the_parser_over_the_whole_file() {
        use crate::parser::ast::literals::Literal;
        let tokens = crate::lexer::Lexer::new(SURFACE).tokenize_with_locations();
        let program = crate::parser_impl::Parser::new(tokens).parse().expect("the surface parses");
        let text_of = |d: &crate::parser::ast::core::Decorator| RawDecorator {
            name: d.name.clone(),
            args: d
                .arguments
                .iter()
                .map(|(k, e)| match e {
                    crate::parser::Expression::Literal { value: Literal::String(s), .. } => (k.clone(), s.clone()),
                    other => panic!("`{k}` is not a string literal: {other:?}"),
                })
                .collect(),
        };
        let surface = CoreSurface::parse_source(SURFACE);
        let mut methods = 0;
        for item in &program.items {
            let Item::Trait { decl, .. } = item else { continue };
            let raw = surface.raw_trait(&decl.name);
            assert_eq!(raw.decorators, decl.decorators.iter().map(text_of).collect::<Vec<_>>(), "trait `{}`", decl.name);
            assert_eq!(raw.methods.len(), decl.methods.len(), "trait `{}` declares {} methods", decl.name, decl.methods.len());
            for m in &decl.methods {
                let found = raw.methods.iter().find(|r| r.name == m.name).unwrap_or_else(|| panic!("`{}::{}` is read", decl.name, m.name));
                assert_eq!(found.decorators, m.decorators.iter().map(text_of).collect::<Vec<_>>(), "`{}::{}`", decl.name, m.name);
                assert_eq!(found.params, m.parameters.iter().filter(|p| p.name != "self").map(|p| p.name.clone()).collect::<Vec<_>>());
                methods += 1;
            }
        }
        assert!(methods > 300, "the whole file was compared: {methods}");
    }

    /// Reading the rows of one trait does not read the others.
    #[test]
    fn a_trait_is_read_when_asked_for() {
        let surface = CoreSurface::parse();
        assert!(surface.inner.traits.values().all(|c| c.entries.get().is_none()));
        let _ = surface.declared("Vec", "len");
        assert!(surface.inner.traits["Vec"].entries.get().is_some());
        assert!(surface.inner.traits["String"].entries.get().is_none(), "String is not read by a Vec lookup");
        let table = surface.lowering_on_demand();
        let _ = table.lookup(Some(crate::stdlib_surface::lowering::CoreType::Vec), "len");
        assert!(surface.inner.traits["Char"].decorators.get().is_none());
    }

    #[test]
    fn the_per_process_parse_leaves_out_the_examples_and_nothing_else() {
        let (embedded, full) = (CoreSurface::parse(), CoreSurface::parse_source(SURFACE));
        let declared = |s: &CoreSurface| s.declared_traits().into_iter().map(|t| (t.name, t.methods.into_iter().map(|m| (m.name, m.params)).collect::<Vec<_>>())).collect::<Vec<_>>();
        assert_eq!(declared(&embedded), declared(&full), "the same traits, methods and parameters");
        let examples = |s: &CoreSurface| s.declared_traits().iter().flat_map(|t| &t.methods).flat_map(|m| &m.decorators).filter(|d| d.name == "example").count();
        assert_eq!(examples(&embedded), 0);
        assert!(examples(&full) > 100, "the file on disk keeps them: {}", examples(&full));
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
