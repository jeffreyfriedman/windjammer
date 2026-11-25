//! `std::test` is the one assertion module (ADR-0039): every canonical assertion passes and fails correctly, and a
//! failing assertion panics with the same message when interpreted and when compiled. The removed spellings are
//! gone (WJ0002 with a machine-applicable rename), and `wj fix` rewrites them to the canonical names.

use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// Every failing case: (case name, the statement that fails, the exact panic message both modes print).
const FAILING: &[(&str, &str, &str)] = &[
    ("gt", "assert_gt(1, 2)", "assertion failed: left > right\n  left: 1\n right: 2"),
    ("gt_equal", "assert_gt(2, 2)", "assertion failed: left > right\n  left: 2\n right: 2"),
    ("lt", "assert_lt(2, 1)", "assertion failed: left < right\n  left: 2\n right: 1"),
    ("ge", "assert_ge(1, 2)", "assertion failed: left >= right\n  left: 1\n right: 2"),
    ("le", "assert_le(2, 1)", "assertion failed: left <= right\n  left: 2\n right: 1"),
    (
        "approx64",
        "assert_approx_eq(3.5, 2.0, 0.5)",
        "assertion failed: values not approximately equal\n  left: 3.5\n  right: 2\n  epsilon: 0.5\n  diff: 1.5",
    ),
    (
        "approx32",
        "let a: f32 = 0.3\n        let b: f32 = 0.1\n        let e: f32 = 0.1\n        assert_approx_eq(a, b, e)",
        "assertion failed: values not approximately equal\n  left: 0.3\n  right: 0.1\n  epsilon: 0.1\n  diff: 0.20000002",
    ),
    ("in_range", "assert_in_range(15, 0, 10)", "assertion failed: value not in range\n  value: 15\n  min: 0\n  max: 10"),
    ("some", "let v = assert_some(no_int())", "assertion failed: expected Some, got None"),
    ("none", "assert_none(some_int())", "assertion failed: expected None, got Some(7)"),
    ("ok", "let v = assert_ok(parse(\"nope\"))", "assertion failed: expected Ok, got Err(\"bad\")"),
    ("err", "let e = assert_err(parse(\"ok\"))", "assertion failed: expected Err, got Ok(5)"),
    (
        "contains_str",
        "assert_contains(\"hello world\", \"foo\")",
        "assertion failed: string does not contain substring\n  string: \"hello world\"\n  substring: \"foo\"",
    ),
    (
        "contains_list",
        "let xs = [1, 2, 3]\n        assert_contains(xs, 9)",
        "assertion failed: collection does not contain item\n  collection: [1, 2, 3]\n  item: 9",
    ),
    (
        "starts_with",
        "assert_starts_with(\"hello\", \"lo\")",
        "assertion failed: string does not start with prefix\n  string: \"hello\"\n  prefix: \"lo\"",
    ),
    (
        "ends_with",
        "assert_ends_with(\"hello\", \"he\")",
        "assertion failed: string does not end with suffix\n  string: \"hello\"\n  suffix: \"he\"",
    ),
    ("empty", "let xs = [1]\n        assert_empty(xs)", "assertion failed: expected empty, got length 1"),
    ("not_empty", "let xs: Vec<int> = []\n        assert_not_empty(xs)", "assertion failed: expected non-empty, got length 0"),
    ("length", "let xs = [1, 2, 3]\n        assert_length(xs, 5)", "assertion failed: length mismatch\n  expected: 5\n  actual: 3"),
];

fn program() -> String {
    let mut arms = String::new();
    for (name, stmt, _) in FAILING {
        arms.push_str(&format!("    if which == \"{name}\" {{\n        {stmt}\n    }}\n"));
    }
    format!(
        r#"use std::test::*
use std::env

fn parse(s: string) -> Result<int, string> {{
    if s == "ok" {{ Ok(5) }} else {{ Err("bad") }}
}}

fn no_int() -> int? {{
    None
}}

fn some_int() -> int? {{
    Some(7)
}}

fn passing() {{
    assert_gt(3, 2)
    assert_lt(2, 3)
    assert_ge(3, 3)
    assert_ge(4, 3)
    assert_le(3, 3)
    assert_le(2, 3)
    assert_approx_eq(1.0, 1.0004, 0.001)
    let a: f32 = 1.0
    let b: f32 = 1.0004
    let e: f32 = 0.001
    assert_approx_eq(a, b, e)
    assert_in_range(5, 0, 10)
    let s = assert_some(some_int())
    assert_eq(s, 7)
    assert_none(no_int())
    let o = assert_ok(parse("ok"))
    assert_eq(o, 5)
    let m = assert_err(parse("nope"))
    assert_eq(m, "bad")
    assert_contains("hello world", "o w")
    let xs = [1, 2, 3]
    assert_contains(xs, 3)
    assert_starts_with("hello", "he")
    assert_ends_with("hello", "llo")
    let empty: Vec<int> = []
    assert_empty(empty)
    assert_not_empty(xs)
    assert_length(xs, 3)
    println("all passed")
}}

fn main() {{
    let args = env::args()
    let which = args[1]
    if which == "pass" {{
        passing()
    }}
{arms}}}
"#
    )
}

fn run(dir: &TempDir, mode: &str, case: &str) -> Output {
    Command::new(WJ).args(["run", mode, "prog.wj", case]).current_dir(dir.path()).output().unwrap()
}

/// The panic message: from `assertion failed` to the line before the backtrace / note.
fn panic_message(stderr: &str) -> String {
    let mut lines = Vec::new();
    let mut on = false;
    for l in stderr.lines() {
        if l.starts_with("assertion failed") {
            on = true;
        }
        if on && (l.starts_with("stack backtrace") || l.starts_with("note:") || l.trim().is_empty()) {
            break;
        }
        if on {
            lines.push(l);
        }
    }
    lines.join("\n")
}

#[test]
fn every_assertion_passes_and_fails_with_the_same_message_in_both_modes() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("prog.wj"), program()).unwrap();
    for mode in ["--interpret", "--compiled"] {
        let o = run(&dir, mode, "pass");
        let (out, err) = (String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        assert!(o.status.success() && out.trim() == "all passed", "{mode}: the passing cases must pass\n{out}\n{err}");
    }
    let mut report = Vec::new();
    for (case, _, want) in FAILING {
        for mode in ["--interpret", "--compiled"] {
            let o = run(&dir, mode, case);
            let err = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                report.push(format!("{case} {mode}: the failing assertion did not fail"));
            } else if panic_message(&err) != *want {
                report.push(format!("{case} {mode}: message differs\n  want: {want:?}\n  got:  {:?}\n{err}", panic_message(&err)));
            }
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}

#[test]
fn removed_spellings_are_gone_with_a_machine_applicable_rename() {
    let dir = TempDir::new().unwrap();
    let src = "use std::test::*\n\nfn parse(s: string) -> Result<int, string> {\n    Ok(5)\n}\n\nfn main() {\n    assert_is_ok(parse(\"a\"))\n    assert_gte(2, 1)\n}\n";
    let f = dir.path().join("old.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["check", "--error-format", "agent"]).arg(&f).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(!o.status.success(), "removed names must not check clean:\n{text}");
    assert!(text.contains("rename to `assert_ok`") && text.contains("rename to `assert_ge`"), "each diagnostic carries the rename:\n{text}");
    // `wj check --fix` and `wj fix` land on the same text
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    fs::write(a.join("old.wj"), src).unwrap();
    fs::write(b.join("old.wj"), src).unwrap();
    Command::new(WJ).args(["check", "--fix"]).arg(a.join("old.wj")).output().unwrap();
    let fixed = Command::new(WJ).args(["fix"]).arg(b.join("old.wj")).output().unwrap();
    assert!(fixed.status.success(), "{}", String::from_utf8_lossy(&fixed.stderr));
    let (ta, tb) = (fs::read_to_string(a.join("old.wj")).unwrap(), fs::read_to_string(b.join("old.wj")).unwrap());
    assert!(tb.contains("assert_ok(parse(\"a\"))") && tb.contains("assert_ge(2, 1)"), "wj fix: {tb}");
    assert_eq!(ta, tb, "wj check --fix and wj fix agree");
    let o = Command::new(WJ).args(["check"]).arg(b.join("old.wj")).output().unwrap();
    assert!(o.status.success(), "the fixed file checks clean: {}", String::from_utf8_lossy(&o.stderr));
}

/// Golden: `wj fix` over every removed spelling. Strings, comments, methods and the program's own functions stay.
#[test]
fn wj_fix_rewrites_every_removed_name_golden() {
    let dir = TempDir::new().unwrap();
    let before = r#"use std::testing

// assert_gte(a, b) in a comment stays
fn assert_approx(label: string) {
    println(label)
}

fn t(r: Result<int, string>, o: int?, xs: Vec<int>) {
    assert_is_ok(&r)
    assert_is_err(&r)
    assert_result_ok(&r)
    assert_result_err(&r)
    assert_is_some(&o)
    assert_is_none(&o)
    assert_gte(2, 1)
    assert_lte(1, 2)
    assert_approx_f32(1.0, 1.0, 0.1)
    assert_approx_f64(1.0, 1.0, 0.1)
    assert_msg(true, "why")
    assert_with_message(true, "why")
    assert_str_contains("abc", "b")
    assert_deep_eq(xs, xs)
    testing::assert_with_message(true, "m")
    assert_approx("own helper stays")
    println("assert_gte( in a string stays")
    for i in (0..3).reversed() {
        println("${i}")
    }
    for i in (0..=3).reversed() {
        println("${i}")
    }
    let ys = xs.reversed()
}
"#;
    let after = r#"use std::test

// assert_gte(a, b) in a comment stays
fn assert_approx(label: string) {
    println(label)
}

fn t(r: Result<int, string>, o: int?, xs: Vec<int>) {
    assert_ok(r)
    assert_err(r)
    assert_ok(r)
    assert_err(r)
    assert_some(o)
    assert_none(o)
    assert_ge(2, 1)
    assert_le(1, 2)
    assert_approx_eq(1.0, 1.0, 0.1)
    assert_approx_eq(1.0, 1.0, 0.1)
    assert(true, "why")
    assert(true, "why")
    assert_contains("abc", "b")
    assert_eq(xs, xs)
    test::assert(true, "m")
    assert_approx("own helper stays")
    println("assert_gte( in a string stays")
    for i in (0..3).rev() {
        println("${i}")
    }
    for i in (0..=3).rev() {
        println("${i}")
    }
    let ys = xs.reversed()
}
"#;
    let f = dir.path().join("g.wj");
    fs::write(&f, before).unwrap();
    let o = Command::new(WJ).args(["fix"]).arg(&f).output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let got = fs::read_to_string(&f).unwrap();
    // `wj fix` also adds `mut` and the like elsewhere; the std::test rewrites are what this golden pins
    assert_eq!(got, after);
    let again = Command::new(WJ).args(["fix", "--check"]).arg(&f).output().unwrap();
    assert!(again.status.success(), "a second run changes nothing: {}", String::from_utf8_lossy(&again.stdout));
}

/// `(a..b).reversed()` and a qualified `test::assert_is_ok(..)` (no glob import): both engines apply the same rename.
#[test]
fn check_fix_renames_qualified_calls_and_range_reversed() {
    let dir = TempDir::new().unwrap();
    let src = "use std::test\n\nfn parse(s: string) -> Result<int, string> {\n    Ok(5)\n}\n\nfn main() {\n    test::assert_is_ok(parse(\"a\"))\n    for i in (0..3).reversed() {\n        println(\"${i}\")\n    }\n}\n";
    let f = dir.path().join("q.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["check", "--error-format", "agent"]).arg(&f).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(text.contains("WJ0002") && text.contains("WJ0015"), "both removed spellings are diagnosed:\n{text}");
    let fixed = dir.path().join("fixed.wj");
    fs::write(&fixed, src).unwrap();
    Command::new(WJ).args(["check", "--fix"]).arg(&fixed).output().unwrap();
    let want = "use std::test\n\nfn parse(s: string) -> Result<int, string> {\n    Ok(5)\n}\n\nfn main() {\n    test::assert_ok(parse(\"a\"))\n    for i in (0..3).rev() {\n        println(\"${i}\")\n    }\n}\n";
    assert_eq!(fs::read_to_string(&fixed).unwrap(), want, "wj check --fix");
    let by_fix = dir.path().join("by_fix.wj");
    fs::write(&by_fix, src).unwrap();
    Command::new(WJ).args(["fix"]).arg(&by_fix).output().unwrap();
    assert_eq!(fs::read_to_string(&by_fix).unwrap(), want, "wj fix");
}

/// One `T` for every operand: a float literal beside an `f32` value is `f32` in the emitted Rust (not the `f64`
/// default), for the comparisons and for `assert_approx_eq`.
#[test]
fn float_literals_take_the_width_of_the_other_operand() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("w.wj");
    fs::write(&f, "use std::test::*\n\nfn main() {\n    let x: f32 = 1.0\n    assert_approx_eq(x, 1.5, 0.001)\n    assert_gt(x, 0.5)\n}\n").unwrap();
    let out = dir.path().join("out");
    let o = Command::new(WJ).args(["build"]).arg(&f).arg("-o").arg(&out).output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rs: String = fs::read_dir(&out)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .map(|e| fs::read_to_string(e.path()).unwrap())
        .collect();
    assert!(rs.contains("assert_approx_eq(x, 1.5_f32, 0.001_f32)"), "{rs}");
    assert!(rs.contains("assert_gt(x, 0.5_f32)"), "{rs}");
}
