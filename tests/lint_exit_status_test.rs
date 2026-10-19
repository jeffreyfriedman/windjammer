//! `wj lint` must exit non-zero when it reports an error-level finding (W0001/W0002/...), not only
//! with `--strict`; warnings alone still exit 0.

use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn lint(src: &str, strict: bool) -> (bool, String) {
    let d = TempDir::new().unwrap();
    let f = d.path().join("l.wj");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(WJ);
    c.arg("lint").arg(&f);
    if strict {
        c.arg("--strict");
    }
    let o = c.output().unwrap();
    (o.status.success(), String::from_utf8_lossy(&o.stdout).to_string())
}

#[test]
fn error_level_findings_fail_the_command() {
    let (ok, out) = lint("fn main() {\n    let v: Option<int> = Some(1)\n    println(\"${v.unwrap()}\")\n}\n", false);
    assert!(!ok, "W0002 is an error and must fail `wj lint`: {out}");
    let (ok, _) = lint("fn total(items: &Vec<int>) -> int {\n    0\n}\n\nfn main() {\n    println(\"${total([1])}\")\n}\n", false);
    assert!(!ok, "W0001 must fail `wj lint`");
}

#[test]
fn clean_and_warning_only_files_pass() {
    let (ok, _) = lint("fn main() {\n    println(\"hi\")\n}\n", false);
    assert!(ok);
    let (ok, _) = lint("fn main() {\n    let s = \"a\"\n    let t = s.clone()\n    println(\"${t}\")\n}\n", false);
    assert!(ok, "W0005 is only a warning");
    let (ok, _) = lint("fn main() {\n    let s = \"a\"\n    let t = s.clone()\n    println(\"${t}\")\n}\n", true);
    assert!(!ok, "--strict fails on warnings");
}
