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

//! Proactive hardening gates — codegen contracts that `cargo check` emitted Rust.
//!
//! These are forward-looking regression guards, not failing repros of known bugs.
//! Primary assert: `wj build --no-cargo` + `cargo check` success.
//!
//! Covered shapes include typed closure formals, split/collect iteration, library enum
//! match qualification, deep receiver chains, escape literals, and `pub use` re-export
//! call sites where callee signatures must resolve through the re-export path (owned
//! aggregate params, no E0308/E0507 at the call site).

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

struct GateResult {
    ok: bool,
    generated: String,
    wj_stderr: String,
    cargo_stderr: String,
}

fn read_emitted_rs(out_dir: &Path, stem: &str) -> String {
    fs::read_to_string(out_dir.join(format!("{stem}.rs"))).unwrap_or_else(|_| {
        fs::read_dir(out_dir)
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
    })
}

fn cargo_check_rs(crate_name: &str, rs_body: &str, tmp_root: &Path) -> (bool, String) {
    let crate_dir = tmp_root.join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{crate_name}"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "{crate_name}"
path = "src/main.rs"
"#
        ),
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!("#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n{rs_body}"),
    )
    .unwrap();

    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    (
        check.status.success(),
        String::from_utf8_lossy(&check.stderr).to_string(),
    )
}

/// Single-file `wj build --no-cargo` then `cargo check` on emitted Rust.
fn wj_single_file_cargo_check(source: &str, crate_name: &str) -> GateResult {
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
        return GateResult {
            ok: false,
            generated: String::new(),
            wj_stderr: format!("wj build failed: {wj_stderr}"),
            cargo_stderr: String::new(),
        };
    }

    let generated = read_emitted_rs(&out, "t");
    let (ok, cargo_stderr) = cargo_check_rs(crate_name, &generated, tmp.path());
    GateResult {
        ok,
        generated,
        wj_stderr,
        cargo_stderr,
    }
}

/// Multipass `--library --module-file` build then `cargo check`.
///
/// `module_files`: `(filename.wj, contents)` written under `src/`.
/// `target_stem`: emitted `.rs` stem to read (e.g. `"routes"` → `routes.rs`).
/// `main_suffix`: appended after filtered module body for a bin `main` harness.
fn wj_library_cargo_check(
    module_files: &[(&str, &str)],
    mod_wj: &str,
    target_stem: &str,
    crate_name: &str,
    main_suffix: &str,
) -> GateResult {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    for (name, body) in module_files {
        fs::write(src.join(name), body).unwrap();
    }
    fs::write(src.join("mod.wj"), mod_wj).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    let wj_stderr = String::from_utf8_lossy(&build.stderr).to_string();
    if !build.status.success() {
        return GateResult {
            ok: false,
            generated: String::new(),
            wj_stderr: format!("wj library build failed: {wj_stderr}"),
            cargo_stderr: String::new(),
        };
    }

    let generated = read_emitted_rs(&out, target_stem);
    let body = generated
        .lines()
        .filter(|l| !l.contains("use super::"))
        .collect::<Vec<_>>()
        .join("\n");
    let rs = format!(
        "#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n{body}\n{main_suffix}"
    );
    let (ok, cargo_stderr) = cargo_check_rs(crate_name, &rs, tmp.path());
    GateResult {
        ok,
        generated,
        wj_stderr,
        cargo_stderr,
    }
}

fn cargo_check_lib(out_dir: &Path, crate_name: &str, lib_body: &str) -> (bool, String) {
    fs::write(
        out_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{crate_name}"
version = "0.0.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#
        ),
    )
    .unwrap();
    fs::write(out_dir.join("lib.rs"), lib_body).unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(out_dir)
        .output()
        .expect("cargo check");
    (
        check.status.success(),
        String::from_utf8_lossy(&check.stderr).to_string(),
    )
}

fn strip_super_glob(out_dir: &Path, names: &[&str]) {
    for name in names {
        let path = out_dir.join(name);
        if let Ok(body) = fs::read_to_string(&path) {
            let cleaned = body.replace("#[allow(unused_imports)]\nuse super::*;\n\n", "");
            let _ = fs::write(path, cleaned);
        }
    }
}

/// Multipass `--library --module-file` build of several modules, then `cargo check` as `[lib]`.
fn wj_multipass_library_cargo_check(
    module_files: &[(&str, &str)],
    mod_wj: &str,
    lib_rs: &str,
    crate_name: &str,
    rs_stems_to_strip: &[&str],
) -> GateResult {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    for (name, body) in module_files {
        fs::write(src.join(name), body).unwrap();
    }
    fs::write(src.join("mod.wj"), mod_wj).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    let wj_stderr = String::from_utf8_lossy(&build.stderr).to_string();
    if !build.status.success() {
        return GateResult {
            ok: false,
            generated: String::new(),
            wj_stderr: format!("wj library build failed: {wj_stderr}"),
            cargo_stderr: String::new(),
        };
    }

    strip_super_glob(&out, rs_stems_to_strip);
    let generated = rs_stems_to_strip
        .iter()
        .map(|stem| {
            fs::read_to_string(out.join(format!("{stem}.rs"))).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    let (ok, cargo_stderr) = cargo_check_lib(&out, crate_name, lib_rs);
    GateResult {
        ok,
        generated,
        wj_stderr,
        cargo_stderr,
    }
}

fn assert_gate(
    gate: &str,
    result: GateResult,
    extra_generated_checks: impl FnOnce(&str),
) {
    assert!(
        !result.generated.is_empty() || result.wj_stderr.is_empty(),
        "{gate}: wj must emit Rust. wj_stderr:\n{}",
        result.wj_stderr
    );
    extra_generated_checks(&result.generated);
    assert!(
        result.ok,
        "{gate}: emitted Rust must cargo check.\nwj_stderr=\n{}\ncargo_stderr=\n{}\ngenerated=\n{}",
        result.wj_stderr, result.cargo_stderr, result.generated
    );
}

/// Proactive: typed closure params beyond `|c: char|` (e.g. `|s: string|` in `find`).
#[test]
fn typed_closure_string_predicate_must_cargo_check() {
    let source = r#"
pub fn first_long(items: Vec<string>) -> Option<string> {
    items.into_iter().find(|s: string| s.len() > 3)
}

fn main() {
    let _ = first_long(vec!["ab", "abcd"]);
}
"#;

    let result = wj_single_file_cargo_check(source, "typed_closure_string_gate");
    assert_gate("typed_closure_string_predicate", result, |_| {});
}

/// Proactive: `split("\n").collect()` yields iterable `Vec<string>`, not `collect::<String>`.
#[test]
fn split_collect_lines_must_cargo_check() {
    let source = r#"
pub fn non_empty_lines(text: string) -> Vec<string> {
    let mut out: Vec<string> = Vec::new()
    for line in text.split("\n").collect() {
        if line.len() > 0 {
            out.push(line)
        }
    }
    out
}

fn main() {
    let _ = non_empty_lines("a\nb\n");
}
"#;

    let result = wj_single_file_cargo_check(source, "split_collect_lines_gate");
    assert_gate("split_collect_lines", result, |generated| {
        assert!(
            !generated.contains("collect::<String>()"),
            "split().collect() for line iteration must not be String. Got:\n{generated}"
        );
    });
}

/// Proactive: multipass library enum match must qualify variants (`Route::Home`).
#[test]
fn enum_match_under_library_must_qualify_variants() {
    let routes_wj = r#"
pub enum Route { Home, Money }

pub fn label(r: Route) -> string {
    match r {
        Home => "home",
        Money => "money",
    }
}
"#;

    let result = wj_library_cargo_check(
        &[("routes.wj", routes_wj)],
        "pub mod routes\n",
        "routes",
        "enum_match_library_gate",
        r#"
fn main() {
    println!("{}", label(Route::Home));
}
"#,
    );
    assert_gate("enum_match_under_library", result, |generated| {
        assert!(
            generated.contains("Route::Home") || generated.contains("routes::Route::Home"),
            "library enum match must qualify variants (Route::Home). Got:\n{generated}"
        );
    });
}

/// Proactive: deep field/method receiver chains (`o.mid.inner.matches(n)`).
#[test]
fn deep_receiver_chain_must_cargo_check() {
    let source = r#"
pub struct Inner { pub id: string }
impl Inner {
    pub fn matches(self, n: string) -> bool { self.id == n }
}
pub struct Mid { pub inner: Inner }
pub struct Outer { pub mid: Mid }
pub fn hit(o: Outer, n: string) -> bool { o.mid.inner.matches(n) }

fn main() {
    let o = Outer { mid: Mid { inner: Inner { id: "a" } } }
    let _ = hit(o, "a")
}
"#;

    let result = wj_single_file_cargo_check(source, "deep_receiver_chain_gate");
    assert_gate("deep_receiver_chain", result, |_| {});
}

/// Proactive: `obj.len()` as `int` formal passed to `substring` must typecheck.
#[test]
fn len_to_int_formal_substring_must_cargo_check() {
    let source = r#"
pub fn field_end(obj: string, start: int) -> int {
    let rest = obj.substring(start, obj.len())
    rest.len()
}

fn main() {
    let _ = field_end("{\"a\":1}", 1);
}
"#;

    let result = wj_single_file_cargo_check(source, "len_int_substring_gate");
    assert_gate("len_to_int_formal_substring", result, |_| {});
}

/// Proactive: multipass library must accept backslash/backtick escape literals.
#[test]
fn library_escape_literals_must_cargo_check() {
    let escape_wj = [
        "pub fn escape_template(s: string) -> string {\n",
        "    s.replace(\"\\\\\", \"\\\\\\\\\")\n",
        "        .replace(\"`\", \"\\\\`\")\n",
        "        .replace(\"${\", \"\\\\${\")\n",
        "}\n",
        "\n",
        "pub fn map_entry(key: string, html: string) -> string {\n",
        "    format!(\"  '{}': `{}`,\\n\", key, escape_template(html))\n",
        "}\n",
    ]
    .concat();

    let result = wj_library_cargo_check(
        &[("escape.wj", &escape_wj)],
        "pub mod escape\n",
        "escape",
        "library_escape_literals_gate",
        r#"
fn main() {
    println!("{}", map_entry("home", "<div>x</div>"));
}
"#,
    );
    assert_gate("library_escape_literals", result, |_| {});
}

/// Proactive: string↔enum roundtrip via match arms must cargo-check.
#[test]
fn json_enum_string_roundtrip_must_cargo_check() {
    let source = r#"
enum Role { Customer, Vendor }

pub fn role_from_json(s: string) -> Role {
    match s {
        "customer" => Role::Customer,
        "vendor" => Role::Vendor,
        _ => Role::Customer,
    }
}

pub fn role_to_json(r: Role) -> string {
    match r {
        Role::Customer => "customer",
        Role::Vendor => "vendor",
    }
}

fn main() {
    let r = role_from_json("vendor")
    let _ = role_to_json(r)
}
"#;

    let result = wj_single_file_cargo_check(source, "json_enum_roundtrip_gate");
    assert_gate("json_enum_string_roundtrip", result, |_| {});
}

/// Proactive: `filter(|x: int| …).collect()` on `Vec<int>` must cargo-check without `*x`.
#[test]
fn filter_int_collect_must_cargo_check() {
    let source = r#"
pub fn positives(nums: Vec<int>) -> Vec<int> {
    nums.into_iter().filter(|x: int| x > 0).collect()
}

fn main() {
    let _ = positives(vec![1, -1, 2]);
}
"#;

    let result = wj_single_file_cargo_check(source, "filter_int_collect_gate");
    assert_gate("filter_int_collect", result, |_| {});
}

/// Proactive: `pub use a::keys_equal` — callee signature must resolve through re-export
/// so owned `Key` params at the call site cargo-check (no E0308/E0507).
#[test]
fn pub_use_signature_call_site_must_cargo_check() {
    let a_wj = r#"
pub struct Key { pub tag: string }
pub fn keys_equal(a: Key, b: Key) -> bool { a.tag == b.tag }
"#;

    let b_wj = r#"
use super::Key
use super::keys_equal
pub fn check(k: Key) -> bool {
    keys_equal(k, Key { tag: "x" })
}
"#;

    let mod_wj = "pub mod a\npub mod b\npub use a::Key\npub use a::keys_equal\n";

    let lib_rs = r#"#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
pub mod a;
pub mod b;
pub use a::Key;
pub use a::keys_equal;
"#;

    let result = wj_multipass_library_cargo_check(
        &[("a.wj", a_wj), ("b.wj", b_wj)],
        mod_wj,
        lib_rs,
        "pub_use_signature_call_site_gate",
        &["a", "b"],
    );
    assert_gate("pub_use_signature_call_site", result, |generated| {
        assert!(
            generated.contains("keys_equal("),
            "b module must call keys_equal. Got:\n{generated}"
        );
    });
}
