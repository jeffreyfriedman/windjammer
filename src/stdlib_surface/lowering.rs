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
}

impl CoreType {
    pub const ALL: [CoreType; 2] = [CoreType::Float, CoreType::Integer];

    /// The trait of `std/rust_core.wji` that declares this type's methods.
    pub fn trait_name(self) -> &'static str {
        match self {
            CoreType::Float => "Float",
            CoreType::Integer => "Integer",
        }
    }

    /// What the type's values are called in an "unsupported" message (`float method `foo``).
    pub fn noun(self) -> &'static str {
        match self {
            CoreType::Float => "float",
            CoreType::Integer => "integer",
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
}

impl Guard {
    pub const ALL: [Guard; 5] = [Guard::ElemPrim, Guard::ElemPrimNotBool, Guard::RecvWide, Guard::RecvI32, Guard::RecvU32];

    pub fn name(self) -> &'static str {
        match self {
            Guard::ElemPrim => "elem_prim",
            Guard::ElemPrimNotBool => "elem_prim_not_bool",
            Guard::RecvWide => "t_wide",
            Guard::RecvI32 => "t_i32",
            Guard::RecvU32 => "t_u32",
        }
    }
}

/// A type-directed emitter the template language cannot express (`hook = "..."`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Hook {
    /// The target's text of the receiver's value (`to_string`).
    Display,
}

impl Hook {
    pub const ALL: [Hook; 1] = [Hook::Display];

    pub fn name(self) -> &'static str {
        match self {
            Hook::Display => "display",
        }
    }
}

/// How an operand is printed (`{name:mod}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Modifier {
    /// Converted to the compute type the trait declares with `widen` (Go: `float64(x)` for an `f32`; nothing for `f64`).
    F64,
}

impl Modifier {
    pub const ALL: [Modifier; 1] = [Modifier::F64];

    pub fn name(self) -> &'static str {
        match self {
            Modifier::F64 => "f64",
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
}

impl Fact {
    pub const ALL: [Fact; 11] =
        [Fact::Bits, Fact::Signed, Fact::Wide, Fact::Min, Fact::Max, Fact::Zero, Fact::One, Fact::Chk, Fact::Go, Fact::WrapOpen, Fact::WrapClose];

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
    pub decorators: Vec<RawDecorator>,
}

/// A declared trait as the typed surface read it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RawTrait {
    pub name: String,
    pub decorators: Vec<RawDecorator>,
    pub methods: Vec<RawMethod>,
}

/// One std method: its declared shape and its rules per target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub params: Vec<String>,
    pub returns_receiver: bool,
    /// Declared `&mut self`: the method changes its receiver.
    pub changes_receiver: bool,
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

/// The rows of the migrated types, keyed by (type, method name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoweringTable {
    rows: BTreeMap<(CoreType, String), Row>,
    widen: BTreeMap<(CoreType, Target), Widen>,
}

impl LoweringTable {
    /// Build the table from the declared traits. Only traits named by a [`CoreType`] are rows; a decorator the table
    /// does not know is an error (a typo must not silently drop a rule).
    pub fn from_declarations(traits: &[RawTrait]) -> Result<LoweringTable, String> {
        let mut table = LoweringTable::default();
        for t in traits {
            let Some(ty) = CoreType::from_trait(&t.name) else {
                for m in &t.methods {
                    if let Some(d) = m.decorators.iter().find(|d| is_lowering_decorator(&d.name)) {
                        return Err(format!("`{}::{}`: `@{}` on a trait that is not a migrated std type (CoreType)", t.name, m.name, d.name));
                    }
                }
                continue;
            };
            for d in &t.decorators {
                let target = target_of(&d.name).ok_or_else(|| format!("trait `{}`: decorator `@{}` is not a lowering decorator", t.name, d.name))?;
                match d.arg("widen") {
                    Some("f64") if d.args.len() == 1 => {
                        table.widen.insert((ty, target), Widen::F64);
                    }
                    other => return Err(format!("trait `{}`: `@{}` takes only `widen = \"f64\"` (got {other:?})", t.name, d.name)),
                }
            }
            for m in &t.methods {
                let row = row_of(&t.name, m)?;
                table.rows.insert((ty, m.name.clone()), row);
            }
        }
        Ok(table)
    }

    pub fn row(&self, ty: CoreType, name: &str) -> Option<&Row> {
        self.rows.get(&(ty, name.to_string()))
    }

    /// Every row, in (type, name) order.
    pub fn rows(&self) -> impl Iterator<Item = (CoreType, &Row)> {
        self.rows.iter().map(|((t, _), r)| (*t, r))
    }

    pub fn widen(&self, ty: CoreType, target: Target) -> Option<Widen> {
        self.widen.get(&(ty, target)).copied()
    }
}

fn is_lowering_decorator(name: &str) -> bool {
    target_of(name).is_some() || name == "example" || name == "internal"
}

fn target_of(name: &str) -> Option<Target> {
    Target::ALL.into_iter().find(|t| t.decorator() == name)
}

fn row_of(trait_name: &str, m: &RawMethod) -> Result<Row, String> {
    let at = |what: &str| format!("`{trait_name}::{}`: {what}", m.name);
    let mut rules: BTreeMap<Target, Vec<Rule>> = BTreeMap::new();
    let mut example = None;
    let mut internal = None;
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
        } else if d.name == "internal" {
            internal = Some(d.arg("reason").ok_or_else(|| at("`@internal` needs `reason = \"...\"`"))?.to_string());
        }
    }
    Ok(Row { name: m.name.clone(), params: m.params.clone(), returns_receiver: m.returns_receiver, changes_receiver: m.changes_receiver, rules, example, internal })
}

fn rule_of(d: &RawDecorator) -> Result<Rule, String> {
    let unknown = d.args.iter().find(|(k, _)| !["emit", "when", "hook", "unsupported"].contains(&k.as_str()));
    if let Some((k, _)) = unknown {
        return Err(format!("`@{}` has no argument `{k}`", d.name));
    }
    let (emit, hook, unsupported) = (d.arg("emit"), d.arg("hook"), d.arg("unsupported"));
    let given = [emit.is_some(), hook.is_some(), unsupported.is_some()].iter().filter(|b| **b).count();
    if given != 1 {
        return Err(format!("`@{}` needs exactly one of `emit`, `hook`, `unsupported`", d.name));
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
        RawMethod { name: name.into(), params: vec![], returns_receiver: true, changes_receiver: false, decorators }
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
    fn a_malformed_decorator_is_an_error_not_a_dropped_rule() {
        let bad = |d: RawDecorator| LoweringTable::from_declarations(&[float_trait(vec![method("m", vec![d])], vec![])]);
        assert!(bad(dec("js", &[("emit", "a"), ("hook", "display")])).is_err());
        assert!(bad(dec("js", &[])).is_err());
        assert!(bad(dec("js", &[("emitt", "a")])).is_err());
        assert!(bad(dec("js", &[("emit", "a"), ("when", "no_such_guard")])).is_err());
        assert!(bad(dec("go", &[("hook", "no_such_hook")])).is_err());
        assert!(bad(dec("example", &[("src", "1.0")])).is_err());
        let off_type = RawTrait { name: "Vec".into(), decorators: vec![], methods: vec![method("push", vec![dec("js", &[("emit", "a")])])] };
        assert!(LoweringTable::from_declarations(&[off_type]).is_err());
    }
}
