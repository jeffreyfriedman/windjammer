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

//! FAILING REPROS — enum match qualification, `chars()` loops, JSON field helpers.
//!
//! Each gate `cargo check`s emitted Rust:
//!
//! 1. **Enum match arms** — idiomatic WJ `match route { Home => …, Money if … => … }`
//!    must emit `Route::Home` / `Route::Money` (not bare bindings → rustc E0170).
//!
//! 2. **`chars()` loops** — `for c in s.chars() { if c == '{' … }` must not emit
//!    `if *c == '{'` (loop var is already `char` → E0614).
//!
//! 3. **JSON field helper** — owned `string` key formal + `find` / call sites with
//!    bare `"role"` must typecheck (`&str`/`String` consistently). Codegen owns
//!    literals when the formal is owned — fixtures must not use `.to_string()` (W0006).

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn wj_build_and_cargo_check(source: &str) -> (bool, String, String) {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(&src, source).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let status = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .status()
        .expect("run wj");
    if !status.success() {
        return (
            false,
            String::new(),
            format!("wj build failed with status {status}"),
        );
    }

    let rs = fs::read_to_string(out.join("t.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "enum_match_chars_json_gate"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "enum_match_chars_json_gate"
path = "src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!("#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n{rs}"),
    )
    .unwrap();

    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    (
        check.status.success(),
        rs,
        String::from_utf8_lossy(&check.stderr).to_string(),
    )
}

#[test]
fn enum_match_arms_must_qualify_variants() {
    let source = r#"
enum Route { Home, Money, Unknown }
enum Mode { Workspace, Business }

pub fn body_for(route: Route, mode: Mode) -> string {
    match route {
        Money if mode == Mode::Business => "<div data-mode-business></div>",
        Money => "<div class=\"hub-panel\"></div>",
        Home => "<div class=\"home-hero\"></div>",
        Unknown => "<p class=\"err\">Unknown</p>",
    }
}

pub fn title(route: Route) -> string {
    match route {
        Home => "Home",
        Money => "Money",
        Unknown => "Unknown",
    }
}

fn main() {
    println!("{}", body_for(Route::Money, Mode::Business))
    println!("{}", title(Route::Home))
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.is_empty(),
        "wj must emit Rust for enum match. stderr:\n{stderr}"
    );
    assert!(
        ok,
        "enum match must cargo check (qualify Route::Variant). stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

#[test]
fn chars_for_loop_must_not_star_deref_char() {
    let source = r#"
pub fn count_objects(array_json: string) -> int {
    let mut depth = 0
    let mut count = 0
    for c in array_json.chars() {
        if c == '{' {
            if depth == 0 {
                count = count + 1
            }
            depth = depth + 1
        }
        if c == '}' {
            depth = depth - 1
        }
    }
    count
}

fn main() {
    println!("{}", count_objects("[{},{\"a\":1}]"))
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.contains("*c ==") && !generated.contains("*c=="),
        "chars() loop var is char — must not emit *c. Got:\n{generated}"
    );
    assert!(
        ok,
        "chars() loop must cargo check. stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

#[test]
fn json_string_field_helper_must_cargo_check() {
    let source = r#"
pub fn json_string_field(obj: string, key: string) -> Option<string> {
    let needle = "\"" + key + "\""
    match obj.find(needle) {
        Some(idx) => {
            let after = obj.substring(idx, obj.len())
            Some(after)
        },
        None => None,
    }
}

pub fn role_hint(obj: string) -> string {
    match json_string_field(obj, "role") {
        Some(r) => r,
        None => {
            if obj.contains("\"customer\"") {
                "customer"
            } else {
                ""
            }
        },
    }
}

pub struct Row {
    pub name: string,
}

pub fn parse_rows(json: string) -> Vec<Row> {
    let slices = vec![json]
    let mut out: Vec<Row> = Vec::new()
    for obj in slices {
        match json_string_field(obj, "display_name") {
            Some(name) => {
                out.push(Row { name: name })
            },
            None => {},
        }
    }
    out
}

fn main() {
    println!("{}", role_hint("{\"role\":\"vendor\"}"))
    println!("{}", parse_rows("{\"display_name\":\"Acme\"}").len())
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        ok,
        "json_string_field + parse_rows must cargo check (owned keys / find Pattern / Option). stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}
