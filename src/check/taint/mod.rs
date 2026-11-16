//! WJ0173: data from an untrusted source (environment, command line, file contents, standard input, an HTTP request)
//! reaches a command, a file path to write or SQL text without a sanitizer (ADR-0009). `wj check`, `wj build`
//! and `wj run` stop on it. See `flow.rs` for the analysis and `catalog.rs` for what counts as source, sink, sanitizer.

mod catalog;
pub mod flow;

use super::front::Parsed;
use super::locate::Locator;
use crate::diagnostics::{Diagnostic, SourceIndex, Span};
use crate::resolve::{self, DefKind, ParsedFile};
use flow::{Finding, Pragma, Summary};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

/// The registry code.
pub const CODE: &str = "WJ0173";

/// Names of the `.wj` doc-comment annotations (`/// wj-taint: sanitizer`, `/// wj-taint: trusted(<reason>)`).
pub const ANNOTATION: &str = "wj-taint:";

pub use catalog::{SOURCES, STD_SANITIZERS};

/// Cheap test whether a program can have a flow at all: it must mention a source and a sink module.
fn may_flow(parsed: &[&Parsed]) -> bool {
    let any = |words: &[&str]| parsed.iter().any(|p| words.iter().any(|w| p.text.contains(w)));
    any(&["process", "fs", "db", "Connection"]) && any(&["env", "fs", "io", "args", "Request", "read"])
}

pub fn run(root: &Path, parsed: &[&Parsed], by_file: &HashMap<String, &Parsed>, out: &mut Vec<Diagnostic>) {
    if !may_flow(parsed) {
        return;
    }
    let base = super::effects::module_base(root);
    let inputs: Vec<ParsedFile<'_>> = parsed
        .iter()
        .map(|p| {
            let path = Path::new(&p.file);
            ParsedFile::new(resolve::module_path_from_file(&base, path), path.to_path_buf(), &p.program)
        })
        .collect();
    let resolved = resolve::resolve_program(&inputs);
    let prog = flow::Program::new(&resolved);
    let defs = resolved.defs();
    let fns: Vec<_> = resolved
        .collected
        .decls
        .iter()
        .filter(|(id, d)| !d.body.is_empty() && matches!(defs.get(**id).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }))
        .filter_map(|(id, d)| defs.get(*id).location.as_ref().map(|l| (*id, *d, l.file.to_string_lossy().to_string())))
        .collect();
    let mut summaries: HashMap<_, Summary> = HashMap::new();
    for _ in 0..12 {
        let mut changed = false;
        for (id, d, file) in &fns {
            let o = flow::analyze_fn(&prog, &summaries, d, file);
            if summaries.get(id) != Some(&o.summary) {
                summaries.insert(*id, o.summary);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut findings: BTreeSet<Finding> = BTreeSet::new();
    for (_, d, file) in &fns {
        findings.extend(flow::analyze_fn(&prog, &summaries, d, file).findings);
        if flow::pragma(d) == Pragma::TrustedNoReason {
            if let Some(p) = by_file.get(file) {
                let ix = SourceIndex::new(p.file.clone(), &p.text);
                let at = Locator::new(&ix).top_level_decl("fn", &d.name, 0).unwrap_or_else(|| ix.span_at(1, 1, 1));
                out.push(
                    Diagnostic::error(CODE, format!("`{ANNOTATION} trusted` on `{}` needs a reason", d.name))
                        .at(at.with_label("annotated without a reason"))
                        .with_help(format!("write `/// {ANNOTATION} trusted(<why this function may use the value>)`: the reason is the review record")),
                );
            }
        }
    }
    for f in &findings {
        out.push(diagnostic(f, by_file));
    }
}

fn span_of(place: &flow::Place, by_file: &HashMap<String, &Parsed>) -> Option<Span> {
    let p = by_file.get(&place.file)?;
    let ix = SourceIndex::new(p.file.clone(), &p.text);
    let loc = Some(crate::source_map::Location { file: place.file.clone().into(), line: place.line, column: place.column });
    if place.line == 0 {
        return None;
    }
    Locator::new(&ix).word(&loc, &place.word).or_else(|| Some(ix.span_at(place.line, place.column.max(1), place.word.len().max(1))))
}

fn diagnostic(f: &Finding, by_file: &HashMap<String, &Parsed>) -> Diagnostic {
    let role = &f.sink.role;
    let mut d = Diagnostic::error(CODE, format!("data from {} reaches {} without being sanitized", f.source_what, role.desc));
    let label = if f.via.is_empty() { role.role.to_string() } else { format!("the value is passed to `{}`, which uses it as {}", f.via.join("` and `"), role.desc) };
    match span_of(&f.report_at, by_file) {
        Some(s) => d = d.at(s.with_label(label)),
        None => d = d.with_note(format!("at {}:{}", f.report_at.file, f.report_at.line)),
    }
    if let Some(s) = span_of(&f.source_at, by_file) {
        d = d.with_secondary(s.with_label(format!("{} is read here", f.source_what)));
    }
    if !f.via.is_empty() {
        d = d.with_note(format!("`{}` is called at {}:{}", f.sink.callee, f.sink.at.file, f.sink.at.line));
    }
    d.with_help(format!("`{}` returns untrusted data; {}", f.source_callee, role.advice))
        .with_note("ADR-0009: untrusted data must not reach a command, a file path or SQL text unsanitized")
}
