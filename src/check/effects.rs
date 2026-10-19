//! Effect findings for `wj check` (ADR-0021): WJ0040 not granted, WJ0041 outside the grant, WJ0042 undeclared,
//! WJ0043 policy, WJ0044 extern without declared effects. The facts come from `ir::effect_facts`; this file
//! only turns findings into diagnostics (spans, help, fixes) and reads the manifest.

use super::front::Parsed;
use super::locate::Locator;
use crate::diagnostics::{Diagnostic, SourceIndex, Span};
use crate::ir::effect_facts::policy::{self, PolicyFinding};
use crate::ir::effect_facts::violations::{self, Finding};
use crate::ir::effect_facts::{analyze, grant_from_manifest_text, Grant};
use crate::resolve::{self, ParsedFile};
use crate::source_map::Location;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Nearest `wj.toml` at or above `root` (a project directory or a single file).
fn manifest_path(root: &Path) -> Option<PathBuf> {
    let mut cur = if root.is_dir() { Some(root) } else { root.parent() };
    while let Some(d) = cur {
        let p = d.join("wj.toml");
        if p.is_file() {
            return Some(p);
        }
        cur = d.parent();
    }
    None
}

pub fn run(root: &Path, parsed: &[&Parsed], by_file: &HashMap<String, &Parsed>, out: &mut Vec<Diagnostic>) {
    let manifest = manifest_path(root);
    let text = manifest.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let dir = manifest.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_else(|| root.to_path_buf());
    let grant: Option<Grant> = match grant_from_manifest_text(&text, &dir) {
        Ok(g) => g,
        Err(m) => {
            out.push(Diagnostic::error("WJ0040", m).with_help("fix the `[capabilities]` table in wj.toml"));
            None
        }
    };
    let policy = match policy::policy_from_manifest_text(&text) {
        Ok(p) => p,
        Err(m) => {
            out.push(Diagnostic::error("WJ0043", m));
            None
        }
    };
    let base = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    let inputs: Vec<ParsedFile<'_>> = parsed
        .iter()
        .map(|p| {
            let path = Path::new(&p.file);
            ParsedFile::new(resolve::module_path_from_file(base, path), path.to_path_buf(), &p.program)
        })
        .collect();
    let r = resolve::resolve_program(&inputs);
    let facts = analyze(&r);
    let span_for = |loc: &Option<Location>, word: &str| -> Option<Span> {
        let l = loc.as_ref()?;
        let p = by_file.get(l.file.to_string_lossy().as_ref())?;
        let ix = SourceIndex::new(p.file.clone(), &p.text);
        Locator::new(&ix).word(&Some(l.clone()), word)
    };
    let last = |s: &str| s.rsplit("::").next().unwrap_or(s).to_string();
    for f in violations::check(&r, &facts, grant.as_ref()) {
        match f {
            Finding::NotGranted { function, effect, denial, entry, sink, site } => {
                let code = if denial.outside_scope { "WJ0041" } else { "WJ0040" };
                let msg = if denial.outside_scope {
                    format!("`{}` is outside the grant", denial.subject())
                } else {
                    format!("effect `{}` is not granted", effect.kind)
                };
                let mut d = Diagnostic::error(code, msg);
                if let Some(sp) = span_for(&site, &last(&sink)) {
                    d = d.at(sp.with_label(format!("`{function}` performs {}", denial.subject())));
                }
                let mut chain = entry.clone();
                if chain.last() != Some(&function) {
                    chain.push(function.clone());
                }
                chain.push(sink.clone());
                d = d.with_note(format!("reached through: {}", chain.join(" -> "))).with_note(format!("grant in force: {}", denial.grant));
                out.push(d.with_help(denial.fix.clone()));
            }
            Finding::Undeclared { function, effect, chain, site, def_site: _ } => {
                let mut d = Diagnostic::error("WJ0042", format!("`{function}` performs `{effect}`, which its @uses/@pure declaration does not allow"));
                if let Some(sp) = span_for(&site, &chain.last().map(|c| last(&c.2)).unwrap_or_default()) {
                    d = d.at(sp.with_label(format!("performs {effect}")));
                }
                let via: Vec<String> = chain.iter().map(|c| c.0.clone()).chain(chain.last().map(|c| c.2.clone())).collect();
                out.push(d.with_note(format!("reached through: {}", via.join(" -> "))).with_help(format!("add `{effect}` to @uses(..), or remove the call")));
            }
            Finding::UndeclaredExtern { function, callee, site } => {
                let mut d = Diagnostic::error("WJ0044", format!("extern `{callee}` has no declared effects (called from `{function}`)"));
                if let Some(sp) = span_for(&site, &callee) {
                    d = d.at(sp.with_label("extern call"));
                }
                out.push(d.with_help(format!("declare what it does: `@uses(fs.read)` or `@pure` on the extern fn, or grant `ffi = [\"{callee}\"]` in wj.toml [capabilities]")));
            }
        }
    }
    if let Some(p) = policy {
        for f in policy::check(&r, &facts, &p) {
            match f {
                PolicyFinding::Forbidden { module, function, effect, entry, callee, site } => {
                    let mut d = Diagnostic::error("WJ0043", format!("module `{module}` may not perform `{effect}` (`{function}` does, directly or through a callee)"));
                    if let Some(sp) = span_for(&site, &last(&callee)) {
                        d = d.at(sp.with_label(format!("performs {effect}")));
                    }
                    out.push(d.with_note(format!("reached through: {}", entry.join(" -> "))).with_help(format!("remove the call, or relax `forbid.{module}` in wj.toml [policy]")));
                }
                PolicyFinding::NotVia { effect, function, module, allowed, callee, site } => {
                    let allowed = allowed.join("`, `");
                    let mut d = Diagnostic::error("WJ0043", format!("`{effect}` may only be performed via `{allowed}` (`{function}` in module `{}` does it)", if module.is_empty() { "<root>" } else { &module }));
                    if let Some(sp) = span_for(&site, &last(&callee)) {
                        d = d.at(sp.with_label(format!("performs {effect} outside `{allowed}`")));
                    }
                    out.push(d.with_help(format!("call a function of `{allowed}` instead")));
                }
            }
        }
    }
}
