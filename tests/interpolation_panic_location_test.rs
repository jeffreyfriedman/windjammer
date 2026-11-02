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

//! Span oracle for runtime diagnostics: a panic inside `${..}` is reported at the file and the line and column of
//! the expression in the source, not at the synthetic key columns interpolated expressions are filed under.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// `(file, line, column)` of the `panicked at file:line:col:` line.
fn panic_site(src: &str) -> (String, usize, usize) {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("p.wj");
    std::fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(&f).output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    let line = err.lines().find(|l| l.contains("panicked at ")).unwrap_or_else(|| panic!("no panic line in:\n{err}"));
    let site = line.split("panicked at ").nth(1).unwrap().trim_end_matches(':');
    let mut parts = site.rsplitn(3, ':');
    let col: usize = parts.next().unwrap().parse().unwrap_or_else(|_| panic!("bad site {site:?}"));
    let ln: usize = parts.next().unwrap().parse().unwrap();
    let file = parts.next().unwrap().to_string();
    assert!(file.ends_with("p.wj"), "file name missing from {site:?}");
    (file, ln, col)
}

#[test]
fn panic_in_interpolation_reports_the_source_line_and_a_source_column() {
    let src = "fn main() {\n    let v: Vec<int> = vec![1]\n    println(\"x ${v[5]}\")\n}\n";
    let (_, line, col) = panic_site(src);
    assert_eq!(line, 3, "line of the interpolated expression");
    let width = src.lines().nth(2).unwrap().len();
    assert!((1..=width + 1).contains(&col), "column {col} is not a column of line 3 (width {width})");
}

#[test]
fn the_report_is_the_same_on_every_run() {
    let src = "fn main() {\n    let v: Vec<int> = vec![1]\n    println(\"x ${v[5]}\")\n}\n";
    let a = panic_site(src);
    let b = panic_site(src);
    assert_eq!((a.1, a.2), (b.1, b.2));
}
