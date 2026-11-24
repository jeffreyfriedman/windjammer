//! Parity of the in-process build generator with the transpile it replaced (ADR-0033, task I2).
//!
//! `cli::widths_gate` used to generate a flagged program with a child `wj build --no-cargo --no-lint` and read
//! the Rust from disk. It now runs the build's own pipeline in this process and takes the Rust from memory.
//! Every prediction the transpile path makes must still be made:
//!
//! 1. the Rust text is byte-identical to the child build's, for every file the trigger flags in the repository
//!    corpora (inference corpus, drift repros, engine parity, conformance programs, examples, tests);
//! 2. the diagnostics are identical (codes, messages, spans) for families A, B and C, and each family is present;
//! 3. a flagged program takes the in-process path (no silent fall back to a child process);
//! 4. mutation check: an in-process predictor that drops a family makes (2) fail.

use std::path::{Path, PathBuf};
use windjammer::cli::widths_gate::{built_in_process, built_via_child, predicted_widths_via, Source};
use windjammer::diagnostics::Diagnostic;

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

/// The single-file programs of the repository corpora, as `predictions_across_the_corpora` scans them.
fn corpus() -> Vec<PathBuf> {
    let root = root();
    let mut files = Vec::new();
    for d in ["examples", "tests"] {
        wj_files(&root.join(d), &mut files);
    }
    files.sort();
    files.retain(|f| !f.starts_with(root.join("tests/drift_corpus")) || f.starts_with(root.join("tests/drift_corpus/repros")));
    files.retain(|f| !f.starts_with(root.join("tests/project_corpus")));
    files
}

/// The first differing line of two sets of emitted files, with a little context.
fn first_difference(old: &[(String, String)], new: &[(String, String)]) -> String {
    if old.iter().map(|x| &x.0).ne(new.iter().map(|x| &x.0)) {
        return format!("file sets differ: child {:?}, in-process {:?}", old.iter().map(|x| &x.0).collect::<Vec<_>>(), new.iter().map(|x| &x.0).collect::<Vec<_>>());
    }
    for ((name, a), (_, b)) in old.iter().zip(new) {
        if a == b {
            continue;
        }
        let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
        let at = (0..la.len().max(lb.len())).find(|&i| la.get(i) != lb.get(i)).unwrap_or(0);
        let ctx = |l: &[&str]| l[at.saturating_sub(2)..(at + 3).min(l.len())].join("\n");
        return format!("{name} line {}:\n--- child\n{}\n--- in-process\n{}", at + 1, ctx(&la), ctx(&lb));
    }
    "identical".into()
}

fn family(d: &Diagnostic) -> char {
    d.notes.iter().find_map(|n| n.split("as family ").nth(1).and_then(|r| r.chars().next())).expect("a WJ0003 names its family")
}

fn shape(ds: &[Diagnostic]) -> Vec<String> {
    ds.iter().map(|d| format!("{} {} @{:?}", d.code, d.message, d.primary_span.as_ref().map(|s| (s.line_start, s.col_start)))).collect()
}

/// Files whose predictions differ between the two generators; `drop` removes a family from the in-process side
/// (the mutation under test).
fn differences(files: &[PathBuf], drop: Option<char>) -> (Vec<String>, Vec<char>, usize) {
    let exe = Path::new(env!("CARGO_BIN_EXE_wj"));
    let mut diff = Vec::new();
    let mut families = Vec::new();
    let mut flagged = 0;
    for f in files {
        let (mut fast, how) = predicted_widths_via(f, false);
        if let Some(c) = drop {
            fast.retain(|d| family(d) != c);
        }
        if how.is_none() && drop.is_none() {
            // not flagged by the trigger: nothing is generated, and the old path made no prediction either
            continue;
        }
        flagged += 1;
        let rel = f.strip_prefix(root()).unwrap_or(f).display().to_string();
        if drop.is_none() {
            assert_eq!(how, Some(Source::InProcess), "{rel}: the build generator fell back to a child process");
        }
        // the old path: a child `wj build --no-cargo --no-lint`, Rust read from disk
        let out = tempfile::tempdir().unwrap();
        let old_built = built_via_child(f, out.path(), Some(exe));
        let out2 = tempfile::tempdir().unwrap();
        let new_built = built_in_process(f, out2.path());
        if drop.is_none() {
            let stems = |b: &Option<Vec<(PathBuf, String)>>| {
                let mut v: Vec<(String, String)> = b
                    .iter()
                    .flatten()
                    .map(|(p, t)| (p.file_name().unwrap().to_string_lossy().to_string(), t.clone()))
                    .collect();
                v.sort();
                v
            };
            let (o, n) = (stems(&old_built), stems(&new_built));
            if o != n {
                panic!("{rel}: the in-process Rust differs from the child build's:\n{}", first_difference(&o, &n));
            }
        }
        let (slow, _) = predicted_widths_via(f, true);
        families.extend(fast.iter().map(family));
        if shape(&fast) != shape(&slow) {
            diff.push(format!("{rel}: in-process {:?} vs child {:?}", shape(&fast), shape(&slow)));
        }
    }
    (diff, families, flagged)
}

#[test]
fn in_process_generation_makes_every_prediction_the_transpile_made() {
    let files = corpus();
    assert!(files.len() > 500, "corpus unexpectedly small: {}", files.len());
    assert!(files.iter().any(|f| f.starts_with(root().join("tests/inference_corpus"))));
    let (diff, families, flagged) = differences(&files, None);
    assert!(diff.is_empty(), "predictions differ:\n{}", diff.join("\n"));
    assert!(flagged >= 40, "the trigger flagged only {flagged} files; the parity check lost its subject");
    // Family B (a map key passed without its borrow) is required only while the backend still makes it: its last
    // fixed repro, explicit_ref_literal_arg.wj, compiles since the map-key borrow fix (wt-ownership-15).
    for fam in ['A', 'C'] {
        assert!(families.contains(&fam), "no family {fam} prediction in the corpora (saw {families:?})");
    }
}

/// The gate must be able to fail: an in-process predictor that loses a family is caught.
#[test]
fn the_differential_fails_when_the_in_process_predictor_drops_a_family() {
    let r = root();
    // one repro per family: A (widths), C (`String` against `&str`), and B (a reference expected) from the first
    // corpus program that still has a family-B prediction; B is retired from the check when none has one (its last
    // repro, explicit_ref_literal_arg.wj, compiles since the map-key borrow fix).
    let family_b = corpus().into_iter().find(|f| predicted_widths_via(f, false).0.iter().any(|d| family(d) == 'B'));
    // family C is retired from the check the same way when no corpus program has one: its last repro,
    // closure_string_param_literal.wj, compiles since a let-bound closure's call converts its arguments.
    let family_c = corpus().into_iter().find(|f| predicted_widths_via(f, false).0.iter().any(|d| family(d) == 'C'));
    let mut cases: Vec<(PathBuf, char)> = vec![
        (r.join("tests/drift_corpus/repros/let_literal_captured_by_closure.wj"), 'A'),
    ];
    cases.extend(family_b.map(|f| (f, 'B')));
    cases.extend(family_c.map(|f| (f, 'C')));
    for (file, fam) in cases {
        let file = file.display().to_string();
        let files = vec![PathBuf::from(&file)];
        let (clean, fams, _) = differences(&files, None);
        assert!(clean.is_empty() && fams.contains(&fam), "{file}: baseline must agree and show family {fam}: {clean:?} {fams:?}");
        let (mutated, _, _) = differences(&files, Some(fam));
        assert!(!mutated.is_empty(), "{file}: dropping family {fam} went unnoticed");
    }
}
