//! W0005 precision: an explicit `.clone()` is reported as redundant only where the compiler would not need it.
//!
//! The recorded case: a `string` moved into an `if` branch inside a loop. The compiler used to emit the move without a clone, and rustc
//! rejected it (E0382), so the clone the author wrote was the only thing that kept the program building; W0005 asked the author to
//! delete it. Since C6 the compiler copies there itself (liveness: the next iteration reads the value again), so without the clone the
//! program builds too; W0005's token rule still stays quiet for such a clone (a missed warning, never a wrong one).

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const LOOP_MOVE: &str = "fn take(s: string, n: int) -> int {\n    s.len() + n\n}\n\nfn run(mode: string) -> int {\n    let base = if mode == \"caps\" { \"off\" } else { \"plain\" }\n    let test = if mode == \"caps\" { \"enforce\" } else { \"record\" }\n    let mut total = 0\n    let mut r = 0\n    while r < 2 {\n        for k in [0, 1, 1, 0] {\n            let arm = if k == 0 { BASE } else { TEST }\n            total += take(arm, k)\n        }\n        r += 1\n    }\n    total\n}\n\nfn main() {\n    println(\"${run(\"caps\")}\")\n}\n";

fn project(src: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(d.path().join("src/main.wj"), src).unwrap();
    d
}

fn lint(d: &Path) -> String {
    let o = Command::new(WJ).args(["lint", "src/main.wj"]).current_dir(d).output().unwrap();
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn rust_of(d: &Path) -> String {
    let o = Command::new(WJ).args(["build", "--no-cargo", "-o", "out", "."]).current_dir(d).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    std::fs::read_to_string(d.join("out/main.rs")).unwrap()
}

/// C6: without the clone the compiler copies the value itself in the branch, because the next iteration reads it again.
#[test]
fn without_the_clone_the_compiler_copies_in_the_loop() {
    let d = project(&LOOP_MOVE.replace("BASE", "base").replace("TEST", "test"));
    let rs = rust_of(d.path());
    let arm = &rs[rs.find("let arm").unwrap()..];
    let arm = &arm[..arm.find("};").unwrap()];
    assert!(arm.contains("base.clone()") && arm.contains("test.clone()"), "{arm}");
}

#[test]
fn w0005_stays_quiet_for_a_clone_the_compiler_would_not_insert() {
    let d = project(&LOOP_MOVE.replace("BASE", "base.clone()").replace("TEST", "test.clone()"));
    let rs = rust_of(d.path());
    assert!(rs.contains("base.clone()") && rs.contains("test.clone()"), "the clones are what keeps rustc from E0382:\n{rs}");
    let out = lint(d.path());
    assert!(!out.contains("W0005"), "W0005 asked for a clone to be removed that the program needs:\n{out}");
}

#[test]
fn w0005_still_fires_for_a_clone_of_a_last_use() {
    let d = project("fn take(s: string) -> int {\n    s.len()\n}\n\nfn main() {\n    let a = \"x\".to_string()\n    let b = a.clone()\n    println(\"${take(b)}\")\n}\n");
    let out = lint(d.path());
    assert!(out.contains("W0005"), "a clone of a value that is never used again is redundant:\n{out}");
}
