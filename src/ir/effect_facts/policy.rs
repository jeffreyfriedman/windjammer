//! Application policy over effect facts (`wj.toml [policy]`, ADR-0021 section 7):
//!
//! ```toml
//! [policy]
//! forbid.ui = ["net"]                          # module `ui` (and below) may not perform net.* even transitively
//! "ledger.post" = { only_via = ["billing"] }   # direct calls of the effect only from module `billing` (and below)
//! ```

use super::analysis::EffectFacts;
use super::kind::EffectKind;
use crate::resolve::{DefKind, ResolvedProgram};
use crate::source_map::Location;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Policy {
    /// module path -> effect names/families it may not perform
    pub forbid: BTreeMap<String, Vec<String>>,
    /// effect name -> modules that may perform it directly
    pub only_via: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum PolicyFinding {
    Forbidden { module: String, function: String, effect: EffectKind, entry: Vec<String>, callee: String, site: Option<Location> },
    NotVia { effect: EffectKind, function: String, module: String, allowed: Vec<String>, callee: String, site: Option<Location> },
}

pub fn policy_from_manifest_text(text: &str) -> Result<Option<Policy>, String> {
    let doc: toml::Value = toml::from_str(text).map_err(|e| format!("wj.toml: {e}"))?;
    let Some(t) = doc.get("policy") else { return Ok(None) };
    let t = t.as_table().ok_or("wj.toml: [policy] must be a table")?;
    let mut p = Policy::default();
    for (k, v) in t {
        if k == "forbid" {
            for (module, list) in v.as_table().ok_or("[policy] forbid must be a table of module = [effects]")? {
                p.forbid.insert(module.clone(), string_list(list).ok_or_else(|| format!("[policy] forbid.{module}: expected a list of strings"))?);
            }
        } else if let Some(via) = v.get("only_via") {
            p.only_via.insert(k.clone(), string_list(via).ok_or_else(|| format!("[policy] {k}.only_via: expected a list of strings"))?);
        } else {
            return Err(format!("[policy]: unknown key `{k}` (known: forbid.<module>, \"<effect>\" = {{ only_via = [..] }})"));
        }
    }
    Ok(Some(p))
}

fn string_list(v: &toml::Value) -> Option<Vec<String>> {
    v.as_array()?.iter().map(|x| x.as_str().map(str::to_string)).collect()
}

fn in_module(path: &str, module: &str) -> bool {
    path == module || path.starts_with(&format!("{module}::"))
}

fn effect_matches(kind: &EffectKind, pat: &str) -> bool {
    let n = kind.name();
    n == pat || n.starts_with(&format!("{pat}."))
}

pub fn check(r: &ResolvedProgram<'_>, facts: &EffectFacts, policy: &Policy) -> Vec<PolicyFinding> {
    let mut out = Vec::new();
    for f in facts.fns.values() {
        if !matches!(r.defs().get(f.def).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }) || !r.collected.decls.contains_key(&f.def) {
            continue;
        }
        let module = r.defs().get(f.module).path.clone();
        for (e, o) in &f.effects {
            for (m, pats) in &policy.forbid {
                if in_module(&module, m) && pats.iter().any(|p| effect_matches(&e.kind, p)) {
                    out.push(PolicyFinding::Forbidden { module: m.clone(), function: f.path.clone(), effect: e.kind.clone(), entry: facts.entry_path(f.def), callee: o.callee.clone(), site: o.site.clone() });
                }
            }
            if o.via.is_none() && o.rule != "SPEC-EFF-019" {
                if let Some(allowed) = policy.only_via.get(&e.kind.name()) {
                    if !allowed.iter().any(|a| in_module(&module, a)) {
                        out.push(PolicyFinding::NotVia { effect: e.kind.clone(), function: f.path.clone(), module: module.clone(), allowed: allowed.clone(), callee: o.callee.clone(), site: o.site.clone() });
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forbid_and_only_via() {
        let p = policy_from_manifest_text("[policy]\nforbid.ui = [\"net\"]\n\"ledger.post\" = { only_via = [\"billing\"] }\n").unwrap().unwrap();
        assert_eq!(p.forbid["ui"], ["net"]);
        assert_eq!(p.only_via["ledger.post"], ["billing"]);
        assert!(policy_from_manifest_text("[policy]\nfrobid.ui = []\n").is_err());
        assert!(policy_from_manifest_text("[package]\nname = \"x\"\n").unwrap().is_none());
    }

    #[test]
    fn families_and_module_prefixes() {
        assert!(effect_matches(&EffectKind::NetConnect, "net"));
        assert!(effect_matches(&EffectKind::NetConnect, "net.connect"));
        assert!(!effect_matches(&EffectKind::Process, "net"));
        assert!(in_module("ui::widgets", "ui"));
        assert!(!in_module("uix", "ui"));
    }
}
