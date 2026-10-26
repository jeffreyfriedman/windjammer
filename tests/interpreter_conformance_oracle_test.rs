#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! The interpreter against the cross-backend conformance corpus oracles. Every `// expect:` line of
//! `tests/conformance_programs/*.wj` was produced by rustc from a Rust transliteration (independent of every
//! Windjammer backend); the interpreter must print exactly that, and exit with the `// exit:` code. R2 of the
//! interpreter-readiness scorecard (docs/design/interpreter-primary-readiness.md) counts a miss as a silent divergence.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance_programs")
}

fn expected(src: &str) -> (String, i32) {
    let mut out = String::new();
    let mut exit = 0;
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("// expect:") {
            out.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            out.push('\n');
        } else if let Some(rest) = line.strip_prefix("// exit:") {
            exit = rest.trim().parse().expect("exit code");
        }
    }
    (out, exit)
}

fn interpret(path: &std::path::Path) -> (String, String, i32) {
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(path).output().expect("spawn wj");
    (String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into(), o.status.code().unwrap_or(-1))
}

/// Header-only programs (no `.meta` seeds, no stdin) run interpreted must match their rustc-derived oracle.
#[test]
fn interpreter_matches_every_conformance_oracle() {
    let mut files: Vec<PathBuf> = fs::read_dir(corpus()).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    let mut bad = Vec::new();
    let mut checked = 0;
    for f in files {
        if f.extension().is_none_or(|e| e != "wj") || f.with_extension("wj.meta").exists() {
            continue;
        }
        let src = fs::read_to_string(&f).unwrap();
        if !src.contains("// expect:") {
            continue;
        }
        let (want, exit) = expected(&src);
        let (got, _, code) = interpret(&f);
        checked += 1;
        if got != want || code != exit {
            bad.push(format!("{} (exit {code} vs {exit})", f.file_name().unwrap().to_string_lossy()));
        }
    }
    assert!(checked >= 100, "only {checked} programs checked");
    assert!(bad.is_empty(), "interpreter output differs from the conformance oracle:\n{}", bad.join("\n"));
}

/// `panic("msg {}", x)` is a template plus arguments on the compiled path too (it used to emit `panic!("msg {{}}")`).
#[test]
fn compiled_panic_formats_its_arguments() {
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("p.wj");
    fs::write(&f, "fn main() {\n    let x = 5\n    panic(\"too large: {} {}\", x, x + 1)\n}\n").unwrap();
    let out = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo", "--no-lint"]).arg(&f).current_dir(dir.path()).output().unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let rs = fs::read_to_string(dir.path().join("build/p.rs")).unwrap();
    assert!(rs.contains("panic!(\"too large: {} {}\", x, x + 1_i32)"), "generated Rust:\n{rs}");
}

/// Several `impl From<S> for T`: `T::from`, `.into()` with a declared type and `?` choose by the argument's type.
#[test]
fn interpreter_picks_the_from_impl_by_argument_type() {
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("f.wj");
    fs::write(
        &f,
        "enum A { P(string), C(int) }\nimpl From<string> for A { fn from(s: string) -> A { A::P(s) } }\nimpl From<int> for A { fn from(c: int) -> A { A::C(c) } }\nfn main() {\n    let a: A = 7.into()\n    let b: A = A::from(\"x\")\n    let n = 8\n    let c: A = n.into()\n    println(\"${a:?} ${b:?} ${c:?}\")\n}\n",
    )
    .unwrap();
    let (out, err, code) = interpret(&f);
    assert_eq!((out.as_str(), code), ("C(7) P(\"x\") C(8)\n", 0), "{err}");
}
