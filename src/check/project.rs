//! Whole-project phases: name resolution, semantic checks, analyzer, numeric inference, IR.

use super::front::Parsed;
use crate::ir::stdlib_tables::StdlibTables;
use super::locate::Locator;
use super::semantic::{self, FileInput, Index};
use crate::diagnostics::{convert, Diagnostic, SourceIndex};
use crate::resolve::{self, DefKind, DiagnosticKind, ParsedFile, Reason, Res};
use std::collections::HashMap;
use std::path::Path;

/// GPU shader files (`@vertex` / `@fragment` / `@compute`) use WGSL built-ins and are compiled by
/// the shader pipeline, not the Rust one; the build skips them and so do the semantic checks.
fn is_shader(p: &Parsed) -> bool {
    p.program.items.iter().any(|i| match i {
        crate::parser::Item::Function { decl, .. } => decl
            .decorators
            .iter()
            .any(|d| matches!(d.name.as_str(), "vertex" | "fragment" | "compute")),
        _ => false,
    })
}

pub fn run(root: &Path, all: &[Parsed], out: &mut Vec<Diagnostic>) {
    let parsed: Vec<&Parsed> = all.iter().filter(|p| !is_shader(p)).collect();
    let parsed = parsed.as_slice();
    let by_file: HashMap<String, &Parsed> = parsed.iter().map(|p| (p.file.clone(), *p)).collect();
    resolve_phase(root, parsed, &by_file, out);
    super::effects::run(root, parsed, &by_file, out);
    let programs: Vec<_> = parsed.iter().map(|p| &p.program).collect();
    let idx = Index::build(&programs);
    for p in parsed {
        semantic::check_file(
            &idx,
            &FileInput { file: &p.file, text: &p.text, items: &p.program.items },
            out,
        );
    }
    // Session-shared stdlib tables, computed on the first analysis.
    let tables = std::cell::OnceCell::new();
    for p in parsed {
        // A file that already has errors would only add cascading analyzer noise.
        let has_errors = out
            .iter()
            .any(|d| d.is_error() && d.primary_span.as_ref().is_some_and(|s| s.file == p.file));
        if !has_errors {
            analysis_phase(p, out, tables.get_or_init(|| std::sync::Arc::new(StdlibTables::compute())).clone());
        }
    }
}

fn resolve_phase(root: &Path, parsed: &[&Parsed], by_file: &HashMap<String, &Parsed>, out: &mut Vec<Diagnostic>) {
    let base = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    let inputs: Vec<ParsedFile<'_>> = parsed
        .iter()
        .map(|p| {
            let path = Path::new(&p.file);
            ParsedFile::new(resolve::module_path_from_file(base, path), path.to_path_buf(), &p.program)
        })
        .collect();
    let r = resolve::resolve_program(&inputs);
    let span_for = |loc: &Option<crate::source_map::Location>, word: &str| {
        let l = loc.as_ref()?;
        let p = by_file.get(l.file.to_string_lossy().as_ref())?;
        let ix = SourceIndex::new(p.file.clone(), &p.text);
        let sl = Some(l.clone());
        Locator::new(&ix).word(&sl, word)
    };
    for c in &r.calls {
        match &c.res {
            // Language intrinsics (`debug`, `todo`, ...) are provided by the compiler, not the program.
            Res::Unresolved(Reason::NotFound(_))
                if c.kind == resolve::CallKind::Free
                    && crate::intrinsics::Intrinsic::from_call_name(&c.callee).is_none()
                    // `scope { .. }` is lowered by the parser to a call of the scope runtime (ADR-0004).
                    && c.callee != crate::parser::task_prefix::SCOPE_FN =>
            {
                let mut d = Diagnostic::error("WJ0002", format!("cannot find function `{}` in this scope", c.callee));
                if let Some(sp) = span_for(&c.location, &c.callee) {
                    d = d.at(sp.with_label("not found in this scope"));
                }
                let names: Vec<String> = r
                    .defs()
                    .iter()
                    .filter(|d| matches!(d.kind, DefKind::Fn))
                    .map(|d| d.name.clone())
                    .collect();
                let best = names
                    .iter()
                    .filter(|n| **n != c.callee)
                    .map(|n| (crate::fuzzy_matcher::levenshtein_distance(&c.callee, n), n))
                    .filter(|(dist, _)| *dist <= (c.callee.len() / 3).clamp(1, 3))
                    .min_by_key(|(dist, n)| (*dist, (*n).clone()));
                d = match best {
                    Some((_, n)) => d.with_help(format!("a function with a similar name exists: `{n}`")),
                    None => d.with_help("define it, or import it with `use`"),
                };
                out.push(d);
            }
            Res::Def(id) if matches!(c.kind, resolve::CallKind::Free | resolve::CallKind::Path) => {
                let def = r.defs().get(*id);
                let checked = matches!(def.kind, DefKind::Fn | DefKind::AssocFn { .. });
                if checked && !def.is_pub && c.kind == resolve::CallKind::Path {
                    let owner = &r.defs().get(def.module).path;
                    let here = &r.defs().get(c.module).path;
                    let visible = here == owner || owner.is_empty() || here.starts_with(&format!("{owner}::"));
                    if !visible {
                        let last = c.callee.rsplit("::").next().unwrap_or(&c.callee);
                        let mut d = Diagnostic::error("WJ0019", format!("function `{}` is private", c.callee));
                        let sp = span_for(&c.location, last).or_else(|| {
                            let p = by_file.values().find(|p| p.text.contains(last))?;
                            let ix = SourceIndex::new(p.file.clone(), &p.text);
                            Locator::new(&ix).first_use(last)
                        });
                        if let Some(sp) = sp {
                            d = d.at(sp.with_label("private function"));
                        }
                        out.push(d.with_help(format!("declare it `pub fn {last}` in module `{owner}`")));
                    }
                }
                if checked && c.arg_count != def.arity {
                    let mut d = Diagnostic::error(
                        "WJ0013",
                        format!(
                            "function `{}` takes {} argument{} but {} {} supplied",
                            c.callee,
                            def.arity,
                            if def.arity == 1 { "" } else { "s" },
                            c.arg_count,
                            if c.arg_count == 1 { "was" } else { "were" }
                        ),
                    );
                    let last = c.callee.rsplit("::").next().unwrap_or(&c.callee);
                    let sp = span_for(&c.location, last).or_else(|| {
                        let p = by_file.values().find(|p| p.text.contains(last))?;
                        let ix = SourceIndex::new(p.file.clone(), &p.text);
                        Locator::new(&ix).first_use(last)
                    });
                    if let Some(sp) = sp {
                        d = d.at(sp.with_label(format!("expected {} argument(s)", def.arity)));
                    }
                    if let Some(l) = &def.location {
                        d = d.with_note(format!("defined at {}:{}", l.file.display(), l.line));
                    }
                    out.push(d);
                }
            }
            _ => {}
        }
    }
    for rd in &r.report.diagnostics {
        if rd.kind == DiagnosticKind::BadImport {
            let word = rd.message.rsplit("::").next().unwrap_or("").trim_matches('`').to_string();
            let mut d = Diagnostic::error("WJ0006", rd.message.clone());
            if let Some(sp) = span_for(&rd.location, &word) {
                d = d.at(sp.with_label("no such module or item"));
            }
            out.push(d.with_help("check the path, or create / declare the module"));
        }
    }
}

fn analysis_phase(p: &Parsed, out: &mut Vec<Diagnostic>, tables: std::sync::Arc<StdlibTables>) {
    use crate::analyzer::{Analyzer, SignatureRegistry};
    let mut analyzer = Analyzer::new();
    let sigs = SignatureRegistry::new();
    match analyzer.analyze_program_with_global_signatures(&p.program, &sigs) {
        Ok((analyzed, registry, _)) => {
            let mut num = crate::ir::numeric_bridge::UnifiedNumericInference::new();
            num.infer_program(&p.program);
            for e in &num.errors {
                out.push(
                    Diagnostic::error("WJ0027", e.clone())
                        .with_note(format!("in {}", p.file)),
                );
            }
            let mut pipeline = crate::ir::IrPipeline::new().with_stdlib_tables(tables);
            let module = pipeline.lower_to_ir(&analyzed, &registry, None);
            use crate::ir::pipeline::DiagnosticSeverity as S;
            for d in &module.diagnostics {
                // Only the solvers' real violations; shadow-mode constraint chatter is not a finding.
                let real = ["Effect violation", "Taint", "Execution"].iter().any(|p| d.message.starts_with(p));
                if d.severity != S::Error || !real {
                    continue;
                }
                out.push(
                    convert::ir(d.severity == S::Error, d.severity == S::Warning, &d.message)
                        .with_note(format!("in {}", p.file)),
                );
            }
        }
        Err(e) => out.push(
            Diagnostic::error("WJ0028", format!("{e}")).with_note(format!("in {}", p.file)),
        ),
    }
}
