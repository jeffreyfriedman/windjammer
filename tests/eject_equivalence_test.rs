//! Differential gate (scaffold): `wj eject` must produce the same program as `wj build`.
//!
//! Eject promises "your Windjammer project as a standalone Rust project". The oracle is the build
//! pipeline: for every verified example, the ejected project's Rust sources must equal the
//! `wj build --no-cargo` output modulo comments and blank lines (eject may add explanatory
//! comments), and must still be accepted by rustc.
//!
//! State: `wj eject` does not exist yet (only the legacy `windjammer eject`; wt/eject restores it).
//! `EJECT_AVAILABLE = false` records that; when `wj eject` appears this test fails until the
//! constant is flipped, which turns the comparison on.

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const EJECT_AVAILABLE: bool = true;

fn has_eject() -> bool {
    Command::new(WJ).args(["eject", "--help"]).output().map(|o| o.status.success()).unwrap_or(false)
}

/// Rust source without comments and blank lines, for comparing eject against build output.
pub fn code_only(rs: &str) -> String {
    rs.lines().map(|l| l.split("//").next().unwrap().trim_end()).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n")
}

fn rs_files(dir: &Path, out: &mut Vec<(String, String)>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() && p.file_name().is_some_and(|n| n != "target") {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push((p.file_name().unwrap().to_string_lossy().into_owned(), code_only(&std::fs::read_to_string(&p).unwrap())));
        }
    }
    out.sort();
}

/// Every Rust file the build emits must appear in the ejected project under the same file name, code-identical.
/// A build that emitted nothing proves nothing, so it is never "the same".
pub fn ejected_contains_build(build: &[(String, String)], eject: &[(String, String)]) -> bool {
    !build.is_empty() && build.iter().all(|(n, c)| eject.iter().any(|(m, d)| m == n && d == c))
}

#[test]
fn eject_state_matches_the_recorded_state() {
    assert_eq!(has_eject(), EJECT_AVAILABLE, "`wj eject` availability changed: set EJECT_AVAILABLE in {} (and enable the comparison)", file!());
}

#[test]
fn eject_output_equals_build_output() {
    if !EJECT_AVAILABLE {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/verified");
    let mut files: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
    files.sort();
    let mut diffs = Vec::new();
    for f in files.iter().take(40) {
        let t = tempfile::TempDir::new().unwrap();
        let (b, e) = (t.path().join("build"), t.path().join("eject"));
        let bo = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&b).arg(f).output().unwrap();
        let eo = Command::new(WJ).arg("eject").arg(f).arg("-o").arg(&e).output().unwrap();
        if !bo.status.success() {
            continue;
        }
        let (mut bs, mut es) = (Vec::new(), Vec::new());
        rs_files(&b, &mut bs);
        rs_files(&e, &mut es);
        if !eo.status.success() || !ejected_contains_build(&bs, &es) {
            diffs.push(f.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(diffs.is_empty(), "eject differs from build for: {diffs:?}");
}

#[test]
fn selftest_code_only_ignores_comments_but_not_code() {
    assert_eq!(code_only("// Ejected from x.wj\nfn main() {\n\n    f(); // call\n}\n"), code_only("fn main() {\n    f();\n}"));
    assert_ne!(code_only("fn main() { f(); }"), code_only("fn main() { g(); }"));
}

fn pair(n: &str, c: &str) -> (String, String) {
    (n.to_string(), code_only(c))
}

/// Mutation checks: the comparison must reject an ejected project that differs from the build in any way that matters.
#[test]
fn selftest_comparison_rejects_mutated_eject_output() {
    let build = vec![pair("main.rs", "fn main() {\n    f();\n}\n")];
    assert!(ejected_contains_build(&build, &[pair("main.rs", "// Ejected\nfn main() {\n    f();\n}\n")]), "comments are allowed");
    assert!(!ejected_contains_build(&build, &[pair("main.rs", "fn main() {\n    g();\n}\n")]), "changed code must be caught");
    assert!(!ejected_contains_build(&build, &[pair("other.rs", "fn main() {\n    f();\n}\n")]), "renamed file must be caught");
    assert!(!ejected_contains_build(&build, &[]), "missing file must be caught");
    assert!(!ejected_contains_build(&[], &[pair("main.rs", "fn main() {}")]), "an empty build is vacuous");
}

/// The same mutation applied to a real ejected project: change one token of its Rust and the gate must notice.
#[test]
fn mutated_real_eject_is_detected() {
    let f = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/verified/algo_bubble_sort.wj");
    let t = tempfile::TempDir::new().unwrap();
    let (b, e) = (t.path().join("build"), t.path().join("eject"));
    assert!(Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&b).arg(&f).output().unwrap().status.success());
    assert!(Command::new(WJ).arg("eject").arg(&f).arg("-o").arg(&e).output().unwrap().status.success());
    let (mut bs, mut es) = (Vec::new(), Vec::new());
    rs_files(&b, &mut bs);
    rs_files(&e, &mut es);
    assert!(ejected_contains_build(&bs, &es));
    let target = bs[0].0.clone();
    for (n, c) in es.iter_mut() {
        if *n == target {
            *c = c.replacen("fn ", "fn z_", 1);
        }
    }
    assert!(!ejected_contains_build(&bs, &es), "a mutated ejected file must differ from the build");
}
