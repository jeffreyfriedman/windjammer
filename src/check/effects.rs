//! Effect findings for `wj check` (ADR-0021): WJ0040 not granted, WJ0041 outside the grant, WJ0042 undeclared,
//! WJ0043 policy, WJ0044 extern without declared effects. The facts come from `ir::effect_facts`; this file
//! only turns findings into diagnostics (spans, help, fixes) and reads the manifest.

use super::dep_sources;
use super::front::Parsed;
use super::locate::Locator;
use crate::diagnostics::{Diagnostic, SourceIndex, Span};
use crate::ir::effect_facts::policy::{self, PolicyFinding};
use crate::ir::effect_facts::violations::{self, Finding};
use crate::ir::effect_facts::analysis::EffectFacts;
use crate::ir::effect_facts::lock::LOCK_FILE;
use crate::ir::effect_facts::{analyze, CapLock, PackageGrants};
use crate::resolve::{self, ParsedFile};
use crate::source_map::Location;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Nearest `wj.toml` at or above `root` (a project directory or a single file).
pub(super) fn manifest_path(root: &Path) -> Option<PathBuf> {
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

/// The directory module paths are relative to: `<project>/src` when the project has one, else the checked directory.
pub fn module_base(root: &Path) -> PathBuf {
    let dir = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    match manifest_path(dir) {
        Some(m) => {
            let src = m.parent().unwrap_or(dir).join("src");
            if src.is_dir() && dir.starts_with(src.parent().unwrap_or(dir)) && (dir == src || dir.starts_with(&src) || dir == src.parent().unwrap_or(dir)) {
                src
            } else {
                dir.to_path_buf()
            }
        }
        None => dir.to_path_buf(),
    }
}

/// Everything the effect pass computed for a project: facts, per-package grants, policy.
pub struct Analysis<'a> {
    pub resolved: resolve::ResolvedProgram<'a>,
    pub facts: EffectFacts,
    pub dep_names: Vec<String>,
    pub dep_revs: std::collections::BTreeMap<String, Option<String>>,
    pub grants: PackageGrants,
    pub policy: Option<policy::Policy>,
    pub manifest_dir: PathBuf,
    pub lock: Option<CapLock>,
    /// Manifest problems (code, message), reported once.
    pub manifest_errors: Vec<(&'static str, String)>,
}

/// Run the effect pass over `app` files (module paths already relative to the source root) and, when the manifest
/// enforces dependencies (or `force_deps`), the project's Windjammer dependencies too.
pub fn analyze_project<'a>(root: &Path, mut inputs: Vec<ParsedFile<'a>>, force_deps: bool) -> Analysis<'a> {
    let manifest = manifest_path(root);
    let text = manifest.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let dir = manifest.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_else(|| root.to_path_buf());
    let mut manifest_errors = Vec::new();
    let lock = match CapLock::load(&dir) {
        Ok(l) => l,
        Err(m) => {
            manifest_errors.push(("WJ0040", m));
            None
        }
    };
    let grants = PackageGrants::from_manifest_text(&text, &dir, lock.as_ref()).unwrap_or_else(|m| {
        manifest_errors.push(("WJ0040", m));
        PackageGrants::from_manifest_text("", &dir, None).expect("empty manifest")
    });
    let policy = policy::policy_from_manifest_text(&text).unwrap_or_else(|m| {
        manifest_errors.push(("WJ0043", m));
        None
    });
    let mut dep_names = Vec::new();
    let mut dep_revs = std::collections::BTreeMap::new();
    if grants.enforce_deps || force_deps {
        for d in dep_sources::load(&dir) {
            dep_names.push(d.name.clone());
            dep_revs.insert(d.name.clone(), d.rev.clone());
            for f in d.files {
                inputs.push(ParsedFile::new(f.module_path, f.file, f.program));
            }
        }
    }
    let resolved = resolve::resolve_program(&inputs);
    let facts = analyze(&resolved);
    Analysis { resolved, facts, dep_names, dep_revs, grants, policy, manifest_dir: dir, lock, manifest_errors }
}

/// Parse the application's `.wj` files under `path` (a file or a project directory) for the effect pass.
/// Module paths are relative to the source root; a single file is its own root module.
pub fn parse_app_inputs(path: &Path) -> anyhow::Result<Vec<ParsedFile<'static>>> {
    let files = super::files::collect(path)?;
    if files.is_empty() {
        anyhow::bail!("no .wj files under {}", path.display());
    }
    let base = module_base(path);
    let single_root = path.is_file() && base == path.parent().unwrap_or(path);
    let mut out = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).map_err(|e| anyhow::anyhow!("reading {}: {e}", f.display()))?;
        let mut lexer = crate::lexer::Lexer::new(&src);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, f.display().to_string(), src.clone());
        let program = parser.parse().map_err(|e| anyhow::anyhow!("{}: {e}", f.display()))?;
        let program: &'static crate::parser::ast::core::Program<'static> = Box::leak(Box::new(program));
        let module_path = if single_root { Vec::new() } else { resolve::module_path_from_file(&base, f) };
        out.push(ParsedFile::new(module_path, f.clone(), program));
    }
    Ok(out)
}

pub fn run(root: &Path, parsed: &[&Parsed], by_file: &HashMap<String, &Parsed>, out: &mut Vec<Diagnostic>) {
    let base = module_base(root);
    let base = base.as_path();
    let inputs: Vec<ParsedFile<'_>> = parsed
        .iter()
        .map(|p| {
            let path = Path::new(&p.file);
            ParsedFile::new(resolve::module_path_from_file(base, path), path.to_path_buf(), &p.program)
        })
        .collect();
    let a = analyze_project(root, inputs, false);
    for (code, m) in &a.manifest_errors {
        out.push(Diagnostic::error(code, m.clone()).with_help("fix wj.toml [capabilities] / [policy] / .wj-capabilities.lock"));
    }
    let (r, facts) = (&a.resolved, &a.facts);
    let span_for = |loc: &Option<Location>, word: &str| -> Option<Span> {
        let l = loc.as_ref()?;
        let key = l.file.to_string_lossy();
        match by_file.get(key.as_ref()) {
            Some(p) => Locator::new(&SourceIndex::new(p.file.clone(), &p.text)).word(&Some(l.clone()), word),
            // a dependency's file: not part of the checked set, read it for the span
            None => {
                let text = std::fs::read_to_string(&l.file).ok()?;
                Locator::new(&SourceIndex::new(key.to_string(), &text)).word(&Some(l.clone()), word)
            }
        }
    };
    let last = |s: &str| s.rsplit("::").next().unwrap_or(s).to_string();
    for f in violations::check(r, facts, &a.grants, &a.dep_names) {
        match f {
            Finding::NotGranted { package, function, effect, denial, entry, sink, site } => {
                let code = if denial.outside_scope { "WJ0041" } else { "WJ0040" };
                let msg = match (package.is_empty(), denial.outside_scope) {
                    (true, true) => format!("`{}` is outside the grant", denial.subject()),
                    (true, false) => format!("effect `{}` is not granted", effect.kind),
                    (false, true) => format!("`{}` is outside the grant of dependency `{package}`", denial.subject()),
                    (false, false) => format!("effect `{}` is not granted to dependency `{package}`", effect.kind),
                };
                let mut d = Diagnostic::error(code, msg);
                if let Some(sp) = span_for(&site, &last(&sink)) {
                    d = d.at(sp.with_label(format!("`{function}` performs {}", denial.subject())));
                } else if let Some(l) = &site {
                    d = d.with_note(format!("at {}:{}", l.file.display(), l.line));
                }
                let mut chain = entry.clone();
                if chain.last() != Some(&function) {
                    chain.push(function.clone());
                }
                chain.push(sink.clone());
                d = d.with_note(format!("reached through: {}", chain.join(" -> "))).with_note(format!("grant in force: {}", denial.grant));
                let help = if package.is_empty() {
                    denial.fix.clone()
                } else {
                    let entry_line = match &denial.resource {
                        Some(r) => format!("{} = [\"{r}\"]", effect.kind),
                        None => format!("{} = true", effect.kind),
                    };
                    format!(
                        "review the dependency, then record what it does with `wj caps approve` ({LOCK_FILE}), or add `{entry_line}` under [capabilities.dependencies.{package}] in wj.toml"
                    )
                };
                out.push(d.with_help(help));
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
                out.push(d.with_help(format!("add `ffi = [\"{callee}\"]` under [capabilities] in wj.toml, or declare what the extern does with `@uses(..)` / `@pure`")));
            }
        }
    }
    if let Some(p) = &a.policy {
        for f in policy::check(r, facts, p) {
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
