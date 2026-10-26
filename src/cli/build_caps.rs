// Compile-time capability enforcement for `wj build` (ADR-0021 stage E2).
//
// With a `[capabilities]` table (or a `.wj-capabilities.lock`) in wj.toml the build fails on the same findings
// `wj check` reports (WJ0040-WJ0044), and a compiled program carries a run-time guard (the installed grant) only
// when some sink call cannot be proven inside the grant here. Zero guards otherwise.

use crate::check::effects::{analyze_project, parse_app_inputs};
use crate::check::{self, CheckOptions};
use crate::diagnostics::render;
use crate::ir::effect_facts::install::{compiled_dependency_gaps, needs_runtime_guards, render_install, render_install_go, render_install_js};
use anyhow::{bail, Result};
use std::path::Path;

const EFFECT_CODES: [&str; 5] = ["WJ0040", "WJ0041", "WJ0042", "WJ0043", "WJ0044"];

/// Which backend the install statement is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Rust,
    JavaScript,
    Go,
}

/// Check the project's effects. Returns the `caps::install` statement for the generated `main` when one is needed.
pub fn prepare(path: &Path, library: bool) -> Result<Option<String>> {
    prepare_for(path, library, Flavor::Rust)
}

pub fn prepare_for(path: &Path, library: bool, flavor: Flavor) -> Result<Option<String>> {
    let Ok(inputs) = parse_app_inputs(path) else { return Ok(None) }; // not a checkable project: the build reports why
    let a = analyze_project(path, inputs, false);
    let durable = crate::ir::effect_facts::durable::analyze(&a.resolved, &a.facts);
    if !durable.functions.is_empty() {
        // Journaling needs the step wrapper in every backend; until the compiled runtimes have it, a build that silently
        // dropped the journal would lose progress on a crash, so it stops instead (ADR-0021 E4).
        bail!(
            "`durable fn` ({}) runs only in the interpreter in this release: compiled backends do not journal steps yet, and a build without the journal would lose progress on a crash\n  = help: run it with `wj run --interpret`, or remove `durable`",
            durable.functions.join(", ")
        );
    }
    if a.grants.app.is_none() && !a.grants.enforce_deps {
        return Ok(None); // no [capabilities], no lock file: nothing to enforce
    }
    let checked = check::check_path(path, &CheckOptions::default())?;
    let source_of = |f: &str| checked.sources.get(f).cloned();
    let mut errors = 0;
    for d in checked.report.diagnostics.iter().filter(|d| d.is_error() && EFFECT_CODES.contains(&d.code.as_str())) {
        eprintln!("{}", render::render(d, &source_of));
        errors += 1;
    }
    for (pkg, kind) in compiled_dependency_gaps(&a.resolved, &a.facts, &a.grants, &a.dep_names) {
        eprintln!(
            "error[WJ0041]: dependency `{pkg}` uses a computed `{kind}` resource, and a compiled program enforces one grant at run time\n  = help: give [capabilities.dependencies.{pkg}] the same `{kind}` entries as [capabilities], or pass the dependency a capability value (`std::cap`)"
        );
        errors += 1;
    }
    if errors > 0 {
        bail!("{errors} capability error(s): the build stops before generating code");
    }
    if library {
        return Ok(None);
    }
    let Some(app) = a.grants.app.as_ref() else { return Ok(None) };
    if needs_runtime_guards(&a.resolved, &a.facts, &a.grants, &a.dep_names) {
        // relative patterns anchor at the program's working directory at run time, like the paths it opens
        let root = Path::new(".");
        Ok(Some(match flavor {
            Flavor::Rust => render_install(app, root),
            Flavor::JavaScript => render_install_js(app, root),
            Flavor::Go => render_install_go(app, root),
        }))
    } else {
        Ok(None)
    }
}
