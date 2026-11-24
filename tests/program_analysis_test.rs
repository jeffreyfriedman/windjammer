#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]
//! The shared program analysis (`src/program_analysis`, docs/design/shared-program-analysis.md step 1) changes nothing a
//! user sees and is actually used.
//!
//! - G1 differential: `wj check --json` with sharing on and with `WJ_NO_SHARED_ANALYSIS=1` agree byte for byte (minus the
//!   elapsed time) over the repository corpora, a two-file project and an interpreted run.
//! - G2 verify: `WJ_VERIFY_SHARED_ANALYSIS=1` recomputes every reuse and compares; no corpus program differs.
//! - G3 taken: the counters show each stage ran once (reuse counted, not silently recomputed).
//! - Keys: a product is reused only for the derivation its key names (in-process, through the library API).

#[path = "common/stages.rs"]
mod stages;

use stages::{wj, Ran, SMALL};
use std::path::{Path, PathBuf};
use windjammer::program_analysis::{FileKey, Globals, Mode, ProgramAnalysis};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            wj_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p);
        }
    }
}

/// Single-file programs: the check-negative mutants, the diagnostics oracle, the move and drift repros, the inference
/// corpus and the 517-line program (`every`: keep one program in `every`, so verify mode, which recomputes, stays quick).
fn corpus(every: usize) -> Vec<PathBuf> {
    let r = root();
    let mut files = Vec::new();
    for d in ["tests/check_negative/mutants", "tests/diagnostics_oracle", "tests/move_prediction", "tests/drift_corpus/repros", "tests/inference_corpus", "tests/fixtures/program_analysis"] {
        wj_files(&r.join(d), &mut files);
    }
    files.retain(|f| !f.starts_with(r.join("tests/fixtures/program_analysis/two_files")));
    files.sort();
    files.into_iter().enumerate().filter(|(i, _)| i % every == 0).map(|(_, f)| f).collect()
}

/// `f` over `items` on every core.
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism().map_or(4, |n| n.get()).min(items.len().max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some(item) = items.get(i) else { break };
                let r = f(item);
                done.lock().unwrap().push((i, r));
            });
        }
    });
    let mut done = done.into_inner().unwrap();
    done.sort_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}

/// Everything a user sees of `wj check --json`, without the elapsed time.
fn seen(r: &Ran) -> String {
    let out: String = r.stdout().lines().filter(|l| !l.contains("elapsed_ms")).collect::<Vec<_>>().join("\n");
    format!("exit={:?}\n{out}\n--- stderr\n{}", r.out.status.code(), r.stderr())
}

/// `wj check --json` of a copy of `file` in a directory of its own (the whole-run cache lives beside the program: two
/// runs of one directory would compare a run with its cached result).
fn check_json(file: &Path, env: &[(&str, &str)]) -> Ran {
    let dir = tempfile::tempdir().unwrap();
    let name = file.file_name().unwrap().to_str().unwrap();
    std::fs::copy(file, dir.path().join(name)).unwrap();
    wj(dir.path(), &["check", "--json", name], env)
}

#[test]
fn sharing_on_and_off_report_identically_over_the_corpora() {
    let files = corpus(1);
    assert!(files.len() > 200, "the corpus is found: {}", files.len());
    let diffs: Vec<String> = parallel(&files, |f| {
        let shared = check_json(f, &[]);
        let off = check_json(f, &[("WJ_NO_SHARED_ANALYSIS", "1")]);
        (seen(&shared) != seen(&off)).then(|| f.display().to_string())
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(diffs.is_empty(), "sharing changed the output of: {diffs:#?}");
}

#[test]
fn verify_mode_finds_no_reused_product_that_differs_from_its_recomputation() {
    let files = corpus(3);
    let bad: Vec<String> = parallel(&files, |f| {
        let shared = check_json(f, &[]);
        let verified = check_json(f, &[("WJ_VERIFY_SHARED_ANALYSIS", "1")]);
        let stderr = String::from_utf8_lossy(&verified.out.stderr).into_owned();
        (verified.out.status.code() == Some(86) || stderr.contains("program_analysis:") || seen(&shared) != seen(&verified)).then(|| format!("{}: {stderr}", f.display()))
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(bad.is_empty(), "verify mode: {bad:#?}");
}

/// A fresh copy of the two-file project fixture (the check writes its cache beside the project).
fn two_files() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("src")).unwrap();
    let from = root().join("tests/fixtures/program_analysis/two_files");
    for f in ["wj.toml", "src/main.wj", "src/dep.wj"] {
        std::fs::copy(from.join(f), d.path().join(f)).unwrap();
    }
    d
}

#[test]
fn a_two_file_project_reports_identically_with_sharing_on_and_off() {
    let run = |env: &[(&str, &str)]| wj(two_files().path(), &["check", "--json", "."], env);
    let (shared, off, verified) = (run(&[]), run(&[("WJ_NO_SHARED_ANALYSIS", "1")]), run(&[("WJ_VERIFY_SHARED_ANALYSIS", "1")]));
    assert!(shared.stdout().contains("WJ0051"), "the fixture reports the cross-file mutability finding: {}", shared.stdout());
    assert_eq!(seen(&shared), seen(&off));
    assert_eq!(seen(&shared), seen(&verified));
}

#[test]
fn an_interpreted_run_prints_the_same_with_sharing_on_and_off() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("p.wj"), SMALL).unwrap();
    let a = wj(d.path(), &["run", "--no-lint", "p.wj"], &[]);
    let b = wj(d.path(), &["run", "--no-lint", "p.wj"], &[("WJ_NO_SHARED_ANALYSIS", "1")]);
    assert_eq!((a.stdout(), a.stderr(), a.out.status.code()), (b.stdout(), b.stderr(), b.out.status.code()));
    assert_eq!(a.stdout().trim(), "3");
}

fn ledger(r: &Ran) -> String {
    ["parse", "analyzer_pass", "analyzer_reused", "numeric", "numeric_reused", "ir_lowering", "registry_copy", "facts_build", "rust_generation"]
        .iter()
        .map(|k| format!("{k}={}", r.n(k)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// G3. The check analyzes a file twice (A1 for the diagnostics and the move prediction, A2 for local mutability) and infers
/// its numeric types once; everything else asks for what exists. A silent fallback to recomputing shows as a higher
/// `analyzer_pass` here.
#[test]
fn the_shared_path_is_taken() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("p.wj"), SMALL).unwrap();
    let r = wj(d.path(), &["check", "p.wj"], &[]);
    assert!(r.out.status.success(), "{}", r.stderr());
    assert_eq!(ledger(&r), PIN_SHARED_SMALL);
    let off = wj(d.path(), &["check", "p.wj"], &[("WJ_NO_SHARED_ANALYSIS", "1")]);
    assert_eq!(ledger(&off), PIN_OFF_SMALL);

    std::fs::copy(root().join("tests/fixtures/program_analysis/gen500.wj"), d.path().join("g.wj")).unwrap();
    let big = wj(d.path(), &["check", "g.wj"], &[]);
    assert_eq!(ledger(&big), PIN_SHARED_517);
}

/// G3 on a two-file project: each file's first pass computed once and its second pass once (4 passes), the analysis
/// phase reuses the first pass of the file that has no earlier error (`dep`), which the move prediction's generator reuses
/// too, and one numeric inference serves both.
#[test]
fn a_two_file_project_analyzes_each_file_twice_and_infers_each_once() {
    let r = wj(two_files().path(), &["check", "."], &[]);
    assert_eq!((r.n("analyzer_pass"), r.n("analyzer_reused"), r.n("numeric"), r.n("numeric_reused"), r.n("rust_generation")), (4, 2, 1, 1, 1), "{:?}", r.counts);
}

// The ledger only shrinks (see tests/stage_counters_test.rs for the pre-session numbers).
const PIN_SHARED_SMALL: &str = "parse=5 analyzer_pass=2 analyzer_reused=2 numeric=1 numeric_reused=1 ir_lowering=2 registry_copy=3 facts_build=1 rust_generation=1";
const PIN_OFF_SMALL: &str = "parse=5 analyzer_pass=4 analyzer_reused=0 numeric=2 numeric_reused=0 ir_lowering=2 registry_copy=6 facts_build=1 rust_generation=1";
const PIN_SHARED_517: &str = "parse=12 analyzer_pass=4 analyzer_reused=2 numeric=2 numeric_reused=1 ir_lowering=4 registry_copy=7 facts_build=1 rust_generation=1";

fn parse(file: &str, text: &str) -> windjammer::parser::Program<'static> {
    let tokens = windjammer::lexer::Lexer::new(text).tokenize_with_locations();
    windjammer::parser::Parser::new_with_source(tokens, file.to_string(), text.to_string()).parse().expect("parses")
}

fn names(a: &windjammer::program_analysis::Analysis) -> Vec<String> {
    a.result.as_ref().expect("analyzes").functions.iter().map(|f| f.decl.name.clone()).collect()
}

/// A product is reused only for the text it was derived from: one file name, two texts, two answers.
#[test]
fn a_key_names_the_text_of_the_file() {
    let (one, two) = ("fn one(x: int) -> int {\n    x + 1\n}\n", "fn two(x: int) -> int {\n    x + 2\n}\n");
    let session = ProgramAnalysis::new(Mode::Shared);
    let a = session.first_pass(&FileKey::new("k.wj", one), &parse("k.wj", one));
    let b = session.first_pass(&FileKey::new("k.wj", two), &parse("k.wj", two));
    assert_eq!((names(&a), names(&b)), (vec!["one".to_string()], vec!["two".to_string()]));
    let again = session.first_pass(&FileKey::new("k.wj", one), &parse("k.wj", one));
    assert!(std::sync::Arc::ptr_eq(&a, &again), "the same derivation is the same product");

    let bad = "fn main() {\n    let a: int = 1\n    let b: float = 2.0\n    let c = a + b\n    println(\"${c}\")\n}\n";
    let clean = "fn main() {\n    let a: int = 1\n    println(\"${a}\")\n}\n";
    let n_bad = session.numeric(&FileKey::new("n.wj", bad), &parse("n.wj", bad));
    let n_clean = session.numeric(&FileKey::new("n.wj", clean), &parse("n.wj", clean));
    assert!(!std::sync::Arc::ptr_eq(&n_bad, &n_clean), "numeric inference is keyed by the text too");
    assert!(std::sync::Arc::ptr_eq(&n_bad, &session.numeric(&FileKey::new("n.wj", bad), &parse("n.wj", bad))));
}

/// The second pass reads the signatures of every file: it is keyed by the files merged, not only by the file.
#[test]
fn the_second_pass_is_keyed_by_the_files_whose_signatures_it_read() {
    let dep = "pub fn fill(items: Vec<int>) {\n    items.push(1)\n}\n";
    let main = "use crate::dep\n\nfn relay(items: Vec<int>) {\n    dep::fill(items)\n}\n";
    let (dep_key, main_key) = (FileKey::new("dep.wj", dep), FileKey::new("main.wj", main));
    let (dep_p, main_p) = (parse("dep.wj", dep), parse("main.wj", main));
    let session = ProgramAnalysis::new(Mode::Shared);
    let mut global = windjammer::analyzer::SignatureRegistry::new();
    for (k, p) in [(&dep_key, &dep_p), (&main_key, &main_p)] {
        global.merge(&session.first_pass(k, p).result.as_ref().expect("analyzes").registry);
    }
    let global = std::sync::Arc::new(global);
    let alone = session.second_pass(&main_key, &main_p, Globals::Merged(vec![main_key.clone()]), &std::sync::Arc::new(windjammer::analyzer::SignatureRegistry::new()));
    let both = session.second_pass(&main_key, &main_p, Globals::Merged(vec![dep_key.clone(), main_key.clone()]), &global);
    assert!(!std::sync::Arc::ptr_eq(&alone, &both), "different globals, different derivation");
    let first = session.first_pass(&main_key, &main_p);
    assert!(!std::sync::Arc::ptr_eq(&first, &both), "the second pass is not the first");
    let modes = |a: &windjammer::program_analysis::Analysis| format!("{:?}", a.result.as_ref().unwrap().functions[0].inferred_ownership);
    assert_ne!(modes(&alone), modes(&both), "the callee's signature changes what the caller's parameter needs");
}
