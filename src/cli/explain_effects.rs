// wj explain-effects - report the effects each function performs, with the call chain to every sink,
// the rule that introduced it and how it is checked against the project's [capabilities] (ADR-0021).
//
// Reporting only: the facts come from `ir::effect_facts`; nothing computed here flows back.

use crate::ir::effect_facts::{analyze, grant_from_manifest_text, report};
use crate::resolve::{self, ParsedFile};
use anyhow::{bail, Context, Result};
use std::path::Path;

pub fn execute(path: &Path, function: Option<&str>, json: bool) -> Result<()> {
    let files = crate::check::files::collect(path)?;
    if files.is_empty() {
        bail!("no .wj files under {}", path.display());
    }
    let base = if path.is_file() { path.parent().unwrap_or(path) } else { path };
    let mut programs: Vec<(Vec<String>, std::path::PathBuf, &'static crate::parser::ast::core::Program<'static>)> = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).with_context(|| format!("reading {}", f.display()))?;
        let mut lexer = crate::lexer::Lexer::new(&src);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, f.display().to_string(), src.clone());
        let program = parser.parse().map_err(|e| anyhow::anyhow!("{}: {e}", f.display()))?;
        programs.push((if path.is_file() { Vec::new() } else { resolve::module_path_from_file(base, f) }, f.clone(), Box::leak(Box::new(program))));
    }
    let inputs: Vec<ParsedFile<'_>> = programs.iter().map(|(m, f, p)| ParsedFile::new(m.clone(), f.clone(), p)).collect();
    let r = resolve::resolve_program(&inputs);
    let facts = analyze(&r);
    if let Some(name) = function {
        let matches = |p: &str| p == name || p.ends_with(&format!("::{name}"));
        if !facts.fns.values().any(|f| matches(&f.path) && r.collected.decls.contains_key(&f.def)) {
            bail!("no function named `{name}`");
        }
    }
    let grant = nearest_manifest(base).and_then(|(dir, text)| grant_from_manifest_text(&text, &dir).ok().flatten());
    if json {
        println!("{}", serde_json::to_string_pretty(&report::to_json(&r, &facts, grant.as_ref()))?);
    } else {
        print!("{}", report::to_text(&r, &facts, function, grant.as_ref()));
    }
    Ok(())
}

fn nearest_manifest(start: &Path) -> Option<(std::path::PathBuf, String)> {
    let mut cur = Some(start);
    while let Some(d) = cur {
        if let Ok(t) = std::fs::read_to_string(d.join("wj.toml")) {
            return Some((d.to_path_buf(), t));
        }
        cur = d.parent();
    }
    None
}
