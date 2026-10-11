//! The resolver must account for every call in the real fixture corpus: each call resolves to a
//! definition, a local, a prelude item, a method-candidate set, or an *external* (std/runtime)
//! item. `NotFound` / `NoAssocItem` / ambiguity in the corpus are resolver bugs.
#[path = "resolve_support.rs"]
mod support;
use std::path::{Path, PathBuf};
use support::parse;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::resolve::{resolve_program, CallResolution, ParsedFile, Res};

const CORPORA: &[&str] = &[
    "tests/fixtures/library_multipass",
    "tests/windjammer_tests",
    "tests/fixtures/resolve",
];

pub fn collect_wj(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map(|r| r.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_wj(&p, out);
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

/// Parse a file, returning `None` when the parser rejects it (negative fixtures exist).
fn try_parse(path: &Path) -> Option<&'static windjammer::parser::Program<'static>> {
    let src = std::fs::read_to_string(path).ok()?;
    let mut lexer = Lexer::new(&src);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new_with_source(
        tokens,
        path.display().to_string(),
        src,
    )));
    parser.parse().ok().map(|p| &*Box::leak(Box::new(p)))
}

/// Resolve one corpus entry (every file is its own single-file program, like the multipass
/// per-file view); returns calls and the number of parse failures.
pub fn resolve_corpus(root: &str) -> (Vec<(PathBuf, Vec<CallResolution>)>, usize) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(root);
    let mut files = Vec::new();
    collect_wj(&base, &mut files);
    let (mut out, mut skipped) = (Vec::new(), 0);
    for f in files {
        let Some(program) = try_parse(&f) else {
            skipped += 1;
            continue;
        };
        let pf = ParsedFile::new(Vec::new(), f.clone(), program);
        let rp = resolve_program(&[pf]);
        out.push((f, rp.calls));
    }
    (out, skipped)
}

#[test]
fn every_call_in_fixture_corpora_is_accounted_for() {
    let _ = parse; // keep helper linked
    let mut bad = Vec::new();
    let mut total = 0;
    for root in CORPORA {
        let (files, skipped) = resolve_corpus(root);
        let calls: usize = files.iter().map(|(_, c)| c.len()).sum();
        total += calls;
        eprintln!(
            "[resolve] {root}: {} files parsed, {skipped} skipped, {calls} calls",
            files.len()
        );
        assert!(!files.is_empty(), "no fixtures found under {root}");
        let mut tally: std::collections::BTreeMap<String, usize> = Default::default();
        for (f, cs) in &files {
            for c in cs {
                *tally.entry(tally_key(&c.res)).or_default() += 1;
                if is_resolver_failure(&c.res) {
                    bad.push(format!(
                        "{}:{:?} {} -> {:?}",
                        f.display(),
                        c.location.as_ref().map(|l| l.line),
                        c.callee,
                        c.res
                    ));
                }
            }
        }
        eprintln!("[resolve] {root} breakdown: {tally:?}");
    }
    assert!(total > 100, "suspiciously few calls: {total}");
    assert!(
        bad.is_empty(),
        "{} unaccounted calls:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

fn is_resolver_failure(r: &Res) -> bool {
    match r {
        Res::Unresolved(reason) => !reason.is_benign(),
        Res::Ambiguous(_) => true,
        _ => false,
    }
}

fn tally_key(r: &Res) -> String {
    match r {
        Res::Def(_) => "Def".into(),
        Res::Local(_) => "Local".into(),
        Res::Prim(_) => "Prim".into(),
        Res::MethodCandidates(_) => "MethodCandidates".into(),
        Res::Ambiguous(_) => "Ambiguous".into(),
        Res::Unresolved(reason) => format!("Unresolved::{}", reason.kind_name()),
    }
}

#[test]
fn driver_hook_reports_shadow_counts_without_side_effects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/resolve");
    let report = windjammer::compiler::resolve_project_shadow(&dir).expect("hook");
    assert!(report.total_calls >= 20, "{report:?}");
    assert_eq!(report.ambiguous, 0);
    assert_eq!(report.unresolved, 0);
    assert_eq!(
        report.total_calls,
        report.resolved + report.method_candidates + report.unresolved + report.ambiguous
    );
}
