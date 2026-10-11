//! Shadow comparison: resolver vs. the legacy string-keyed `SignatureRegistry` lookup on the
//! fixture corpora. Disagreements are *data* (listed, counted), never failures.
//!
//! Regenerate the committed report with:
//! `cargo test --release --test all write_shadow_diffs_doc -- --ignored`
#[path = "resolve_support.rs"]
mod support;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use windjammer::analyzer::SignatureRegistry;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::resolve::{resolve_program, CallKind, DefKind, ParsedFile, Res, ResolvedProgram};

const CORPORA: &[&str] = &[
    "tests/fixtures/library_multipass",
    "tests/windjammer_tests",
    "tests/fixtures/resolve",
];
const EXAMPLES_PER_KIND: usize = 12;

#[derive(Debug)]
struct Disagreement {
    kind: &'static str,
    file: String,
    line: usize,
    callee: String,
    resolver: String,
    legacy: String,
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    fn go(d: &Path, out: &mut Vec<PathBuf>) {
        let mut es: Vec<_> = std::fs::read_dir(d)
            .map(|r| r.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        es.sort();
        for p in es {
            if p.is_dir() {
                go(&p, out);
            } else if p.extension().is_some_and(|e| e == "wj") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    go(root, &mut v);
    v
}

fn parse_file(path: &Path) -> Option<&'static windjammer::parser::Program<'static>> {
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

fn describe(rp: &ResolvedProgram<'_>, res: &Res) -> String {
    let path = |d: &windjammer::resolve::DefId| rp.defs().get(*d).path.clone();
    match res {
        Res::Def(d) => format!("Def({})", path(d)),
        Res::Ambiguous(v) => format!(
            "Ambiguous({})",
            v.iter().map(path).collect::<Vec<_>>().join(" | ")
        ),
        Res::MethodCandidates(v) => format!(
            "Candidates({})",
            v.iter().map(path).collect::<Vec<_>>().join(", ")
        ),
        other => format!("{other:?}"),
    }
}

fn compare_file(file: &Path, rel: &str, out: &mut Vec<Disagreement>) -> usize {
    let Some(program) = parse_file(file) else {
        return 0;
    };
    let registry = SignatureRegistry::from_program_declarations(program);
    let rp = resolve_program(&[ParsedFile::new(Vec::new(), file.to_path_buf(), program)]);
    for c in &rp.calls {
        let keys = registry.callee_lookup_keys(&c.callee);
        let kind = classify(&registry, &rp, c, &keys);
        if let Some(kind) = kind {
            out.push(Disagreement {
                kind,
                file: rel.to_string(),
                line: c.location.as_ref().map(|l| l.line).unwrap_or(0),
                callee: c.callee.clone(),
                resolver: describe(&rp, &c.res),
                legacy: format!("{keys:?}"),
            });
        }
    }
    rp.calls.len()
}

fn classify(
    reg: &SignatureRegistry,
    rp: &ResolvedProgram<'_>,
    c: &windjammer::resolve::CallResolution,
    keys: &[String],
) -> Option<&'static str> {
    match (&c.res, c.kind) {
        (Res::Def(d), _) => {
            let def = rp.defs().get(*d);
            if keys.is_empty() && def.kind != DefKind::ExternFn {
                return Some("legacy-has-no-signature-for-resolved-def");
            }
            if keys.len() > 1 {
                return Some("legacy-collision-resolver-unique");
            }
            let sig = reg.get_signature(&c.callee)?;
            let want = def.arity + usize::from(def.has_self);
            (sig.param_types.len() != want).then_some("legacy-signature-arity-differs-from-def")
        }
        (Res::Local(_), _) if !keys.is_empty() => Some("local-shadows-name-legacy-would-match"),
        (Res::Prim(_) | Res::Unresolved(_), CallKind::Free) if !keys.is_empty() => {
            Some("legacy-matches-free-name-resolver-says-not-a-program-fn")
        }
        (Res::Ambiguous(_), _) => Some("resolver-ambiguous"),
        (Res::MethodCandidates(v), _) if v.len() != keys.len() => {
            Some("method-candidate-count-differs-from-legacy-keys")
        }
        (Res::Unresolved(_), CallKind::Method) if !keys.is_empty() => {
            Some("legacy-matches-method-resolver-has-no-candidate")
        }
        _ => None,
    }
}

fn run_corpora() -> (Vec<Disagreement>, usize, usize) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (mut diffs, mut files, mut calls) = (Vec::new(), 0, 0);
    for root in CORPORA {
        for f in files_under(&manifest.join(root)) {
            let rel = f.strip_prefix(manifest).unwrap_or(&f).display().to_string();
            let n = compare_file(&f, &rel, &mut diffs);
            files += usize::from(n > 0);
            calls += n;
        }
    }
    (diffs, files, calls)
}

fn render(diffs: &[Disagreement], files: usize, calls: usize) -> String {
    let mut by_kind: BTreeMap<&str, Vec<&Disagreement>> = BTreeMap::new();
    for d in diffs {
        by_kind.entry(d.kind).or_default().push(d);
    }
    let mut s = String::from("# Resolver shadow-mode diffs\n\n");
    s.push_str("Generated by `tests/resolve_shadow_test.rs` (`write_shadow_diffs_doc`). Compares `windjammer::resolve` with the legacy string-keyed `SignatureRegistry::callee_lookup_keys` on the fixture corpora. Disagreements are data, not failures.\n\n");
    s.push_str(&format!("Files with calls: {files}; calls compared: {calls}; disagreements: {}.\n\n| Kind | Count |\n|---|---|\n", diffs.len()));
    for (k, v) in &by_kind {
        s.push_str(&format!("| {k} | {} |\n", v.len()));
    }
    for (k, v) in &by_kind {
        s.push_str(&format!("\n## {k} ({})\n\n", v.len()));
        for d in v.iter().take(EXAMPLES_PER_KIND) {
            s.push_str(&format!(
                "- `{}:{}` `{}`: resolver {}; legacy {}\n",
                d.file, d.line, d.callee, d.resolver, d.legacy
            ));
        }
    }
    s
}

#[test]
fn shadow_comparison_runs_over_corpora() {
    let (diffs, files, calls) = run_corpora();
    assert!(
        files > 20 && calls > 100,
        "corpora not found: {files} files {calls} calls"
    );
    let doc = render(&diffs, files, calls);
    assert!(doc.contains("Resolver shadow-mode diffs"));
    eprintln!(
        "[resolve-shadow] {} disagreements over {calls} calls",
        diffs.len()
    );
}

#[test]
#[ignore = "regenerates docs/resolve-shadow-diffs.md"]
fn write_shadow_diffs_doc() {
    let (diffs, files, calls) = run_corpora();
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/resolve-shadow-diffs.md");
    std::fs::write(out, render(&diffs, files, calls)).expect("write doc");
}
