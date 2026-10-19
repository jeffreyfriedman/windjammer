//! ADR-0018: `wj fix` optional-sugar migration is idempotent, keeps programs parsing and running, and the
//! migrated program prints the same output as the original (interpreter oracle).

use std::process::Command;
use tempfile::TempDir;

const BEFORE: &str = r#"fn lookup(k: string) -> Option<int> {
    if k == "a" {
        Some(2)
    } else {
        None
    }
}

fn main() {
    let a: Option<int> = lookup("a")
    let n = a.unwrap_or(0)
    let m = lookup("z").unwrap_or(7)
    let k = lookup("z").unwrap_or_else(|| 40 + 2)
    let t = 1 + a.unwrap_or(5)
    let u = a.unwrap()
    println("${n} ${m} ${k} ${t} ${u}")
}
"#;

fn wj(args: &[&str], dir: &std::path::Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(dir).output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr))
}

#[test]
fn fix_migrates_and_preserves_behaviour() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("main.wj");
    std::fs::write(&f, BEFORE).unwrap();
    let (ok, want) = wj(&["run", "--interpret", "main.wj"], dir.path());
    assert!(ok, "{want}");
    let want: Vec<&str> = want.lines().filter(|l| l.contains(' ') && !l.starts_with("warn") && !l.contains("-->")).collect();

    let (ok, out) = wj(&["fix", "main.wj"], dir.path());
    assert!(ok, "{out}");
    let fixed = std::fs::read_to_string(&f).unwrap();
    assert!(fixed.contains("-> int? {"), "{fixed}");
    assert!(fixed.contains("let a: int? = lookup(\"a\")"), "{fixed}");
    assert!(fixed.contains("let n = a ?? 0"), "{fixed}");
    assert!(fixed.contains("let m = lookup(\"z\") ?? 7"), "{fixed}");
    assert!(fixed.contains("let k = lookup(\"z\") ?? 40 + 2"), "{fixed}");
    assert!(fixed.contains("let t = 1 + a.unwrap_or(5)"), "inside arithmetic: left alone\n{fixed}");
    assert!(fixed.contains("let u = a.unwrap()"), ".unwrap() is never rewritten\n{fixed}");

    let (ok, got) = wj(&["run", "--interpret", "main.wj"], dir.path());
    assert!(ok, "{got}");
    let got: Vec<&str> = got.lines().filter(|l| l.contains(' ') && !l.starts_with("warn") && !l.contains("-->")).collect();
    assert_eq!(got, want, "behaviour changed by wj fix");

    // idempotent: a second run changes nothing, and --check agrees
    let (ok, out) = wj(&["fix", "--check", "main.wj"], dir.path());
    assert!(ok, "second fix would still change the file: {out}");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), fixed);
}
