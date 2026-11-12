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

//! Interpreter correctness fixes found by earlier agents: each test runs a small program through
//! `wj run --interpret` and compares with what the std declaration / compiled runtime does.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Run {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run(dir: &std::path::Path, src: &str) -> Run {
    std::fs::write(dir.join("p.wj"), src).unwrap();
    let o = Command::new(WJ).current_dir(dir).args(["run", "--interpret", "--no-lint", "p.wj"]).output().unwrap();
    Run { stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into(), code: o.status.code().unwrap_or(-1) }
}

/// `std/fs.wj` declares `DirEntry { name, path, .. }` and `Metadata { size, .. }`: a field read gives what the
/// accessor method gives (it used to panic with `cannot project Field(name)`).
#[test]
fn dir_entry_name_field_reads_like_the_name_method() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(data.join("one.txt"), "12345").unwrap();
    let src = "use std::fs\n\nfn main() {\n    match fs::read_dir(\"data\") {\n        Ok(entries) => {\n            for e in entries {\n                println(\"field=${e.name} method=${e.name()} path=${e.path}\")\n                match e.metadata() {\n                    Ok(m) => println(\"size=${m.size} ${m.size()} file=${m.is_file}\"),\n                    Err(x) => println(\"meta error ${x}\"),\n                }\n            }\n        }\n        Err(x) => println(\"error ${x}\"),\n    }\n}\n";
    let r = run(dir.path(), src);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let line = r.stdout.lines().next().unwrap_or_default();
    assert!(line.starts_with("field=one.txt method=one.txt path="), "{}", r.stdout);
    assert!(r.stdout.contains("size=5 5 file=true"), "{}", r.stdout);
}

/// Compiled (`#[serde(deny_unknown_fields)]` on a struct with named fields, none on an enum or an empty struct):
/// the interpreter denies in exactly those cases (ADR-0027).
#[test]
fn json_unknown_fields_follow_the_compiled_derive() {
    let dir = tempfile::tempdir().unwrap();
    let src = r#"use std::json

struct Point {
    x: int,
}

struct Empty {
}

enum Shape {
    Rect { w: int, h: int },
}

fn main() {
    let a: Result<Point, string> = json::parse_string("{\"x\":1,\"y\":2}")
    println("struct: ${a.is_err()}")
    let b: Result<Empty, string> = json::parse_string("{\"y\":2}")
    println("empty: ${b.is_ok()}")
    let c: Result<Shape, string> = json::parse_string("{\"Rect\":{\"w\":1,\"h\":2,\"depth\":3}}")
    println("variant: ${c.is_ok()}")
}
"#;
    let r = run(dir.path(), src);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(r.stdout, "struct: true\nempty: true\nvariant: true\n", "{}", r.stderr);
}

/// `items(n)?.len()` reads `?.` as optional chaining; on a `Result` it must be `?` then `.len()` (compiled
/// build), and `Err` must return from the function. `?.` on an `Option` is still optional chaining (ADR-0018).
#[test]
fn question_dot_on_a_result_is_try_then_method() {
    let dir = tempfile::tempdir().unwrap();
    let src = r#"fn items(n: int) -> Result<Vec<int>, string> {
    if n < 0 {
        return Err("negative")
    }
    let mut v: Vec<int> = Vec::new()
    let mut i = 0
    while i < n {
        v.push(i)
        i = i + 1
    }
    Ok(v)
}

fn count(n: int) -> Result<int, string> {
    Ok(items(n)?.len() as int)
}

fn shout(name: Option<string>) -> Option<string> {
    name?.to_uppercase()
}

fn show(r: Result<int, string>) {
    match r {
        Ok(n) => println("ok ${n}"),
        Err(e) => println("err ${e}"),
    }
}

fn show_opt(o: Option<string>) {
    match o {
        Some(n) => println("some ${n}"),
        None => println("none"),
    }
}

fn main() {
    show(count(3))
    show(count(-1))
    show_opt(shout(Some("abcd")))
    show_opt(shout(None))
}
"#;
    let r = run(dir.path(), src);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(r.stdout, "ok 3\nerr negative\nsome ABCD\nnone\n", "{}", r.stderr);
}

/// The compiled side of the same finding: `items(n)?.len()` on a `Result` receiver is emitted as `?` followed by
/// the member (it used to be refused as optional chaining without a receiver type).
#[test]
fn question_dot_on_a_result_is_emitted_as_try_then_member() {
    let dir = tempfile::tempdir().unwrap();
    let src = "fn items(n: int) -> Result<Vec<int>, string> {\n    if n < 0 {\n        return Err(\"negative\")\n    }\n    Ok(Vec::new())\n}\n\nfn count(n: int) -> Result<int, string> {\n    Ok(items(n)?.len() as int)\n}\n\nfn main() {\n    match count(1) {\n        Ok(n) => println(\"${n}\"),\n        Err(e) => println(e),\n    }\n}\n";
    std::fs::write(dir.path().join("p.wj"), src).unwrap();
    let out = dir.path().join("out");
    let o = Command::new(WJ).current_dir(dir.path()).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg("p.wj").output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rs = std::fs::read_to_string(out.join("p.rs")).unwrap();
    assert!(rs.contains("let _oc = items(n)?;"), "{rs}");
    assert!(!rs.contains("unreachable!()"), "{rs}");
}
