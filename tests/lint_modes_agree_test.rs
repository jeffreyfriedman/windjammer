//! The Rust-leakage lints are one set: `wj lint`, `wj build` and `wj run --interpret` all use
//! `linter::lint_text`. Gate: the interpreter path reports the same codes the shared function returns,
//! and every verified-corpus program is clean under it.

use std::path::PathBuf;
use std::process::Command;
use windjammer::linter::lint_text;

/// Error-level lint (W0002): `wj build` rejects it, so the interpreter refuses it too.
const BAD: &str = "fn f(v: Vec<int>) -> int {\n    let o = v.first()\n    let x = o.unwrap()\n    return x\n}\nfn main() {\n    println(\"${f([1])}\")\n}\n";
/// Warning-level lints only (W0005, W0009): the interpreter reports them and still runs.
const WARN: &str = "fn f(v: Vec<int>) -> int {\n    let w = v.clone()\n    let x = w[0]\n    return x\n}\nfn main() {\n    println(\"${f([1])}\")\n}\n";

#[test]
fn shared_lint_set_reports_leakage_and_style() {
    let codes: Vec<String> = lint_text("t.wj", BAD).unwrap().into_iter().map(|d| d.lint_name).collect();
    assert!(codes.contains(&"W0002".to_string()), "{codes:?}");
    assert!(codes.contains(&"W0009".to_string()), "{codes:?}");
}

fn run_interpreter(src: &str) -> std::process::Output {
    let dir = std::env::temp_dir().join(format!("wj-lintmodes-{}-{}", std::process::id(), src.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("t.wj");
    std::fs::write(&f, src).unwrap();
    Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&f).output().unwrap()
}

#[test]
fn interpreter_reports_the_same_codes() {
    let out = run_interpreter(WARN);
    let err = String::from_utf8_lossy(&out.stderr);
    let codes: Vec<String> = lint_text("t.wj", WARN).unwrap().into_iter().map(|d| d.lint_name).collect();
    assert!(codes.contains(&"W0005".to_string()) && codes.contains(&"W0009".to_string()), "{codes:?}");
    for code in codes {
        assert!(err.contains(&code), "interpreter stderr lacks {code}:\n{err}");
    }
    assert!(String::from_utf8_lossy(&out.stdout).contains('1'), "program still runs");
}

#[test]
fn interpreter_refuses_error_level_lints_like_build() {
    let out = run_interpreter(BAD);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && out.stdout.is_empty(), "stdout={:?}", String::from_utf8_lossy(&out.stdout));
    assert!(err.contains("W0002") && err.contains("not running"), "{err}");
}

#[test]
fn verified_corpus_is_clean_under_the_shared_lints() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/verified");
    let mut bad = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "wj") {
            let src = std::fs::read_to_string(&p).unwrap();
            let ds = lint_text(&p.to_string_lossy(), &src).unwrap_or_default();
            if !ds.is_empty() {
                bad.push(format!("{}: {}", p.file_name().unwrap().to_string_lossy(), ds[0].lint_name));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}
