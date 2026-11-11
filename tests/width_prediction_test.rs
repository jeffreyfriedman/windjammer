//! Mixed integer widths predicted from the generated Rust (`check::widths`, WJ0003 before an interpreted run,
//! ADR-0033). Family A of docs/drift/findings.md: the compiled backend types a literal `i32` next to an `int`.
//!
//! Precision gate: across the repository's single-file corpora every prediction must be a program rustc
//! rejects with E0308 on its generated code (checked here with rustc, an independent oracle), and the list of
//! files with a prediction is pinned. A new prediction fails until it is confirmed and added (or fixed).
//! Second oracle: every recorded golden of the convergence gates that the prediction flags must be one rustc
//! rejects (tests/golden/rustc_rejects.txt).

use std::path::{Path, PathBuf};
use windjammer::check::widths::clashes;
use windjammer::cli::widths_gate::predicted_widths;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn files_with_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            files_with_ext(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

/// rustc's verdict on one generated file: `Some(stderr)` when it rejects it.
fn rustc_rejects(rust: &str) -> Option<String> {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("main.rs");
    std::fs::write(&f, rust).unwrap();
    rustc_rejects_file(&f)
}

/// rustc's verdict on a crate root (its `mod` files next to it).
fn rustc_rejects_file(f: &Path) -> Option<String> {
    let rust = std::fs::read_to_string(f).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let crate_type = if rust.contains("fn main(") { "bin" } else { "lib" };
    let o = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", crate_type, "--emit=metadata", "-A", "warnings", "--crate-name", "widths", "-o"])
        .arg(tmp.path().join("out.rmeta"))
        .arg(f)
        .output()
        .expect("rustc on PATH");
    (!o.status.success()).then(|| String::from_utf8_lossy(&o.stderr).to_string())
}

/// The Rust `wj build --no-cargo` emits for the single file at `path` (the text the prediction is confirmed on).
fn built_rust(path: &Path) -> Option<String> {
    let out = tempfile::tempdir().unwrap();
    windjammer::build_project(path, out.path(), windjammer::CompilationTarget::Rust, false).ok()?;
    let stem = path.file_stem()?.to_string_lossy().to_string();
    std::fs::read_to_string(out.path().join(format!("{stem}.rs"))).ok()
}

fn n(rust: &str) -> usize {
    clashes(rust).len()
}

#[test]
fn certain_width_mixes_are_found() {
    // the shapes of the drift ledger's family A, as the compiler emits them
    assert_eq!(n("fn main() { let five = 5_i32; let add5 = |x: i64| x + five; }"), 1, "closure parameter + captured literal");
    assert_eq!(n("struct N { next: Option<i64> }\nimpl N { fn a(self) -> i64 { match self.next.clone() { Some(v) => v - 1_i32, None => 0_i64 } } }"), 1, "Option field payload");
    assert_eq!(n("fn main() { let item: Option<i64> = Some(5_i64); match item { Some(item) => { let item = item + 1_i32 as i32; } None => {} } }"), 1, "cast literal");
    assert_eq!(n("fn main() { let item = Some(5_i32); let x = match item { Some(item) => item * 3_i32, None => 0_i64 }; }"), 1, "match arms");
    assert_eq!(n("fn main() { let mut grid = vec![vec![1_i32, 2_i32]]; grid[0][1] = 9_i64; }"), 1, "indexed assignment");
    assert_eq!(n("fn h(n: i64) -> i64 { n }\nfn main() { let x = 3_i32; h(x); }"), 1, "argument of a function of the file");
    assert_eq!(n("fn main() { let v: Vec<i64> = Vec::new(); let k = 2_i64; if v.len() > k {} }"), 1, "usize from len() against int");
    assert_eq!(n("fn main() { let o: Option<i64> = None; let d = 0_i32; let x = o.unwrap_or(d); }"), 1, "unwrap_or default");
    assert_eq!(n("struct P { x: i64 }\nfn main() { let a = 1_i32; let p = P { x: a }; }"), 1, "struct field");
}

#[test]
fn uncertain_or_valid_shapes_are_not_reported() {
    for (why, rust) in [
        ("one width", "fn main() { let a = 1_i64; let b = 2_i64; let c = a + b * 3; }"),
        ("unsuffixed literal infers", "fn main() { let a = 1_i64; let c = a + 1; }"),
        ("shift counts take any width", "fn main() { let a = 1_i64; let b = 2_i32; let c = a << b; }"),
        ("shadowing rebinds", "fn main() { let a = 1_i64; let b = 2_i32; let b = 3_i64; let c = a + b; }"),
        ("inner block ends", "fn main() { let a = 1_i64; let b = 2_i64; { let b = 2_i32; } let c = a + b; }"),
        ("closure parameter shadows", "fn main() { let x = 1_i32; let f = |x: i64| x + 1_i64; }"),
        ("a closure named like a function", "fn h(n: i64) -> i64 { n }\nfn main() { let h = |n: i32| n; let x = 3_i32; h(x); }"),
        ("a user Some", "enum E { Some(i32), None }\nfn main() { let a = Some(1_i32); }"),
        ("a constant in a pattern", "const K: i32 = 1;\nfn main() { let v = 2_i64; match 3_i32 { K => {}, _ => {} } let c = v + 1_i64; }"),
        ("user operator: the left side picks the impl", "struct V(i64);\nimpl std::ops::Mul<i64> for V { type Output = V; fn mul(self, k: i64) -> V { V(self.0 * k) } }\nfn main() { let p = V(1); let q = p * 2_i64; }"),
        ("a nested function of the same name", "fn h(n: i64) -> i64 { n }\nfn main() { fn h(n: i32) -> i32 { n } let x = 3_i32; h(x); }"),
        ("trait impl methods are not inherent", "struct S;\ntrait T { fn f(&self, n: i32) -> i32; }\nimpl T for S { fn f(&self, n: i32) -> i32 { n } }\nfn main() { let s = S; let a = s.f(1_i32); }"),
        ("a generic function", "fn id<T>(t: T) -> T { t }\nfn main() { let a = id(1_i32) + 2_i32; }"),
        ("comparison of one width", "fn main() { let a = 1_i32; if a > 2_i32 {} }"),
        ("pow takes u32", "fn main() { let a = 2_i64; let b = a.pow(3_u32); }"),
    ] {
        let found = clashes(rust);
        assert!(found.is_empty(), "{why}: {found:?}");
        assert!(rustc_rejects(rust).is_none(), "{why}: the case itself must be valid Rust:\n{}", rustc_rejects(rust).unwrap());
    }
}

#[test]
fn the_found_shapes_are_rejected_by_rustc() {
    // the oracle agrees with the positive cases above (a case rustc accepts would be a false positive)
    for rust in [
        "fn main() { let five = 5_i32; let add5 = |x: i64| x + five; }",
        "fn main() { let mut grid = vec![vec![1_i32, 2_i32]]; grid[0][1] = 9_i64; }",
        "fn main() { let v: Vec<i64> = Vec::new(); let k = 2_i64; if v.len() > k {} }",
        "struct P { x: i64 }\nfn main() { let a = 1_i32; let p = P { x: a }; }",
    ] {
        let err = rustc_rejects(rust).unwrap_or_else(|| panic!("rustc accepts: {rust}"));
        assert!(err.contains("E0308"), "{rust}: {err}");
    }
}

#[test]
fn drift_repros_of_the_families_are_predicted() {
    let repros = root().join("tests/drift_corpus/repros");
    for (f, line) in [
        ("let_literal_captured_by_closure.wj", 5),
        ("shadow_match_binding_literal.wj", 6),
        ("shadowed_option_literal_arms.wj", 5),
        // family B (a reference expected) and C (`String` against `&str`)
        ("explicit_ref_literal_arg.wj", 9),
        ("closure_string_param_literal.wj", 6),
    ] {
        let d = predicted_widths(&repros.join(f));
        assert!(!d.is_empty(), "{f}: no prediction");
        assert!(d.iter().all(|x| x.code == "WJ0003"), "{f}: {d:?}");
        let at = d[0].primary_span.as_ref().map(|s| s.line_start);
        assert!(at.is_some_and(|l| l <= line + 6), "{f}: placed at {at:?}");
    }
}

/// `wj check` reports the prediction as an error; an interpreted run warns before the program's output and runs it.
#[test]
fn check_errs_and_the_interpreted_run_warns_and_runs() {
    let f = root().join("tests/drift_corpus/repros/let_literal_captured_by_closure.wj");
    let wj = env!("CARGO_BIN_EXE_wj");
    let c = std::process::Command::new(wj).arg("check").arg(&f).output().unwrap();
    assert_eq!(c.status.code(), Some(1), "check: {}", String::from_utf8_lossy(&c.stdout));
    assert!(String::from_utf8_lossy(&c.stdout).contains("error[WJ0003]: the compiled build would reject this"));
    let r = std::process::Command::new(wj).args(["run", "--interpret"]).arg(&f).output().unwrap();
    let err = String::from_utf8_lossy(&r.stderr);
    assert_eq!(r.status.code(), Some(0), "{err}");
    assert_eq!(String::from_utf8_lossy(&r.stdout), "6\n");
    assert!(err.contains("warning[WJ0003]: this runs interpreted, but the compiled build would reject it"), "{err}");
    assert!(err.contains("the interpreter runs it; rustc is stricter"), "{err}");
}

/// The warning goes to stderr only: stdout and the exit status are exactly those of the same program without the
/// predicted clash (the documented workaround), including a non-zero exit.
#[test]
fn the_interpreted_warning_never_changes_stdout_or_the_exit_status() {
    let tmp = tempfile::tempdir().unwrap();
    let body = |five: &str| format!("use std::process\n\nfn main() {{\n    {five}\n    let add5 = |x: int| x + five\n    println(\"${{add5(1)}}\")\n    process::exit(3)\n}}\n");
    let warned = tmp.path().join("warned.wj");
    let clean = tmp.path().join("clean.wj");
    std::fs::write(&warned, body("let five = 5")).unwrap();
    std::fs::write(&clean, body("let five: int = 5")).unwrap();
    let run = |f: &Path| std::process::Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(f).output().unwrap();
    let (w, c) = (run(&warned), run(&clean));
    assert!(String::from_utf8_lossy(&w.stderr).contains("warning[WJ0003]"), "the clash is predicted: {}", String::from_utf8_lossy(&w.stderr));
    assert!(!String::from_utf8_lossy(&c.stderr).contains("WJ0003"), "the workaround is not: {}", String::from_utf8_lossy(&c.stderr));
    assert_eq!(w.stdout, b"6\n");
    assert_eq!(w.stdout, c.stdout, "stdout is the program's alone");
    assert_eq!(w.status.code(), Some(3));
    assert_eq!(w.status.code(), c.status.code(), "the exit status is the program's");
}

#[test]
fn the_documented_workaround_builds_and_is_not_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("five.wj");
    std::fs::write(&f, "fn main() {\n    let five: int = 5\n    let add5 = |x: int| x + five\n    println(\"${add5(1)}\")\n}\n").unwrap();
    assert!(predicted_widths(&f).is_empty());
    let rust = built_rust(&f).expect("builds");
    assert!(rustc_rejects(&rust).is_none(), "{rust}");
}

#[test]
fn predictions_across_the_corpora() {
    let root = root();
    let mut files = Vec::new();
    for d in ["examples", "tests"] {
        files_with_ext(&root.join(d), "wj", &mut files);
    }
    files.sort();
    // the drift corpus copies whole multi-file projects (one file read alone cannot be judged); its single-file
    // repros are programs and are scanned
    files.retain(|f| !f.starts_with(root.join("tests/drift_corpus")) || f.starts_with(root.join("tests/drift_corpus/repros")));
    // the project corpus holds multi-file programs: each is scanned whole, from its `main.wj`, below
    files.retain(|f| !f.starts_with(root.join("tests/project_corpus")));
    let mut hits = Vec::new();
    for f in &files {
        let d = predicted_widths(f);
        if !d.is_empty() {
            let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
            // independent oracle: rustc rejects the generated code with a type mismatch
            let rust = built_rust(f).expect("a predicted file builds");
            let err = rustc_rejects(&rust).unwrap_or_else(|| panic!("false positive, rustc accepts {rel}: {d:?}"));
            assert!(err.contains("E0308"), "{rel}: rustc rejects it, but not with E0308:\n{err}");
            hits.push(format!("{rel} {}", d.len()));
        }
    }
    // every entry was confirmed by rustc (E0308) on the build's output above; programs `wj check` already rejects
    // for a type error (check_negative mutants, error-system fixtures) are pinned too: their builds fail the same way
    let pinned = [
        "examples/syntax_tests/99_error_test/main.wj 1",
        "examples/testing_examples.wj 3",
        "examples/verified/hashset_basics.wj 2",
        "examples/verified/prog_lru_cache.wj 2",
        "tests/check_negative/mutants/algo_caesar_cipher__string_annotated_int.wj 1",
        "tests/check_negative/mutants/algo_roman_numerals__string_annotated_int.wj 1",
        "tests/check_negative/mutants/basics_for_range__string_annotated_int.wj 1",
        "tests/check_negative/mutants/basics_interpolation__string_annotated_int.wj 1",
        "tests/check_negative/mutants/basics_loop_break__string_annotated_int.wj 1",
        "tests/check_negative/mutants/basics_while__string_annotated_int.wj 1",
        "tests/check_negative/mutants/chars_ascii_arith__string_annotated_int.wj 1",
        "tests/check_negative/mutants/error_unwrap_or_expect__let_int_gets_string.wj 1",
        "tests/check_negative/mutants/from_conversion__let_int_gets_string.wj 1",
        "tests/check_negative/mutants/numbers_integer_widths__let_int_gets_string.wj 1",
        "tests/check_negative/mutants/prog_run_length_encoding__string_annotated_int.wj 1",
        "tests/check_negative/mutants/prog_text_wrap__string_annotated_int.wj 1",
        "tests/check_negative/mutants/strings_basics__string_annotated_int.wj 1",
        "tests/check_negative/mutants/strings_build__string_annotated_int.wj 1",
        "tests/check_negative/mutants/strings_format_specs__string_annotated_int.wj 1",
        "tests/check_negative/mutants/strings_unicode__string_annotated_int.wj 1",
        "tests/conformance_programs/174_closure_mut_capture.wj 3",
        "tests/conformance_programs/62_vec_alias_semantics.wj 1",
        "tests/conformance_programs/98_bignum_factorial.wj 1",
        "tests/diagnostics/cases/let_string_to_int.wj 1",
        "tests/diagnostics/cases/numeric_string_to_int.wj 1",
        "tests/diagnostics_oracle/check/let_string_to_int.wj 1",
        "tests/diagnostics_oracle/check/numeric_string_to_int.wj 1",
        "tests/dogfood/voxel_raymarch.wj 7",
        "tests/drift_corpus/repros/closure_string_param_literal.wj 1",
        "tests/drift_corpus/repros/explicit_ref_literal_arg.wj 2",
        "tests/drift_corpus/repros/let_literal_captured_by_closure.wj 1",
        "tests/drift_corpus/repros/shadow_match_binding_literal.wj 1",
        "tests/drift_corpus/repros/shadowed_option_literal_arms.wj 1",
        "tests/effects/corpus/fs_helper_and_closure.wj 1",
        "tests/engine_parity/closure_params.wj 1",
        "tests/engine_parity/closures_compiled.wj 1",
        "tests/engine_parity/map_get_match.wj 2",
        "tests/engine_parity/match_arms.wj 1",
        "tests/engine_parity/match_value.wj 3",
        "tests/engine_parity/option_arm_values.wj 1",
        "tests/error_system/test_type_mismatch.wj 1",
        "tests/error_system_e2e/test_type_errors.wj 1",
        "tests/fixtures/library_multipass/postgres_query_vec_params.wj 1",
        "tests/fixtures/library_multipass/u64_index_len_compare.wj 1",
        "tests/inference_corpus/closure_len_width.wj 1",
        "tests/spec/closures/SPEC-CLOS-009.wj 1",
    ];
    assert_eq!(hits, pinned, "files with a width prediction changed (scanned {} files)", files.len());
}

#[test]
fn multi_file_programs_are_judged_whole() {
    let mut mains = Vec::new();
    for e in std::fs::read_dir(root().join("tests/project_corpus")).unwrap().flatten() {
        let m = e.path().join("main.wj");
        if m.exists() {
            mains.push(m);
        }
    }
    mains.sort();
    assert!(mains.len() > 10, "project corpus unexpectedly small: {}", mains.len());
    let mut hits = Vec::new();
    for m in &mains {
        let d = predicted_widths(m);
        if d.is_empty() {
            continue;
        }
        // oracle: rustc on the whole generated crate (its modules are files next to main.rs)
        let out = tempfile::tempdir().unwrap();
        windjammer::build_project(m, out.path(), windjammer::CompilationTarget::Rust, false).unwrap();
        let err = rustc_rejects_file(&out.path().join("main.rs"));
        let err = match err {
            Some(e) => e,
            None => panic!("false positive, rustc accepts {}: {d:?}", m.display()),
        };
        assert!(err.contains("E0308"), "{}: {err}", m.display());
        hits.push(m.display().to_string());
    }
    assert!(hits.is_empty(), "project programs with a width prediction (confirm and pin them): {hits:?}");
}

#[test]
fn flagged_goldens_are_ones_rustc_rejects() {
    let root = root();
    let rejects = std::fs::read_to_string(root.join("tests/golden/rustc_rejects.txt")).unwrap();
    let mut goldens = Vec::new();
    files_with_ext(&root.join("tests/golden/rust"), "golden", &mut goldens);
    assert!(goldens.len() > 100, "golden corpus unexpectedly small: {}", goldens.len());
    for g in goldens {
        let rust = std::fs::read_to_string(&g).unwrap();
        if clashes(&rust).is_empty() {
            continue;
        }
        let rel = g.strip_prefix(root.join("tests/golden/rust")).unwrap().display().to_string();
        let src = rel.trim_end_matches(".golden");
        assert!(rejects.lines().any(|l| l.starts_with(&format!("{src} |"))), "{src}: flagged, but rustc accepts its golden");
    }
}
