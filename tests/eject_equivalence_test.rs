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
const EJECT_AVAILABLE: bool = false;

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
        let eo = Command::new(WJ).arg("eject").arg(f).arg(&e).output().unwrap();
        if !bo.status.success() {
            continue;
        }
        let (mut bs, mut es) = (Vec::new(), Vec::new());
        rs_files(&b, &mut bs);
        rs_files(&e, &mut es);
        // Every Rust file the build emits must appear, code-identical, in the ejected project.
        let same = bs.iter().all(|(_, c)| es.iter().any(|(_, d)| d == c));
        if !eo.status.success() || !same {
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
