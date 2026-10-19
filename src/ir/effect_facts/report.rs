//! `wj explain-effects`: text and JSON views of the effect facts (reporting only; nothing flows back).

use super::analysis::{EffectFacts, EffectRef, FnEffects, Resource};
use super::grant::Grant;
use crate::resolve::{DefKind, ResolvedProgram};
use serde_json::{json, Value};

fn reported(r: &ResolvedProgram<'_>, f: &FnEffects) -> bool {
    matches!(r.defs().get(f.def).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }) && r.collected.decls.contains_key(&f.def)
}

/// `(function path, "fs.read, random + f")` per function with a body, in path order.
/// Effect kinds are sorted by name; `pure` when none; `+ f` for called function parameters; `+ ?` for open calls.
pub fn compact_lines(r: &ResolvedProgram<'_>, facts: &EffectFacts) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = facts.fns.values().filter(|f| reported(r, f)).map(|f| (f.path.clone(), compact(f))).collect();
    out.sort();
    out
}

fn compact(f: &FnEffects) -> String {
    let mut names: Vec<String> = f.kinds().iter().map(|k| k.name()).collect();
    names.sort();
    let mut s = if names.is_empty() { "pure".to_string() } else { names.join(", ") };
    let mut tail: Vec<String> = f.latent.iter().cloned().collect();
    if f.open {
        tail.push("?".to_string());
    }
    if !tail.is_empty() {
        s.push_str(" + ");
        s.push_str(&tail.join(", "));
    }
    s
}

fn resource_text(e: &EffectRef) -> String {
    match &e.resource {
        Resource::None => e.kind.name(),
        Resource::Lit(s) => format!("{}(\"{}\")", e.kind, s),
        Resource::Computed => format!("{}(<computed>)", e.kind),
    }
}

/// How a use is checked against a grant: `proven` (literal inside the grant), `runtime guard` (computed
/// resource, granted for some), `denied`, or `granted` when no grant was given.
fn placement(e: &EffectRef, grant: Option<&Grant>) -> &'static str {
    let Some(g) = grant else { return "unchecked" };
    match &e.resource {
        Resource::Computed => match g.granted_at_all(&e.kind) {
            Ok(()) if matches!(g.scope(&e.kind), Some(super::grant::Scope::Any)) || e.kind.default_granted() => "proven",
            Ok(()) => "runtime guard",
            Err(_) => "denied",
        },
        Resource::Lit(s) => match g.check(&e.kind, Some(s)) {
            Ok(()) => "proven",
            Err(_) => "denied",
        },
        Resource::None => match g.check(&e.kind, None) {
            Ok(()) => "proven",
            Err(_) => "denied",
        },
    }
}

fn site_text(l: &Option<crate::source_map::Location>) -> String {
    match l {
        Some(l) if !l.file.as_os_str().is_empty() && l.line < 1_000_000 => format!("{}:{}:{}", l.file.display(), l.line, l.column),
        _ => String::from("<interpolation>"),
    }
}

/// Human report: the compact line per function, then each effect with its rule, chain and grant placement.
pub fn to_text(r: &ResolvedProgram<'_>, facts: &EffectFacts, only: Option<&str>, grant: Option<&Grant>) -> String {
    let mut out = String::new();
    let mut fns: Vec<&FnEffects> = facts.fns.values().filter(|f| reported(r, f)).collect();
    fns.sort_by(|a, b| a.path.cmp(&b.path));
    for f in fns {
        if only.is_some_and(|o| f.path != o && !f.path.ends_with(&format!("::{o}"))) {
            continue;
        }
        out.push_str(&format!("{}: {}\n", f.path, compact(f)));
        for (e, o) in &f.effects {
            let chain = facts.chain(f.def, e);
            let hops: Vec<String> = chain.iter().map(|(p, _, _)| p.clone()).chain(chain.last().map(|(_, _, c)| c.clone())).collect();
            let at = chain.last().map(|(_, s, _)| site_text(s)).unwrap_or_default();
            let place = placement(e, grant);
            out.push_str(&format!("    {}  [{}]  {}  at {}  ({})\n", resource_text(e), o.rule, hops.join(" -> "), at, place));
        }
    }
    out
}

pub fn to_json(r: &ResolvedProgram<'_>, facts: &EffectFacts, grant: Option<&Grant>) -> Value {
    let mut fns: Vec<&FnEffects> = facts.fns.values().filter(|f| reported(r, f)).collect();
    fns.sort_by(|a, b| a.path.cmp(&b.path));
    let items: Vec<Value> = fns
        .iter()
        .map(|f| {
            let effects: Vec<Value> = f
                .effects
                .iter()
                .map(|(e, o)| {
                    let chain: Vec<Value> = facts
                        .chain(f.def, e)
                        .into_iter()
                        .map(|(p, s, c)| json!({"function": p, "callee": c, "at": site_text(&s)}))
                        .collect();
                    json!({
                        "effect": e.kind.name(),
                        "resource": match &e.resource { Resource::None => Value::Null, Resource::Lit(s) => json!(s), Resource::Computed => json!("<computed>") },
                        "rule": o.rule,
                        "chain": chain,
                        "check": placement(e, grant),
                    })
                })
                .collect();
            json!({
                "function": f.path,
                "summary": compact(f),
                "pure": f.effects.is_empty(),
                "latent": f.latent,
                "open": f.open,
                "effects": effects,
            })
        })
        .collect();
    json!({ "functions": items })
}
