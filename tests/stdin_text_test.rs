//! `wj run --stdin-text '<text>'`: C-style escapes give the program its stdin; interpreted, compiled and
//! `--stdin-file` runs of the same bytes print the same output, and agent mode still ends with `exit status: N`.

use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const PROG: &str = "use std::fs\n\nfn main() {\n    let text = fs::read_to_string(\"/dev/stdin\").unwrap_or(\"\")\n    for line in text.lines() {\n        println(\"[${line}]\")\n    }\n    println(\"bytes ${text.len()}\")\n}\n";

/// Lines, a spreadsheet formula, a tab, a quote, a backslash, an octal `=`, UTF-8.
const TEXT: &str = r#"alpha\n\x3dA2*3\nt\tab \"q\" \\ \75B\n\u{e9}\n"#;

fn run(d: &TempDir, args: &[&str], agent: bool) -> Output {
    let mut c = Command::new(WJ);
    c.args(args).current_dir(d.path()).env_remove("WJ_ERROR_FORMAT").env_remove("WJ_PROGRESS").env_remove("WJ_AGENT");
    if agent {
        c.env("WJ_AGENT", "1");
    }
    c.output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn interpreted_compiled_and_stdin_file_agree() {
    let d = TempDir::new().unwrap();
    fs::write(d.path().join("main.wj"), PROG).unwrap();
    let bytes = "alpha\n=A2*3\nt\tab \"q\" \\ =B\n\u{e9}\n";
    fs::write(d.path().join("in.txt"), bytes).unwrap();

    let want = run(&d, &["run", "--interpret", "--stdin-file", "in.txt", "main.wj"], false);
    assert!(want.status.success(), "{}", String::from_utf8_lossy(&want.stderr));
    assert!(out(&want).contains("[=A2*3]") && out(&want).contains("[\u{e9}]"), "{}", out(&want));

    let interp = run(&d, &["run", "--interpret", "--stdin-text", TEXT, "main.wj"], false);
    assert!(interp.status.success(), "{}", String::from_utf8_lossy(&interp.stderr));
    assert_eq!(out(&interp), out(&want));

    let compiled = run(&d, &["run", "--compiled", "--stdin-text", TEXT, "main.wj"], false);
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    assert_eq!(out(&compiled), out(&want));
}

#[test]
fn agent_mode_still_ends_with_the_exit_status() {
    let d = TempDir::new().unwrap();
    fs::write(d.path().join("main.wj"), PROG).unwrap();
    let o = run(&d, &["run", "--interpret", "--stdin-text", r"x\ny", "main.wj"], true);
    let t = out(&o);
    assert!(t.contains("[x]") && t.contains("[y]") && t.trim_end().ends_with("exit status: 0"), "{t}");
}

#[test]
fn bad_escape_and_conflict_are_errors_not_panics() {
    let d = TempDir::new().unwrap();
    fs::write(d.path().join("main.wj"), PROG).unwrap();
    let o = run(&d, &["run", "--interpret", "--stdin-text", r"ab\q", "main.wj"], false);
    let e = String::from_utf8_lossy(&o.stderr).into_owned();
    assert_eq!(o.status.code(), Some(2), "{e}");
    assert!(e.contains("unknown escape") && e.contains("byte 2") && !e.contains("panicked"), "{e}");
    let o = run(&d, &["run", "--interpret", "--stdin-text", "a", "--stdin-file", "in.txt", "main.wj"], false);
    assert_eq!(o.status.code(), Some(2));
}
