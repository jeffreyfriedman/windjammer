//! Gate G8 (ADR-0039 decision H): an iterator used as a list is one diagnostic (code, position, fix) from `wj check`,
//! `wj run --interpret` and `wj build`, and the fixed program runs. The compiled run of the fixed program
//! (`WJ_G8_COMPILED=1`, needs cargo) must print what the interpreted run prints.

use std::path::{Path, PathBuf};
use std::process::Command;

fn wj() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_wj"));
    c.env("NO_COLOR", "1");
    c
}

fn cases() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/diagnostics_oracle/check/iter_as_list");
    let mut v: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
    v.sort();
    v
}

/// The WJ0035 diagnostic blocks of a command's output (from the line naming the code to the next blank line), with the
/// directory of the file removed from every path.
fn wj0035(out: &std::process::Output, file: &Path) -> Vec<String> {
    let dir = format!("{}/", file.parent().unwrap().display());
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace(&dir, "");
    let mut blocks = Vec::new();
    let mut cur: Option<String> = None;
    for l in text.lines() {
        match (&mut cur, l.contains("WJ0035") && l.contains("error")) {
            (None, true) => cur = Some(l.to_string()),
            (Some(_), _) if l.trim().is_empty() => blocks.push(cur.take().unwrap()),
            (Some(b), _) => {
                b.push('\n');
                b.push_str(l);
            }
            _ => {}
        }
    }
    blocks.extend(cur);
    blocks.sort();
    blocks
}

#[test]
fn the_three_entry_points_report_the_same_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    for case in cases() {
        let f = dir.path().join(case.file_name().unwrap());
        std::fs::copy(&case, &f).unwrap();
        let check = wj().args(["check"]).arg(&f).output().unwrap();
        let run = wj().args(["run", "--interpret"]).arg(&f).output().unwrap();
        let out_dir = dir.path().join(format!("out-{}", case.file_stem().unwrap().to_string_lossy()));
        let build = wj().args(["build", "--no-cargo"]).arg(&f).arg("-o").arg(&out_dir).output().unwrap();
        let (a, b, c) = (wj0035(&check, &f), wj0035(&run, &f), wj0035(&build, &f));
        assert!(!a.is_empty(), "{}: wj check reports no WJ0035", case.display());
        assert!(a.iter().any(|l| l.contains("collect") || l.contains("count") || l.contains("next")), "{}: no fix text: {a:?}", case.display());
        assert_eq!(a, b, "{}: wj check vs wj run --interpret", case.display());
        assert_eq!(a, c, "{}: wj check vs wj build", case.display());
    }
}

#[test]
fn the_fix_makes_the_program_run_and_both_engines_agree() {
    let dir = tempfile::tempdir().unwrap();
    for case in cases() {
        let f = dir.path().join(case.file_name().unwrap());
        std::fs::copy(&case, &f).unwrap();
        let fixed = wj().args(["check", "--fix"]).arg(&f).output().unwrap();
        let again = wj().args(["check"]).arg(&f).output().unwrap();
        assert!(again.status.success(), "{}: still failing after --fix:\n{}{}\n{}", case.display(), String::from_utf8_lossy(&fixed.stdout), String::from_utf8_lossy(&again.stdout), std::fs::read_to_string(&f).unwrap());
        let interpreted = wj().args(["run", "--interpret"]).arg(&f).output().unwrap();
        assert!(interpreted.status.success(), "{}: the fixed program fails: {}", case.display(), String::from_utf8_lossy(&interpreted.stderr));
        if std::env::var("WJ_G8_COMPILED").is_ok() {
            let compiled = wj().args(["run", "--compiled"]).arg(&f).output().unwrap();
            assert_eq!(interpreted.stdout, compiled.stdout, "{}: engines differ after the fix", case.display());
        }
    }
}

/// The interpreter's defensive error carries the checker's code when a program skipped the check.
#[test]
fn run_time_guard_has_the_same_code() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("g.wj");
    std::fs::write(&f, "fn main() {\n    let n = \"a b\".split(\" \").len()\n    println(\"${n}\")\n}\n").unwrap();
    let out = wj().args(["run", "--interpret"]).arg(&f).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(!out.status.success() && text.contains("WJ0035"), "{text}");
}
