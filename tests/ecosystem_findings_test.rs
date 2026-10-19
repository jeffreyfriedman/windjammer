#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! Generic regression gates for findings from porting applications to the ecosystem packages
//! (`docs/known-issues/*.md`). Each test builds a small program through the real CLI and requires
//! the generated Rust to compile and print the hand-derived output.
//! Ownership/width classes owned by the convergence stream are in `ecosystem_findings_handoff_test.rs`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// `wj build` the single file, then `rustc` it. Returns the generated Rust and the run output.
fn build_and_run(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr)));
    }
    let rs = dir.path().join("build/test.rs");
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(&rs).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    if !rustc.status.success() {
        let generated = fs::read_to_string(&rs).unwrap_or_default();
        return Err(format!("rustc failed: {}\n--- generated\n{}", String::from_utf8_lossy(&rustc.stderr), generated));
    }
    let run = Command::new(&bin).output().unwrap();
    Ok(String::from_utf8_lossy(&run.stdout).to_string())
}

fn expect_output(src: &str, expect: &str) {
    let got = build_and_run(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got, expect);
}

/// Generate Rust only (no rustc) for shapes that need runtime crates to link.
fn generated_rust(src: &str) -> String {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(wj.status.success(), "wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr));
    fs::read_to_string(dir.path().join("build/test.rs")).unwrap()
}

// ------------------------------------------------------------------ std idiom lowering

/// E1: `fs.read_to_string(path)` with a `string` parameter is a path call, not a method call.
#[test]
fn fs_read_to_string_with_string_param_is_a_path_call() {
    let rs = generated_rust(
        r#"use std::fs

fn size(path: string) -> int {
    match fs.read_to_string(path) {
        Ok(t) => t.len(),
        Err(_) => 0,
    }
}

fn main() {
    println("${size("Cargo.toml")}")
}
"#,
    );
    assert!(!rs.contains("fs.read_to_string"), "method-call form emitted:\n{rs}");
    assert!(rs.contains("fs::read_to_string(path)"), "expected a path call:\n{rs}");
}

/// E2: a list literal in argument position takes the parameter's type.
#[test]
fn list_literal_argument_takes_vec_parameter_type() {
    expect_output(
        r#"fn count(v: Vec<string>) -> int {
    v.len()
}

fn sum(v: Vec<int>) -> int {
    let mut t = 0
    for x in v {
        t = t + x
    }
    t
}

fn main() {
    println("${count(["a", "b"])} ${sum([1, 2, 3])}")
}
"#,
        "2 6\n",
    );
}

/// E3: `map.get(k).unwrap_or(0)` has exactly one `.copied()`.
#[test]
fn hashmap_get_unwrap_or_literal_copies_once() {
    expect_output(
        r#"use std::collections::HashMap

fn lookup(seen: HashMap<string, int>, key: string) -> int {
    seen.get(key).unwrap_or(0)
}

fn main() {
    let mut m = HashMap::new()
    m.insert("a", 1)
    println("${lookup(m, "a")} ${lookup(m, "b")}")
}
"#,
        "1 0\n",
    );
}

/// E4: `Vec<string>.contains(param)` keeps the parameter callable with a string literal.
#[test]
fn vec_string_contains_param_accepts_string_literal() {
    expect_output(
        r#"struct Room {
    members: Vec<string>,
}

impl Room {
    fn has(self, name: string) -> bool {
        self.members.contains(name)
    }
}

fn main() {
    let mut members = Vec::new()
    members.push("ann")
    let room = Room { members: members }
    println("${room.has("ann")} ${room.has("bob")}")
}
"#,
        "true false\n",
    );
}

/// E5: `std::time::now()` declared and emitted types agree (`timestamp_millis` exists on the result).
#[test]
fn time_now_declared_type_matches_runtime_type() {
    let rs = generated_rust(
        r#"use std::time

fn main() {
    println("${time.now().timestamp_millis()}")
}
"#,
    );
    assert!(rs.contains("timestamp_millis"), "{rs}");
    // the declared return type must be the one the runtime returns
    let decl = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/std/time.wj")).unwrap();
    let rt = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/crates/windjammer-runtime/src/time.rs"))
        .or_else(|_| fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/runtime/time.rs")))
        .unwrap_or_default();
    let decl_line = decl.lines().find(|l| l.contains("fn now(")).unwrap_or("");
    let rt_line = rt.lines().find(|l| l.contains("pub fn now(")).unwrap_or("");
    let ty = |l: &str| l.split("->").nth(1).map(|s| s.trim().trim_end_matches('{').trim().to_string()).unwrap_or_default();
    assert!(!rt_line.is_empty(), "runtime now() not found");
    assert_eq!(ty(decl_line), ty(rt_line), "declared `{decl_line}` vs runtime `{rt_line}`");
}

/// E6: a string literal address for `server_serve` is passed as an owned `String`.
#[test]
fn server_serve_string_literal_address_is_owned() {
    let rs = generated_rust(
        r#"use std::http::*

fn main() {
    match server_serve("0.0.0.0", 8080, |req| ServerResponse::ok("hi")) {
        Ok(_) => {},
        Err(e) => eprintln("${e}"),
    }
}
"#,
    );
    assert!(!rs.contains("&String::from(\"0.0.0.0\")"), "borrowed literal passed to owned String param:\n{rs}");
}

// ------------------------------------------------------------------ type annotations

/// E7: a local `Vec::new()` receiving `String`s is not annotated with the function's return element type.
#[test]
fn local_vec_element_type_not_taken_from_return_type() {
    expect_output(
        r#"struct Delivery {
    conn: int,
}

struct Conn {
    id: int,
    user: string,
}

struct Hub {
    conns: Vec<Conn>,
}

impl Hub {
    fn members(self) -> Vec<Conn> {
        let mut out = Vec::new()
        for c in self.conns {
            out.push(Conn { id: c.id, user: c.user.clone() })
        }
        out
    }

    fn presence(self) -> Vec<Delivery> {
        let members = self.members()
        let mut users = Vec::new()
        for m in members {
            users.push(m.user.clone())
        }
        users.sort()
        let mut out = Vec::new()
        out.push(Delivery { conn: 1 })
        out
    }
}

fn main() {
    let hub = Hub { conns: Vec::new() }
    println("${hub.presence().len()}")
}
"#,
        "1\n",
    );
}

/// E3b: `Vec<string>.contains` accepts a literal and a by-value `string` local as well as a parameter.
#[test]
fn vec_string_contains_literal_and_local() {
    expect_output(
        r#"struct Room {
    members: Vec<string>,
}

impl Room {
    fn has_ann(self) -> bool {
        self.members.contains("ann")
    }

    fn has_local(self, who: string) -> bool {
        let w = who
        self.members.contains(w)
    }
}

fn main() {
    let mut members = Vec::new()
    members.push("ann")
    let room = Room { members: members }
    println("${room.has_ann()} ${room.has_local("bob")}")
}
"#,
        "true false\n",
    );
}

// ------------------------------------------------------------------ crate-level dependencies

fn derive_line_before<'a>(rs: &'a str, decl: &str) -> &'a str {
    let at = rs.find(decl).unwrap_or_else(|| panic!("`{decl}` not in:\n{rs}"));
    rs[..at].lines().rev().find(|l| l.contains("derive(")).unwrap_or("")
}

/// E8: `std::json` in a crate derives `Serialize` for structs made of serializable types only; structs holding a
/// type imported from another crate (or holding such a struct) are left alone.
#[test]
fn serde_derive_skips_structs_holding_foreign_types() {
    let rs = generated_rust(
        r#"use std::json
use std::collections::HashMap
use wj_rate_limit::WindowBucket

pub struct Reply {
    pub ok: bool,
}

pub struct Limiter {
    pub buckets: HashMap<string, WindowBucket>,
}

pub struct Wrapper {
    pub inner: Limiter,
}

pub fn health() -> string {
    json.to_string(Reply { ok: true }).unwrap_or("")
}

fn main() {
    println(health())
}
"#,
    );
    assert!(derive_line_before(&rs, "struct Reply").contains("Serialize"), "{rs}");
    assert!(!derive_line_before(&rs, "struct Limiter").contains("Serialize"), "{rs}");
    assert!(!derive_line_before(&rs, "struct Wrapper").contains("Serialize"), "{rs}");
}
