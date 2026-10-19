// wj explain-effects - report the effects each function performs, with the call chain to every sink,
// the rule that introduced it and how it is checked against the project's [capabilities] (ADR-0021).
//
// Reporting only: the facts come from `ir::effect_facts`; nothing computed here flows back.

use crate::check::effects::{analyze_project, parse_app_inputs};
use crate::ir::effect_facts::report;
use anyhow::{bail, Result};
use std::path::Path;

pub fn execute(path: &Path, function: Option<&str>, json: bool) -> Result<()> {
    let inputs = parse_app_inputs(path)?;
    let a = analyze_project(path, inputs, false);
    if let Some(name) = function {
        let matches = |p: &str| p == name || p.ends_with(&format!("::{name}"));
        if !a.facts.fns.values().any(|f| matches(&f.path) && a.resolved.collected.decls.contains_key(&f.def)) {
            bail!("no function named `{name}`");
        }
    }
    let grant = a.grants.app.as_ref();
    if json {
        println!("{}", serde_json::to_string_pretty(&report::to_json(&a.resolved, &a.facts, grant))?);
    } else {
        print!("{}", report::to_text(&a.resolved, &a.facts, function, grant));
    }
    Ok(())
}
