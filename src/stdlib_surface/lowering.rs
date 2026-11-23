//! The lowering table of the std surface (docs/design/table-driven-std-surface.md): one row per std method, with one
//! lowering rule per portable target, read from the `@js` / `@go` / `@example` decorators on the declarations in
//! `std/rust_core.wji`.
//!
//! The portable emitters (JavaScript / TypeScript, Go) and the portable typer are interpreters of this table: a std
//! method is added or changed in the declaration, never in an emitter. The vocabulary here (guards, hooks, modifiers,
//! facts) names type shapes and emitter capabilities, never std methods; `tests/std_lowering_table_test.rs` pins how
//! many variants each enum has, so a new one needs a reason in that test (gate G6).
//!
//! This module owns the data types and the validation of the decorators. It reads no parser type: the typed surface
//! (`typing::core_surface`) hands over the decorators as plain strings ([`RawDecorator`]).

use std::collections::BTreeMap;

/// A portable lowering target. TypeScript uses the JavaScript rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Target {
    Js,
    Go,
}

impl Target {
    pub const ALL: [Target; 2] = [Target::Js, Target::Go];

    /// The decorator that carries this target's rules.
    pub fn decorator(self) -> &'static str {
        match self {
            Target::Js => "js",
            Target::Go => "go",
        }
    }
}

/// The std types whose methods are migrated to the table, named by the trait that declares them in
/// `std/rust_core.wji`. Closed: a trait with lowering decorators must be listed here (a stage adds its types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum CoreType {
    Float,
    Integer,
    Char,
    String,
    Option,
    Result,
    /// `HashMap<K, V>` and `BTreeMap<K, V>`: one portable type (`Ty::Map`), declared by `trait HashMap`.
    Map,
    /// `HashSet<T>` and `BTreeSet<T>`: one portable type (`Ty::Set`), declared by `trait HashSet`.
    Set,
    /// `Vec<T>` (a list; `VecDeque` is not a portable type): `trait Vec` is `@adapts(to = "Iterator")`, so a list has every iterator row too.
    Vec,
    /// A lazy sequence (`Iter`, `Range`): `trait Iterator`.
    Iter,
    /// What every value has (`to_string`): the last place a lookup falls to, for any receiver type; `trait Any`.
    Any,
}

impl CoreType {
    pub const ALL: [CoreType; 11] = [
        CoreType::Float,
        CoreType::Integer,
        CoreType::Char,
        CoreType::String,
        CoreType::Option,
        CoreType::Result,
        CoreType::Map,
        CoreType::Set,
        CoreType::Vec,
        CoreType::Iter,
        CoreType::Any,
    ];

    /// The trait of `std/rust_core.wji` that declares this type's methods.
    pub fn trait_name(self) -> &'static str {
        match self {
            CoreType::Float => "Float",
            CoreType::Integer => "Integer",
            CoreType::Char => "Char",
            CoreType::String => "String",
            CoreType::Option => "Option",
            CoreType::Result => "Result",
            CoreType::Map => "HashMap",
            CoreType::Set => "HashSet",
            CoreType::Vec => "Vec",
            CoreType::Iter => "Iterator",
            CoreType::Any => "Any",
        }
    }

    /// What the type's values are called in an "unsupported" message (`float method `foo``).
    pub fn noun(self) -> &'static str {
        match self {
            CoreType::Float => "float",
            CoreType::Integer => "integer",
            CoreType::Char => "char",
            CoreType::String => "string",
            CoreType::Option => "Option",
            CoreType::Result => "Result",
            CoreType::Map => "HashMap",
            CoreType::Set => "HashSet",
            CoreType::Vec => "list",
            CoreType::Iter => "iterator",
            CoreType::Any => "value",
        }
    }

    pub fn from_trait(name: &str) -> Option<CoreType> {
        CoreType::ALL.into_iter().find(|t| t.trait_name() == name)
    }
}

/// A condition on the shape of the call's types, evaluated by the emitter (`when = "..."`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Guard {
    /// The element type of the receiver is a primitive (`includes` instead of a structural search).
    ElemPrim,
    /// ... and not `bool`.
    ElemPrimNotBool,
    /// The receiver is a 64-bit integer (a BigInt in JavaScript).
    RecvWide,
    /// The receiver is an `i32`.
    RecvI32,
    /// The receiver is a `u32`.
    RecvU32,
    /// The call's result is a `Result` whose `Ok` is an integer (`parse::<i32>()`), an `f32`, an `f64`, a `bool`.
    OkInt,
    OkF32,
    OkF64,
    OkBool,
    /// The map type of the call (the receiver, or the result of an associated function) has a key a JavaScript `Map`
    /// compares by value: an integer, string, char or bool.
    KeyPrim,
    /// The call's result type has this shape (`collect` into a `HashSet`; `sum` of an integer or a float): a row that depends on
    /// the type the call is expected to give.
    ResInt,
    ResFloat,
    ResSet,
    ResMap,
    ResStr,
    ResResult,
    ResOption,
}

impl Guard {
    pub const ALL: [Guard; 17] = [
        Guard::ElemPrim,
        Guard::ElemPrimNotBool,
        Guard::RecvWide,
        Guard::RecvI32,
        Guard::RecvU32,
        Guard::OkInt,
        Guard::OkF32,
        Guard::OkF64,
        Guard::OkBool,
        Guard::KeyPrim,
        Guard::ResInt,
        Guard::ResFloat,
        Guard::ResSet,
        Guard::ResMap,
        Guard::ResStr,
        Guard::ResResult,
        Guard::ResOption,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Guard::ElemPrim => "elem_prim",
            Guard::ElemPrimNotBool => "elem_prim_not_bool",
            Guard::RecvWide => "t_wide",
            Guard::RecvI32 => "t_i32",
            Guard::RecvU32 => "t_u32",
            Guard::OkInt => "ok_int",
            Guard::OkF32 => "ok_f32",
            Guard::OkF64 => "ok_f64",
            Guard::OkBool => "ok_bool",
            Guard::KeyPrim => "key_prim",
            Guard::ResInt => "r_int",
            Guard::ResFloat => "r_float",
            Guard::ResSet => "r_set",
            Guard::ResMap => "r_map",
            Guard::ResStr => "r_str",
            Guard::ResResult => "r_result",
            Guard::ResOption => "r_option",
        }
    }
}

/// A type-directed emitter the template language cannot express (`hook = "..."`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Hook {
    /// The target's text of the receiver's value (`to_string`).
    Display,
    /// The target's numeric conversion from the first argument's type to the call's type (`i64::from(x)`, `f64::from(x)`).
    Cast,
    /// A copy of the elements of the receiver by the element type's own cloner (JavaScript `cloned` / `copied`).
    Clone,
}

impl Hook {
    pub const ALL: [Hook; 3] = [Hook::Display, Hook::Cast, Hook::Clone];

    pub fn name(self) -> &'static str {
        match self {
            Hook::Display => "display",
            Hook::Cast => "cast",
            Hook::Clone => "clone",
        }
    }
}

/// How an operand is printed (`{name:mod}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Modifier {
    /// Converted to the compute type the trait declares with `widen` (Go: `float64(x)` for an `f32`; nothing for `f64`).
    F64,
    /// A `char` operand where the target wants a string (Go: `string(rune(x))`); any other operand is printed as it is.
    Str,
    /// An integer operand where the target wants a plain number (JavaScript: `Number(x)`, a BigInt is not an index).
    Num,
}

impl Modifier {
    pub const ALL: [Modifier; 3] = [Modifier::F64, Modifier::Str, Modifier::Num];

    pub fn name(self) -> &'static str {
        match self {
            Modifier::F64 => "f64",
            Modifier::Str => "str",
            Modifier::Num => "num",
        }
    }
}

/// A fact about the receiver's type `T` or the result `R` (`{T.max}`), computed by the emitter's numeric helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Fact {
    Bits,
    Signed,
    Wide,
    Min,
    Max,
    Zero,
    One,
    Chk,
    Go,
    /// The text that opens a wrap of an expression into `T`'s range (two's complement); `WrapClose` closes it.
    WrapOpen,
    WrapClose,
    /// The bounds of the type as arbitrary-precision literals (`-128n`, `255n`), whatever the width: what a parser compares in.
    Lo,
    Hi,
    /// The target's text of the debug formatter of a type (`$dbgStr`, `$dbgVec($dbgInt)`): what a `Result`'s `unwrap` prints
    /// of its error. Asked of the error type (`{E.dbg}`); a target whose runtime formats errors itself has no such fact.
    Dbg,
}

impl Fact {
    pub const ALL: [Fact; 14] = [
        Fact::Bits,
        Fact::Signed,
        Fact::Wide,
        Fact::Min,
        Fact::Max,
        Fact::Zero,
        Fact::One,
        Fact::Chk,
        Fact::Go,
        Fact::WrapOpen,
        Fact::WrapClose,
        Fact::Lo,
        Fact::Hi,
        Fact::Dbg,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Fact::Bits => "bits",
            Fact::Signed => "signed",
            Fact::Wide => "wide",
            Fact::Min => "min",
            Fact::Max => "max",
            Fact::Zero => "zero",
            Fact::One => "one",
            Fact::Chk => "chk",
            Fact::Go => "go",
            Fact::WrapOpen => "wrap_open",
            Fact::WrapClose => "wrap_close",
            Fact::Lo => "lo",
            Fact::Hi => "hi",
            Fact::Dbg => "dbg",
        }
    }
}

/// The compute type a trait declares for a target (`@go(widen = "f64")`): operands marked `:f64` are converted to it
/// and a result declared `T` is converted back to the receiver's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Widen {
    F64,
}

/// One lowering rule of a row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rule {
    /// Print the template (`emit = "..."`); with `when`, only when every guard holds. The first matching rule wins.
    Emit { template: String, when: Vec<Guard> },
    /// A place update (`assign = "..."`, JavaScript): the template is the new value of the receiver's place, printed as
    /// `(place = value, undefined)`; `{&self}` is the place.
    Assign(String),
    /// A type-directed emitter (`hook = "..."`).
    Hook(Hook),
    /// Not available on this target; the text is the diagnostic and the ledger reason (`unsupported = "..."`).
    Unsupported(String),
}

/// `@example(src = "<expression>", expect = "<printed result>")` (with an optional `given = "<statements>"`): the program the per-row conformance runner builds.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Example {
    /// Statements the example needs before its expression (`let x: u8 = 250`; `;` separates several), when a bare
    /// literal would not carry the type the row is about.
    pub given: Option<String>,
    pub src: String,
    pub expect: String,
}

/// A decorator as the parser read it, with every argument as text (a string literal's contents).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RawDecorator {
    pub name: String,
    pub args: Vec<(String, String)>,
}

impl RawDecorator {
    fn arg(&self, key: &str) -> Option<&str> {
        self.args.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// A declared method as the typed surface read it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RawMethod {
    pub name: String,
    /// Parameter names, without `self`.
    pub params: Vec<String>,
    /// The declared result is the trait's own `T` (the receiver's type).
    pub returns_receiver: bool,
    /// Declared `&mut self`.
    pub changes_receiver: bool,
    /// An associated function (`Vec::new`, `i64::MAX`): it has no receiver.
    pub is_static: bool,
    /// The declared result is `usize` itself (an index slot, ADR-0028).
    pub returns_usize: bool,
    pub decorators: Vec<RawDecorator>,
}

/// A declared trait as the typed surface read it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RawTrait {
    pub name: String,
    pub decorators: Vec<RawDecorator>,
    pub methods: Vec<RawMethod>,
}

/// The shape of expected type a result taken from the context accepts (`@result_from_context(kind = "..")`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextKind {
    /// A collection a sequence is gathered into: list, set, map, string, `Result`, `Option` (`collect`).
    Collection,
    /// A number (`sum`, `product`).
    Numeric,
}

impl ContextKind {
    pub fn name(self) -> &'static str {
        match self {
            ContextKind::Collection => "collection",
            ContextKind::Numeric => "numeric",
        }
    }
}

/// One std method: its declared shape and its rules per target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub params: Vec<String>,
    pub returns_receiver: bool,
    /// Declared `&mut self`: the method changes its receiver.
    pub changes_receiver: bool,
    /// The declared result is `usize`: a call typed as a narrower integer converts it (the length narrowing, ADR-0028).
    pub returns_usize: bool,
    /// `@result_from_context(kind = "..")`: the call's type is the expected type when it has this shape.
    pub from_context: Option<ContextKind>,
    pub rules: BTreeMap<Target, Vec<Rule>>,
    pub example: Option<Example>,
    /// `@internal(reason = "...")`: a lowering target the compiler produces itself (not a user-facing method).
    pub internal: Option<String>,
}

impl Row {
    pub fn rules(&self, target: Target) -> &[Rule] {
        self.rules.get(&target).map_or(&[], Vec::as_slice)
    }

    /// The rule that applies: the first whose guards all hold (`holds` answers each guard from the call's types).
    pub fn select(&self, target: Target, holds: impl Fn(Guard) -> bool) -> Option<&Rule> {
        self.rules(target).iter().find(|r| match r {
            Rule::Emit { when, .. } => when.iter().all(|g| holds(*g)),
            _ => true,
        })
    }
}

/// A condition a trait puts on every call of its methods and associated functions on a target (`@js(requires = "key_prim",
/// reason = "...")`): when the guard does not hold the call is refused with the reason (`{K}` is the key type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub guard: Guard,
    pub reason: String,
}

/// Reads one declared trait on demand (the typed surface, which parses a trait the first time it is asked for).
pub type TraitSource = std::sync::Arc<dyn Fn(&str) -> RawTrait + Send + Sync>;

/// The rows of one trait: its methods (a migrated type's) and its associated functions with a rule.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TraitRows {
    /// By name: a lookup by `&str` allocates nothing.
    rows: BTreeMap<String, Row>,
    /// The associated functions (`Vec::new`, `i64::MAX`): of a migrated type all of them, of any other trait only those with rules
    /// (the other `Vec` methods are not migrated yet).
    statics: BTreeMap<String, Row>,
}

/// The rows of the migrated types, keyed by (type, method name). The rows of a trait are built the first time a call on it is
/// lowered (a table made by [`LoweringTable::on_demand`]), so a compilation pays for the traits it uses.
#[derive(Clone, Default)]
pub struct LoweringTable {
    /// Every declared trait; the rows are built on first use.
    traits: BTreeMap<String, std::sync::OnceLock<TraitRows>>,
    source: Option<TraitSource>,
    widen: BTreeMap<(CoreType, Target), Widen>,
    /// The trait that declares the associated functions of a type spelling (`i64` -> `Integer`, `BTreeMap` -> itself).
    owners: BTreeMap<String, String>,
    requires: BTreeMap<String, BTreeMap<Target, Requirement>>,
    /// `@adapts(to = "Iterator")`: the types whose rows a lookup falls to when the type has none (a list has the iterator's).
    adapts: BTreeMap<CoreType, Vec<CoreType>>,
}

impl std::fmt::Debug for LoweringTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoweringTable").field("traits", &self.traits.keys().collect::<Vec<_>>()).finish()
    }
}

impl PartialEq for LoweringTable {
    fn eq(&self, other: &Self) -> bool {
        self.traits.keys().eq(other.traits.keys())
            && self.traits.keys().all(|t| self.built(t) == other.built(t))
            && self.widen == other.widen
            && self.owners == other.owners
            && self.requires == other.requires
            && self.adapts == other.adapts
    }
}
impl Eq for LoweringTable {}

impl LoweringTable {
    /// Build the table from the declared traits. Only traits named by a [`CoreType`] are rows; a decorator the table
    /// does not know is an error (a typo must not silently drop a rule). Every row is read now.
    pub fn from_declarations(traits: &[RawTrait]) -> Result<LoweringTable, String> {
        let mut table = LoweringTable::default();
        for t in traits {
            table.add_trait_level(t)?;
            let _ = table.traits.entry(t.name.clone()).or_default().set(build_trait(t)?);
        }
        Ok(table)
    }

    /// A table whose trait-level decorators are read now (`traits`: each declared trait with its own decorators) and whose rows
    /// are read from `source` when first asked for.
    pub fn on_demand(traits: Vec<(String, Vec<RawDecorator>)>, source: TraitSource) -> Result<LoweringTable, String> {
        let mut table = LoweringTable { source: Some(source), ..Default::default() };
        for (name, decorators) in traits {
            table.add_trait_level(&RawTrait { name: name.clone(), decorators, methods: Vec::new() })?;
            table.traits.entry(name).or_default();
        }
        Ok(table)
    }

    fn add_trait_level(&mut self, t: &RawTrait) -> Result<(), String> {
        let table = self;
        let ty = CoreType::from_trait(&t.name);
        for d in &t.decorators {
            if d.name == "owner" {
                // `@owner(names = "i8, i16")`: the spellings of the types whose associated functions this trait declares
                let names = d.arg("names").ok_or_else(|| format!("trait `{}`: `@owner` takes `names = \"..\"`", t.name))?;
                for n in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    table.owners.insert(n.to_string(), t.name.clone());
                }
                continue;
            }
            if d.name == "adapts" {
                let ty = ty.ok_or_else(|| format!("trait `{}`: `@adapts` on a trait that is not a migrated std type", t.name))?;
                let to = d.arg("to").and_then(CoreType::from_trait).ok_or_else(|| format!("trait `{}`: `@adapts` takes `to = \"<migrated trait>\"`", t.name))?;
                table.adapts.entry(ty).or_default().push(to);
                continue;
            }
            let target = target_of(&d.name).ok_or_else(|| format!("trait `{}`: decorator `@{}` is not a lowering decorator", t.name, d.name))?;
            match (d.arg("widen"), d.arg("requires")) {
                (Some("f64"), None) if d.args.len() == 1 => {
                    let ty = ty.ok_or_else(|| format!("trait `{}`: `@{} widen` on a trait that is not a migrated std type", t.name, d.name))?;
                    table.widen.insert((ty, target), Widen::F64);
                }
                (None, Some(g)) if d.args.len() == 2 && d.arg("reason").is_some() => {
                    let guard = Guard::ALL.into_iter().find(|k| k.name() == g).ok_or_else(|| format!("trait `{}`: unknown guard `{g}`", t.name))?;
                    let reason = d.arg("reason").unwrap_or_default().to_string();
                    table.requires.entry(t.name.clone()).or_default().insert(target, Requirement { guard, reason });
                }
                _ => return Err(format!("trait `{}`: `@{}` takes `widen = \"f64\"` or `requires = \"<guard>\", reason = \"..\"`", t.name, d.name)),
            }
        }
        Ok(())
    }

    /// The rows of the trait `name`, built on first use.
    fn built(&self, name: &str) -> Option<&TraitRows> {
        let cell = self.traits.get(name)?;
        Some(cell.get_or_init(|| {
            let source = self.source.as_ref().unwrap_or_else(|| panic!("lowering table: trait `{name}` has no rows"));
            build_trait(&source(name)).unwrap_or_else(|e| panic!("std/rust_core.wji lowering decorators: {e}"))
        }))
    }

    /// The associated function `name` of the type spelled `owner` (`Vec`, `i64`, `BTreeMap`), with the trait that declares it.
    pub fn static_row(&self, owner: &str, name: &str) -> Option<(&str, &Row)> {
        let trait_name = self.owners.get(owner).map_or(owner, String::as_str);
        let (trait_name, _) = self.traits.get_key_value(trait_name)?;
        Some((trait_name.as_str(), self.built(trait_name)?.statics.get(name)?))
    }

    /// The trait whose associated functions the type spelled `owner` has.
    pub fn owner_trait<'a>(&'a self, owner: &'a str) -> &'a str {
        self.owners.get(owner).map_or(owner, String::as_str)
    }

    /// Every associated function with its trait, in (trait, name) order.
    pub fn statics(&self) -> impl Iterator<Item = (&str, &Row)> {
        self.traits.keys().filter_map(|t| Some((t, self.built(t)?))).flat_map(|(t, b)| b.statics.values().map(move |r| (t.as_str(), r)))
    }

    /// The condition `trait_name` puts on every call on `target`.
    pub fn requirement(&self, trait_name: &str, target: Target) -> Option<&Requirement> {
        self.requires.get(trait_name)?.get(&target)
    }

    pub fn row(&self, ty: CoreType, name: &str) -> Option<&Row> {
        self.built(ty.trait_name())?.rows.get(name)
    }

    /// The row a call of `name` on a value of `ty` is, with the type that declares it: the type's own, then the types it adapts
    /// (a list has the iterator's), then what every value has (`Any`). `ty` is `None` for a receiver no trait is declared for.
    pub fn lookup(&self, ty: Option<CoreType>, name: &str) -> Option<(CoreType, &Row)> {
        let own = ty.into_iter().flat_map(|t| std::iter::once(t).chain(self.adapts.get(&t).into_iter().flatten().copied()));
        own.chain(std::iter::once(CoreType::Any)).find_map(|t| Some((t, self.row(t, name)?)))
    }

    /// Every row, in (type, name) order.
    pub fn rows(&self) -> impl Iterator<Item = (CoreType, &Row)> {
        CoreType::ALL.into_iter().filter_map(|t| Some((t, self.built(t.trait_name())?))).flat_map(|(t, b)| b.rows.values().map(move |r| (t, r)))
    }

    pub fn widen(&self, ty: CoreType, target: Target) -> Option<Widen> {
        self.widen.get(&(ty, target)).copied()
    }
}

/// The rows of one declared trait.
fn build_trait(t: &RawTrait) -> Result<TraitRows, String> {
    let ty = CoreType::from_trait(&t.name);
    let mut built = TraitRows::default();
    for m in &t.methods {
        let lowered = m.decorators.iter().any(|d| is_lowering_decorator(&d.name));
        match ty {
            Some(_) if !m.is_static => {
                built.rows.insert(m.name.clone(), row_of(&t.name, m)?);
            }
            // an associated function of a migrated type is a row (the gates ask for its rules); of any other trait, only
            // when it has rules (`Vec::new`: the other `Vec` methods are not migrated yet)
            _ if m.is_static && (ty.is_some() || lowered) => {
                built.statics.insert(m.name.clone(), row_of(&t.name, m)?);
            }
            _ if lowered => return Err(format!("`{}::{}`: `@{}` on a trait that is not a migrated std type (CoreType)", t.name, m.name, m.decorators[0].name)),
            _ => {}
        }
    }
    Ok(built)
}

fn is_lowering_decorator(name: &str) -> bool {
    target_of(name).is_some() || name == "example" || name == "internal" || name == "result_from_context"
}


fn target_of(name: &str) -> Option<Target> {
    Target::ALL.into_iter().find(|t| t.decorator() == name)
}

fn row_of(trait_name: &str, m: &RawMethod) -> Result<Row, String> {
    let at = |what: &str| format!("`{trait_name}::{}`: {what}", m.name);
    let mut rules: BTreeMap<Target, Vec<Rule>> = BTreeMap::new();
    let mut example = None;
    let mut internal = None;
    let mut from_context = None;
    for d in &m.decorators {
        if let Some(target) = target_of(&d.name) {
            rules.entry(target).or_default().push(rule_of(d).map_err(|e| at(&e))?);
        } else if d.name == "example" {
            let (Some(src), Some(expect)) = (d.arg("src"), d.arg("expect")) else {
                return Err(at("`@example` takes `src = \"...\", expect = \"...\"`"));
            };
            if example.replace(Example { given: d.arg("given").map(str::to_string), src: src.to_string(), expect: expect.to_string() }).is_some() {
                return Err(at("more than one `@example`"));
            }
        } else if d.name == "result_from_context" {
            from_context = Some(match d.arg("kind") {
                Some("collection") => ContextKind::Collection,
                Some("numeric") => ContextKind::Numeric,
                _ => return Err(at("`@result_from_context` takes `kind = \"collection\"` or `kind = \"numeric\"`")),
            });
        } else if d.name == "internal" {
            internal = Some(d.arg("reason").ok_or_else(|| at("`@internal` needs `reason = \"...\"`"))?.to_string());
        }
    }
    Ok(Row { name: m.name.clone(), params: m.params.clone(), returns_receiver: m.returns_receiver, changes_receiver: m.changes_receiver, returns_usize: m.returns_usize, from_context, rules, example, internal })
}

fn rule_of(d: &RawDecorator) -> Result<Rule, String> {
    let unknown = d.args.iter().find(|(k, _)| !["emit", "when", "hook", "unsupported", "assign"].contains(&k.as_str()));
    if let Some((k, _)) = unknown {
        return Err(format!("`@{}` has no argument `{k}`", d.name));
    }
    let (emit, hook, unsupported, assign) = (d.arg("emit"), d.arg("hook"), d.arg("unsupported"), d.arg("assign"));
    let given = [emit.is_some(), hook.is_some(), unsupported.is_some(), assign.is_some()].iter().filter(|b| **b).count();
    if given != 1 {
        return Err(format!("`@{}` needs exactly one of `emit`, `assign`, `hook`, `unsupported`", d.name));
    }
    if let Some(t) = assign {
        if d.arg("when").is_some() {
            return Err(format!("`@{}`: `when` goes with `emit`", d.name));
        }
        return Ok(Rule::Assign(t.to_string()));
    }
    if d.arg("when").is_some() && emit.is_none() {
        return Err(format!("`@{}`: `when` goes with `emit`", d.name));
    }
    if let Some(t) = emit {
        let when = match d.arg("when") {
            None => Vec::new(),
            Some(list) => list
                .split(',')
                .map(|g| Guard::ALL.into_iter().find(|k| k.name() == g.trim()).ok_or_else(|| format!("`@{}`: unknown guard `{}`", d.name, g.trim())))
                .collect::<Result<_, _>>()?,
        };
        return Ok(Rule::Emit { template: t.to_string(), when });
    }
    if let Some(h) = hook {
        return Hook::ALL.into_iter().find(|k| k.name() == h).map(Rule::Hook).ok_or_else(|| format!("`@{}`: unknown hook `{h}`", d.name));
    }
    Ok(Rule::Unsupported(unsupported.unwrap_or_default().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(name: &str, args: &[(&str, &str)]) -> RawDecorator {
        RawDecorator { name: name.into(), args: args.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
    }

    fn float_trait(methods: Vec<RawMethod>, decorators: Vec<RawDecorator>) -> RawTrait {
        RawTrait { name: "Float".into(), decorators, methods }
    }

    fn method(name: &str, decorators: Vec<RawDecorator>) -> RawMethod {
        RawMethod { name: name.into(), params: vec![], returns_receiver: true, changes_receiver: false, is_static: false, returns_usize: false, decorators }
    }

    #[test]
    fn rules_guards_and_widen_are_read() {
        let t = float_trait(
            vec![method(
                "sqrt",
                vec![
                    dec("js", &[("emit", "a"), ("when", "elem_prim, elem_prim_not_bool")]),
                    dec("js", &[("emit", "b")]),
                    dec("go", &[("hook", "display")]),
                    dec("example", &[("src", "1.0"), ("expect", "1")]),
                ],
            )],
            vec![dec("go", &[("widen", "f64")])],
        );
        let table = LoweringTable::from_declarations(&[t]).unwrap();
        let row = table.row(CoreType::Float, "sqrt").unwrap();
        assert_eq!(row.rules(Target::Js).len(), 2);
        assert_eq!(row.select(Target::Js, |_| true), Some(&Rule::Emit { template: "a".into(), when: vec![Guard::ElemPrim, Guard::ElemPrimNotBool] }));
        assert_eq!(row.select(Target::Js, |_| false), Some(&Rule::Emit { template: "b".into(), when: vec![] }));
        assert_eq!(row.select(Target::Go, |_| false), Some(&Rule::Hook(Hook::Display)));
        assert_eq!(row.example, Some(Example { given: None, src: "1.0".into(), expect: "1".into() }));
        assert_eq!(table.widen(CoreType::Float, Target::Go), Some(Widen::F64));
        assert_eq!(table.widen(CoreType::Float, Target::Js), None);
    }

    #[test]
    fn an_assign_rule_and_the_ok_guards_are_read() {
        let t = RawTrait {
            name: "String".into(),
            decorators: vec![],
            methods: vec![method(
                "push",
                vec![dec("js", &[("assign", "{&self} + {ch}")]), dec("go", &[("emit", "p({&self})"), ("when", "ok_int, ok_f32")]), dec("go", &[("unsupported", "no")])],
            )],
        };
        let table = LoweringTable::from_declarations(&[t]).unwrap();
        let row = table.row(CoreType::String, "push").unwrap();
        assert_eq!(row.select(Target::Js, |_| false), Some(&Rule::Assign("{&self} + {ch}".into())));
        assert_eq!(row.select(Target::Go, |g| g == Guard::OkInt || g == Guard::OkF32), Some(&Rule::Emit { template: "p({&self})".into(), when: vec![Guard::OkInt, Guard::OkF32] }));
        assert_eq!(row.select(Target::Go, |_| false), Some(&Rule::Unsupported("no".into())));
        assert_eq!(table.rows().map(|(t, r)| (t, r.name.as_str())).collect::<Vec<_>>(), vec![(CoreType::String, "push")]);
    }

    #[test]
    fn a_malformed_decorator_is_an_error_not_a_dropped_rule() {
        let bad = |d: RawDecorator| LoweringTable::from_declarations(&[float_trait(vec![method("m", vec![d])], vec![])]);
        assert!(bad(dec("js", &[("emit", "a"), ("hook", "display")])).is_err());
        assert!(bad(dec("js", &[])).is_err());
        assert!(bad(dec("js", &[("emitt", "a")])).is_err());
        assert!(bad(dec("js", &[("emit", "a"), ("when", "no_such_guard")])).is_err());
        assert!(bad(dec("go", &[("hook", "no_such_hook")])).is_err());
        assert!(bad(dec("js", &[("assign", "a"), ("when", "ok_int")])).is_err());
        assert!(bad(dec("js", &[("assign", "a"), ("emit", "b")])).is_err());
        assert!(bad(dec("example", &[("src", "1.0")])).is_err());
        let off_type = RawTrait { name: "Instant".into(), decorators: vec![], methods: vec![method("push", vec![dec("js", &[("emit", "a")])])] };
        assert!(LoweringTable::from_declarations(&[off_type]).is_err());
    }
}
