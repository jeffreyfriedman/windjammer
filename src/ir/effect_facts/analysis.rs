//! The effect pass: per-function effect sets with provenance, computed from resolved calls.
//!
//! Input is `crate::resolve::ResolvedProgram` (identity-based call resolution, ADR-0017); output is
//! [`EffectFacts`]. Rules (spec chapter `docs/spec/effects.md`):
//!
//! * SPEC-EFF-002  a call to a sink (`sinks.rs`), an `extern` fn or an `@effect` fn performs its effect
//! * SPEC-EFF-003  a function performs the effects of every function it calls (fixpoint, recursion included)
//! * SPEC-EFF-004  calling a function-typed parameter is *latent*: the argument's effects belong to the caller
//!   (closure bodies are attributed to the function that contains them, named functions via `fn_refs`)
//! * SPEC-EFF-005  calling a function value the pass cannot see (`(s.cb)()`) is effect-open
//! * SPEC-EFF-006  a method call on an unknown receiver performs the union over every same-name candidate
//!
//! The pass reads facts and writes only effect facts; it never touches ownership.

use super::kind::EffectKind;
use super::sinks::{self, Res as SinkRes};
use crate::parser::ast::core::{Decorator, Expression, FunctionDecl};
use crate::parser::ast::types::Type;
use crate::resolve::{CallKind, DefId, DefKind, Reason, Res, ResolvedProgram};
use crate::source_map::Location;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resource {
    /// The effect has no refinement (or it is unknown to the pass).
    None,
    /// A literal path, host, name, program.
    Lit(String),
    /// Computed at run time: only a runtime check can decide.
    Computed,
}

impl Resource {
    pub fn literal(&self) -> Option<&str> {
        match self {
            Resource::Lit(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectRef {
    pub kind: EffectKind,
    pub resource: Resource,
}

/// Where an effect of a function comes from: provenance (rule, call site, callee).
#[derive(Debug, Clone)]
pub struct Origin {
    /// The spec rule that introduced it (`SPEC-EFF-002` sink, `-003` through a callee, `-006` candidate set).
    pub rule: &'static str,
    pub site: Option<Location>,
    /// Display name of the callee at `site` (`fs::read_to_string`, `load`).
    pub callee: String,
    /// Set when the effect came through a callee: follow it to reach the sink.
    pub via: Option<DefId>,
}

#[derive(Debug, Clone)]
pub struct FnEffects {
    pub def: DefId,
    pub path: String,
    pub module: DefId,
    pub location: Option<Location>,
    pub effects: BTreeMap<EffectRef, Origin>,
    /// Function-typed parameters this function calls (directly or by passing them on).
    pub latent: BTreeSet<String>,
    /// Calls function values whose effects are not visible.
    pub open: bool,
    /// `@pure` (empty) or `@uses(...)`; `None` = not declared.
    pub declared: Option<BTreeSet<EffectKind>>,
}

impl FnEffects {
    pub fn kinds(&self) -> BTreeSet<EffectKind> {
        self.effects.keys().map(|e| e.kind.clone()).collect()
    }
}

#[derive(Debug, Default)]
pub struct EffectFacts {
    pub fns: BTreeMap<DefId, FnEffects>,
    /// Call graph (caller -> callees) the pass propagated along.
    pub calls: BTreeMap<DefId, BTreeSet<DefId>>,
}

impl EffectFacts {
    pub fn get(&self, id: DefId) -> Option<&FnEffects> {
        self.fns.get(&id)
    }

    /// Shortest call path from `main` to `f` (function paths, `f` last); just `[f]` when `main` does not reach it.
    pub fn entry_path(&self, f: DefId) -> Vec<String> {
        let root = self.fns.values().find(|x| x.path == "main").map(|x| x.def);
        let alone = || vec![self.fns.get(&f).map(|x| x.path.clone()).unwrap_or_default()];
        let Some(root) = root else { return alone() };
        let mut prev: BTreeMap<DefId, DefId> = BTreeMap::new();
        let mut queue = std::collections::VecDeque::from([root]);
        let mut seen = BTreeSet::from([root]);
        while let Some(cur) = queue.pop_front() {
            if cur == f {
                break;
            }
            for next in self.calls.get(&cur).into_iter().flatten() {
                if seen.insert(*next) {
                    prev.insert(*next, cur);
                    queue.push_back(*next);
                }
            }
        }
        if !seen.contains(&f) {
            return alone();
        }
        let mut path = vec![f];
        while let Some(p) = prev.get(path.last().unwrap()) {
            path.push(*p);
        }
        path.reverse();
        path.iter().filter_map(|d| self.fns.get(d).map(|x| x.path.clone())).collect()
    }

    /// The chain from `f` to the sink that introduced `e`: (function path, call site, callee name) per hop.
    pub fn chain(&self, f: DefId, e: &EffectRef) -> Vec<(String, Option<Location>, String)> {
        let mut out = Vec::new();
        let mut cur = f;
        let mut guard = 0;
        while let Some(fe) = self.fns.get(&cur) {
            let Some(o) = fe.effects.get(e) else { break };
            out.push((fe.path.clone(), o.site.clone(), o.callee.clone()));
            match o.via {
                Some(next) if guard < 64 => {
                    cur = next;
                    guard += 1;
                }
                _ => break,
            }
        }
        out
    }
}

struct Edge {
    callee: DefId,
    site: Option<Location>,
    name: String,
    rule: &'static str,
    /// Positional identifier arguments of the call (for latent propagation).
    idents: Vec<Option<String>>,
}

/// Analyze a resolved program.
pub fn analyze(r: &ResolvedProgram<'_>) -> EffectFacts {
    let defs = r.defs();
    let decls = &r.collected.decls;
    let mut facts = EffectFacts::default();
    for def in defs.iter() {
        if matches!(def.kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. } | DefKind::ExternFn | DefKind::TraitMethod { .. }) {
            facts.fns.insert(
                def.id,
                FnEffects {
                    def: def.id,
                    path: def.path.clone(),
                    module: def.module,
                    location: def.location.clone(),
                    effects: BTreeMap::new(),
                    latent: BTreeSet::new(),
                    open: false,
                    declared: decls.get(&def.id).and_then(|d| declared_effects(d)),
                },
            );
        }
    }
    let mut edges: BTreeMap<DefId, Vec<Edge>> = BTreeMap::new();

    // @effect("x") on a function: it performs x, and calling it is the sink use.
    let mut user_effect_of: BTreeMap<DefId, Vec<EffectKind>> = BTreeMap::new();
    for (id, d) in decls {
        let ks: Vec<EffectKind> = d
            .decorators
            .iter()
            .filter(|dec| dec.name == "effect")
            .filter_map(|dec| dec.arguments.first().and_then(|(_, e)| string_literal(e)))
            .map(EffectKind::User)
            .collect();
        if !ks.is_empty() {
            user_effect_of.insert(*id, ks);
        }
    }
    for (id, ks) in &user_effect_of {
        if let Some(f) = facts.fns.get_mut(id) {
            for k in ks {
                f.effects.insert(
                    EffectRef { kind: k.clone(), resource: Resource::None },
                    Origin { rule: "SPEC-EFF-019", site: f.location.clone(), callee: format!("@effect(\"{k}\")"), via: None },
                );
            }
        }
    }

    for c in &r.calls {
        let Some(owner) = c.enclosing else { continue };
        let site = c.location.clone();
        let mut edge = |callee: DefId, name: &str, rule: &'static str| {
            edges.entry(owner).or_default().push(Edge { callee, site: site.clone(), name: name.to_string(), rule, idents: c.ident_args.clone() });
        };
        let mut direct: Vec<(EffectKind, Resource)> = Vec::new();
        match &c.res {
            Res::Unresolved(Reason::External(path)) | Res::Prim(path) => {
                direct.extend(sink_uses(&full_path(path, &c.callee), c));
            }
            Res::Unresolved(Reason::NotFound(_)) if c.kind == CallKind::Free => {
                direct.extend(sink_uses(&c.callee, c));
            }
            Res::Unresolved(Reason::Dynamic) => {
                if let Some(f) = facts.fns.get_mut(&owner) {
                    f.open = true;
                }
            }
            Res::Unresolved(Reason::GenericDispatch(p)) => {
                let last = p.rsplit("::").next().unwrap_or(p);
                for d in defs.iter().filter(|d| d.name == last && matches!(d.kind, DefKind::Method { .. } | DefKind::AssocFn { .. } | DefKind::TraitMethod { .. })) {
                    edge(d.id, &c.callee, "SPEC-EFF-006");
                }
            }
            Res::Unresolved(_) => {}
            Res::Def(id) => {
                let d = defs.get(*id);
                match d.kind {
                    DefKind::ExternFn => {
                        let declared = decls.get(id).and_then(|x| declared_effects(x));
                        match declared {
                            Some(ks) => direct.extend(ks.into_iter().map(|k| (k, Resource::None))),
                            None => direct.push((EffectKind::Ffi, Resource::Lit(d.name.clone()))),
                        }
                    }
                    DefKind::TraitMethod { .. } => {
                        edge(*id, &c.callee, "SPEC-EFF-003");
                        for m in defs.iter().filter(|m| m.name == d.name && matches!(m.kind, DefKind::Method { .. })) {
                            edge(m.id, &c.callee, "SPEC-EFF-006");
                        }
                    }
                    DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. } => edge(*id, &c.callee, "SPEC-EFF-003"),
                    _ => {}
                }
                if let Some(ks) = user_effect_of.get(id) {
                    direct.extend(ks.iter().map(|k| (k.clone(), Resource::None)));
                }
            }
            Res::MethodCandidates(ids) | Res::Ambiguous(ids) => {
                for id in ids {
                    edge(*id, &c.callee, "SPEC-EFF-006");
                    if let Some(ks) = user_effect_of.get(id) {
                        direct.extend(ks.iter().map(|k| (k.clone(), Resource::None)));
                    }
                }
            }
            Res::Local(name) => {
                // calling a function-typed parameter: latent (SPEC-EFF-004)
                let is_fn_param = decls.get(&owner).is_some_and(|d| d.parameters.iter().any(|p| &p.name == name && is_fn_type(&p.type_)));
                if is_fn_param {
                    if let Some(f) = facts.fns.get_mut(&owner) {
                        f.latent.insert(name.clone());
                    }
                }
            }
        }
        if let Some(f) = facts.fns.get_mut(&owner) {
            for (kind, resource) in direct {
                let callee = c.callee.clone();
                f.effects.entry(EffectRef { kind, resource }).or_insert(Origin { rule: "SPEC-EFF-002", site: c.location.clone(), callee, via: None });
            }
        }
    }

    // a named function used as a value is a potential call at the use site (SPEC-EFF-004)
    for fr in &r.fn_refs {
        if let Some(owner) = fr.enclosing {
            edges.entry(owner).or_default().push(Edge {
                callee: fr.def,
                site: fr.location.clone(),
                name: defs.get(fr.def).name.clone(),
                rule: "SPEC-EFF-004",
                idents: Vec::new(),
            });
        }
    }

    for (owner, es) in &edges {
        facts.calls.entry(*owner).or_default().extend(es.iter().map(|e| e.callee));
    }

    // fixpoint over the call graph (deterministic: BTreeMap order)
    loop {
        let mut changed = false;
        let owners: Vec<DefId> = edges.keys().copied().collect();
        for owner in owners {
            for e in &edges[&owner] {
                let Some(callee) = facts.fns.get(&e.callee) else { continue };
                let (cl_effects, cl_latent, cl_open) = (callee.effects.keys().cloned().collect::<Vec<_>>(), callee.latent.clone(), callee.open);
                let callee_params: Vec<String> = decls.get(&e.callee).map(|d| d.parameters.iter().filter(|p| p.name != "self").map(|p| p.name.clone()).collect()).unwrap_or_default();
                let owner_fn_params: BTreeSet<String> = decls
                    .get(&owner)
                    .map(|d| d.parameters.iter().filter(|p| is_fn_type(&p.type_)).map(|p| p.name.clone()).collect())
                    .unwrap_or_default();
                let Some(f) = facts.fns.get_mut(&owner) else { continue };
                for eff in cl_effects {
                    if !f.effects.contains_key(&eff) {
                        f.effects.insert(eff, Origin { rule: e.rule, site: e.site.clone(), callee: e.name.clone(), via: Some(e.callee) });
                        changed = true;
                    }
                }
                if cl_open && !f.open {
                    f.open = true;
                    changed = true;
                }
                // passing my function parameter to a callee that calls its parameter: I am latent too
                for lp in &cl_latent {
                    if let Some(idx) = callee_params.iter().position(|p| p == lp) {
                        if let Some(Some(arg)) = e.idents.get(idx) {
                            if owner_fn_params.contains(arg) && f.latent.insert(arg.clone()) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    facts
}

/// The resolver reports the module an external callee came from (`std::fs`); the callee's last segment completes it.
fn full_path(external: &str, callee: &str) -> String {
    let last = callee.rsplit("::").next().unwrap_or(callee);
    if external == last || external.ends_with(&format!("::{last}")) {
        external.to_string()
    } else {
        format!("{external}::{last}")
    }
}

/// Effects a call to the sink at `path` performs, with resources read from the call's literal arguments.
fn sink_uses(path: &str, c: &crate::resolve::CallResolution) -> Vec<(EffectKind, Resource)> {
    let Some(uses) = sinks::lookup(path) else { return Vec::new() };
    uses.iter()
        .map(|u| {
            let res = match u.res {
                SinkRes::None => Resource::None,
                SinkRes::Fixed(s) => Resource::Lit(s.to_string()),
                SinkRes::Arg(n) => match c.str_args.get(n) {
                    Some(Some(s)) => Resource::Lit(s.clone()),
                    _ => Resource::Computed,
                },
                SinkRes::CmdArg(n) => match c.str_args.get(n) {
                    Some(Some(s)) => Resource::Lit(super::scope_match::program_name(s)),
                    _ => Resource::Computed,
                },
            };
            (u.effect.clone(), res)
        })
        .collect()
}

fn is_fn_type(t: &Type) -> bool {
    match t {
        Type::FunctionPointer { .. } => true,
        Type::Reference(inner) | Type::MutableReference(inner) => is_fn_type(inner),
        Type::ImplTrait(n) | Type::TraitObject(n) | Type::Custom(n) | Type::Generic(n) => {
            n.starts_with("Fn") || n.starts_with("FnMut") || n.starts_with("FnOnce")
        }
        Type::Parameterized(n, _) => n.starts_with("Fn"),
        _ => false,
    }
}

fn string_literal(e: &Expression<'_>) -> Option<String> {
    use crate::parser::ast::literals::Literal;
    match e {
        Expression::Literal { value: Literal::String(s), .. } => Some(s.clone()),
        _ => None,
    }
}

/// Dotted name of a `fs.read` / `process` expression.
fn dotted(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, field, .. } => Some(format!("{}.{}", dotted(object)?, field)),
        Expression::MethodCall { object, method, .. } => Some(format!("{}.{}", dotted(object)?, method)),
        _ => None,
    }
}

/// `@pure` -> `Some({})`, `@uses(fs.read, net.connect("h"))` -> `Some({fs.read, net.connect})`, otherwise `None`.
pub fn declared_effects(d: &FunctionDecl<'_>) -> Option<BTreeSet<EffectKind>> {
    let mut out: Option<BTreeSet<EffectKind>> = None;
    for dec in &d.decorators {
        out = merge(out, decorator_effects(dec));
    }
    out
}

fn merge(a: Option<BTreeSet<EffectKind>>, b: Option<BTreeSet<EffectKind>>) -> Option<BTreeSet<EffectKind>> {
    match (a, b) {
        (None, x) | (x, None) => x,
        (Some(mut x), Some(y)) => {
            x.extend(y);
            Some(x)
        }
    }
}

fn decorator_effects(dec: &Decorator<'_>) -> Option<BTreeSet<EffectKind>> {
    match dec.name.as_str() {
        "pure" => Some(BTreeSet::new()),
        "uses" => Some(
            dec.arguments
                .iter()
                .filter_map(|(_, e)| dotted(e))
                .map(|n| EffectKind::parse_builtin(&n).unwrap_or(EffectKind::User(n)))
                .collect(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{resolve_program, ParsedFile};

    fn facts_of(src: &str) -> (EffectFacts, Vec<(String, String)>) {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, "t.wj".into(), src.into());
        let program: &'static crate::parser::ast::core::Program<'static> = Box::leak(Box::new(parser.parse().expect("parse")));
        let files = [ParsedFile::new(vec![], "t.wj".into(), program)];
        let r = resolve_program(&files);
        let facts = analyze(&r);
        let lines = super::super::report::compact_lines(&r, &facts);
        (facts, lines)
    }

    fn line(lines: &[(String, String)], f: &str) -> String {
        lines.iter().find(|(n, _)| n == f).map(|(_, l)| l.clone()).unwrap_or_else(|| format!("<missing {f}>"))
    }

    #[test]
    fn sinks_and_propagation() {
        let (_, l) = facts_of("use std::fs\nfn load() -> string { fs::read_to_string(\"c\").unwrap_or(\"\") }\nfn top() -> string { load() }\nfn pure_fn(a: int) -> int { a + 1 }\n");
        assert_eq!(line(&l, "load"), "fs.read");
        assert_eq!(line(&l, "top"), "fs.read");
        assert_eq!(line(&l, "pure_fn"), "pure");
    }

    #[test]
    fn recursion_terminates_and_keeps_effects() {
        let (_, l) = facts_of("use std::random\nfn a(n: int) -> int { if n == 0 { random::range(0, 2) } else { b(n - 1) } }\nfn b(n: int) -> int { a(n) }\n");
        assert_eq!(line(&l, "a"), "random");
        assert_eq!(line(&l, "b"), "random");
    }

    #[test]
    fn latent_parameters_and_closures() {
        let (_, l) = facts_of(
            "use std::random\nfn apply(f: fn(int) -> int, x: int) -> int { f(x) }\nfn twice(f: fn(int) -> int, x: int) -> int { apply(f, apply(f, x)) }\nfn main() { let a = apply(|x| x + random::range(0, 1), 1) }\n",
        );
        assert_eq!(line(&l, "apply"), "pure + f");
        assert_eq!(line(&l, "twice"), "pure + f");
        assert_eq!(line(&l, "main"), "random");
    }

    #[test]
    fn named_function_passed_as_value_counts() {
        let (_, l) = facts_of("use std::fs\nfn load(p: string) -> string { fs::read_to_string(p).unwrap_or(\"\") }\nfn apply(f: fn(string) -> string, x: string) -> string { f(x) }\nfn main() { let s = apply(load, \"a\") }\n");
        assert_eq!(line(&l, "main"), "fs.read");
    }

    #[test]
    fn dynamic_calls_are_open() {
        let (_, l) = facts_of("struct Job { run: fn() -> int }\nfn run_job(j: Job) -> int { (j.run)() }\n");
        assert_eq!(line(&l, "run_job"), "pure + ?");
    }

    #[test]
    fn literal_resources_are_recorded() {
        let (f, _) = facts_of("use std::fs\nfn a() { fs::write(\"out.txt\", \"x\").unwrap() }\nfn b(p: string) { fs::write(p, \"x\").unwrap() }\n");
        let res = |name: &str| {
            f.fns.values().find(|x| x.path == name).unwrap().effects.keys().map(|e| e.resource.clone()).collect::<Vec<_>>()
        };
        assert_eq!(res("a"), [Resource::Lit("out.txt".into())]);
        assert_eq!(res("b"), [Resource::Computed]);
    }

    #[test]
    fn declared_effects_parse() {
        let (f, _) = facts_of("@pure\nfn p() -> int { 1 }\n@uses(fs.read, net.connect(\"h\"))\nfn u() {}\nfn n() {}\n");
        let d = |name: &str| f.fns.values().find(|x| x.path == name).unwrap().declared.clone();
        assert_eq!(d("p"), Some(BTreeSet::new()));
        assert_eq!(d("u"), Some([EffectKind::FsRead, EffectKind::NetConnect].into_iter().collect()));
        assert_eq!(d("n"), None);
    }
}

