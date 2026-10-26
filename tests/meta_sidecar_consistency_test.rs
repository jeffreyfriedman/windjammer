//! A `<file>.wj.meta` sidecar is stale analysis output, not a source of truth. With a deliberately stale
//! sidecar next to a file (written for an older version of the same source), `wj build` and
//! `wj explain-ownership` must answer exactly as they do with no sidecar at all.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const V1: &str = "fn f(v: Vec<int>) -> Vec<int> {\n    v\n}\n\nfn main() {\n    let data = [1, 2, 3]\n    let w = f(data)\n    println(\"${w.len()}\")\n}\n";
const V2: &str = "fn f(v: Vec<int>) -> int {\n    v.len()\n}\n\nfn main() {\n    let data = [1, 2, 3]\n    let n = f(data)\n    println(\"${n}\")\n}\n";

fn build(dir: &Path, file: &str) -> String {
    let out = dir.join("out");
    let o = Command::new(WJ)
        .args(["build", file, "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(o.status.success(), "build failed: {}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    std::fs::read_to_string(out.join("prog.rs")).unwrap()
}

fn explain(dir: &Path, file: &str) -> String {
    let o = Command::new(WJ).args(["explain-ownership", file, "--json"]).current_dir(dir).output().unwrap();
    assert!(o.status.success(), "explain failed: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).replace(&dir.display().to_string(), "<dir>")
}

#[test]
fn stale_meta_sidecar_changes_neither_build_nor_explain() {
    let stale = tempfile::tempdir().unwrap();
    let clean = tempfile::tempdir().unwrap();

    // Produce a sidecar for V1 …
    std::fs::write(stale.path().join("prog.wj"), V1).unwrap();
    build(stale.path(), "prog.wj");
    let meta = stale.path().join("prog.wj.meta");
    assert!(meta.exists(), "build did not write the sidecar this gate depends on");
    // … then edit the source to V2 and keep the (now stale) sidecar.
    std::fs::write(stale.path().join("prog.wj"), V2).unwrap();
    std::fs::remove_dir_all(stale.path().join("out")).unwrap();

    std::fs::write(clean.path().join("prog.wj"), V2).unwrap();

    assert_eq!(build(stale.path(), "prog.wj"), build(clean.path(), "prog.wj"), "wj build differs with a stale sidecar");
    assert_eq!(explain(stale.path(), "prog.wj"), explain(clean.path(), "prog.wj"), "explain-ownership differs with a stale sidecar");
}

#[test]
fn sidecar_from_another_compiler_build_is_ignored() {
    // A sidecar fingerprinted by a different compiler identity must not be trusted either.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("prog.wj"), V2).unwrap();
    let clean = build(dir.path(), "prog.wj");
    let meta = dir.path().join("prog.wj.meta");
    let text = std::fs::read_to_string(&meta).unwrap();
    let forged = text.replacen("\"compiler_version\": \"", "\"compiler_version\": \"other-build-", 1);
    assert_ne!(text, forged, "sidecar has no compiler_version to forge");
    std::fs::write(&meta, forged.replace("\"param_ownership\": [\n        \"Borrowed\"", "\"param_ownership\": [\n        \"Owned\"")).unwrap();
    std::fs::remove_dir_all(dir.path().join("out")).unwrap();
    assert_eq!(build(dir.path(), "prog.wj"), clean);
}
