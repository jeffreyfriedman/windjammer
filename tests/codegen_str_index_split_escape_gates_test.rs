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

//! Gates — string indexing, split/collect, escape literals, multipass formals.
//!
//! Contract per gate: `wj build` emits Rust that `cargo check`s (or, for
//! lexer/parser gates, `wj` itself must accept the source).

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
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    let wj_stderr = String::from_utf8_lossy(&build.stderr).to_string();
    if !build.status.success() {
        return (
            false,
            String::new(),
            format!("wj build failed: {wj_stderr}"),
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
name = "str_index_split_escape_gate"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "str_index_split_escape_gate"
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

/// `char_indices` + substring/slice by index must use `usize` (not `i64`).
#[test]
fn char_indices_slice_must_use_usize() {
    let source = r#"
pub fn object_slices(array_json: string) -> Vec<string> {
    let mut out: Vec<string> = Vec::new()
    let mut depth = 0
    let mut start: Option<int> = None
    for i_c in array_json.char_indices() {
        let i = i_c.0
        let c = i_c.1
        if c == '{' {
            if depth == 0 {
                start = Some(i)
            }
            depth = depth + 1
        }
        if c == '}' {
            depth = depth - 1
            if depth == 0 {
                match start {
                    Some(s) => {
                        out.push(array_json.substring(s, i + 1))
                        start = None
                    },
                    None => {},
                }
            }
        }
    }
    out
}

fn main() {
    println!("{}", object_slices("[{},{\"a\":1}]").len())
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.is_empty(),
        "wj must emit Rust for char_indices walk. stderr:\n{stderr}"
    );
    assert!(
        ok,
        "char_indices + substring must cargo check (usize indices). stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// `str::find` with a typed char predicate must parse and cargo-check.
#[test]
fn find_char_predicate_must_parse_and_check() {
    let source = r#"
pub fn value_end(rest: string) -> int {
    match rest.find(|c: char| c == ',' || c == '}' || c == ']') {
        Some(end) => end,
        None => rest.len(),
    }
}

pub fn slice_num(rest: string) -> string {
    let end = value_end(rest)
    rest.substring(0, end).trim().to_string()
}

fn main() {
    println!("{}", slice_num("42,true"))
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.is_empty() || ok,
        "find(|c: char| …) must parse. stderr:\n{stderr}"
    );
    assert!(
        ok,
        "find(char predicate) + substring must cargo check. stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// Char-count `int` end + `substring` must cargo-check (`usize` / owned `String`).
#[test]
fn int_index_substring_must_cargo_check() {
    let source = r#"
pub fn value_end(rest: string) -> int {
    let mut i = 0
    for c in rest.chars() {
        if c == ',' || c == '}' || c == ']' {
            return i
        }
        i = i + 1
    }
    rest.len()
}

pub fn slice_num(rest: string) -> string {
    let end = value_end(rest)
    rest.substring(0, end).trim().to_string()
}

fn main() {
    println!("{}", slice_num("42,true"))
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.is_empty(),
        "wj must emit for int-index substring. stderr:\n{stderr}"
    );
    assert!(
        ok,
        "char-count end + substring must cargo check. stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// `find` + `substring` match arms must return owned `String` (not `&str`).
#[test]
fn substring_match_arm_must_return_owned_string() {
    let source = r#"
pub fn after_key(json: string, key: string) -> string {
    let needle = "\"" + key + "\""
    match json.find(needle) {
        Some(idx) => json.substring(idx, json.len()),
        None => "",
    }
}

fn main() {
    println!("{}", after_key("{\"role\":\"x\"}", "role"))
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.is_empty(),
        "wj must emit for find+substring. stderr:\n{stderr}"
    );
    assert!(
        ok,
        "substring match arms must be String (not &str). stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// `split("|").collect()` for field indexing must emit `Vec`, not `String` / bare `Split`.
#[test]
fn pipe_split_collect_vec_index_must_cargo_check() {
    let source = r##"
pub fn resolve_key(chord: string) -> Option<string> {
    let normalized = chord.trim().to_ascii_lowercase()
    if normalized == "/" {
        return None
    }
    let rows = vec![
        "a|Alpha|#/a",
        "b|Beta|#/b",
        "c|Gamma|#/c",
    ]
    for row in rows {
        let parts = row.split("|").collect()
        if parts.len() >= 3 {
            if parts[0] == normalized {
                return Some(parts[2].to_string())
            }
        }
    }
    None
}

pub fn crumb_html(items: Vec<string>) -> string {
    let mut out = "<nav>"
    let mut i = 0
    for item in items {
        let parts = item.split("|").collect()
        if parts.len() >= 2 {
            if i > 0 {
                out = out + " / "
            }
            out = out + "<a href=\"" + parts[0] + "\">" + parts[1] + "</a>"
        }
        i = i + 1
    }
    out + "</nav>"
}

fn main() {
    match resolve_key("b") {
        Some(h) => println!("{}", h),
        None => println!("none"),
    }
    let items = vec!["#/a|Alpha", "#/b|Beta"]
    println!("{}", crumb_html(items))
}
"##;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.is_empty(),
        "wj must emit for pipe split. stderr:\n{stderr}"
    );
    assert!(
        !generated.contains("collect::<String>()")
            || generated.contains("collect::<Vec")
            || generated.contains("collect::<Vec<"),
        "split().collect() for field access must be Vec, not String. Got:\n{generated}"
    );
    assert!(
        ok,
        "split+collect+index must cargo check. stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// Backslash and backtick inside string literals must lex/compile (JS-escape style).
#[test]
fn backslash_and_backtick_string_literals_must_compile() {
    let source = [
        "pub fn escape_template(s: string) -> string {\n",
        "    s.replace(\"\\\\\", \"\\\\\\\\\")\n",
        "        .replace(\"`\", \"\\\\`\")\n",
        "        .replace(\"${\", \"\\\\${\")\n",
        "}\n",
        "\n",
        "pub fn map_entry(key: string, html: string) -> string {\n",
        "    format!(\"  '{}': `{}`,\\n\", key, escape_template(html))\n",
        "}\n",
        "\n",
        "fn main() {\n",
        "    println!(\"{}\", map_entry(\"home\", \"<div>x</div>\"))\n",
        "}\n",
    ]
    .concat();

    let (ok, generated, stderr) = wj_build_and_cargo_check(&source);
    assert!(
        !generated.is_empty(),
        "wj must accept backslash/backtick string literals. stderr:\n{stderr}"
    );
    assert!(
        ok,
        "escape_template must cargo check. stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// Multipass library: read-only `string` formals consumed by owned builder methods
/// must not emit `&String`, and `&str` call sites must typecheck.
#[test]
fn multipass_owned_builder_string_formals_must_not_be_ref_string() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();
    fs::write(
        src.join("tiles.wj"),
        r#"
pub struct Tile {
    pub value_html: string,
}

impl Tile {
    pub fn new() -> Tile {
        Tile { value_html: "" }
    }
    pub fn value_html(self, html: string) -> Tile {
        self.value_html = html
        self
    }
    pub fn render(self) -> string {
        self.value_html
    }
}

pub fn render_grid(left_html: string, right_html: string, ok: bool) -> string {
    let a = Tile::new().value_html(left_html).render()
    let b = Tile::new().value_html(right_html).render()
    if ok {
        a + b
    } else {
        a
    }
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod tiles\n").unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let status = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .status()
        .expect("run wj");
    assert!(status.success(), "wj library build must succeed");

    let generated = fs::read_to_string(out.join("tiles.rs")).expect("tiles.rs");
    assert!(
        !generated.contains("left_html: &String")
            && !generated.contains(": &String,")
            && !generated.contains(": &String)"),
        "multipass+owned builder must not emit &String formals. Got:\n{generated}"
    );

    let body = generated
        .lines()
        .filter(|l| !l.contains("use super::"))
        .collect::<Vec<_>>()
        .join("\n");
    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "mp_tile_formals"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "mp_tile_formals"
path = "src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!(
            "#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n{body}\n\nfn main() {{\n    println!(\"{{}}\", render_grid(\"$1\", \"$2\", true));\n}}\n"
        ),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "multipass owned-builder formals must cargo check with &str call sites. stderr=\n{}\ngenerated=\n{generated}",
        String::from_utf8_lossy(&check.stderr)
    );
}
