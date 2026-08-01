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

//! FAILING REPROS — LedgerKit `packages/finance-ui` Rust → pure Windjammer.
//!
//! Goal: eliminate hand-written Rust under
//! `financial-management-platform/packages/finance-ui/` (routes, read_models,
//! pages, books_mode, …). Host glue (`apps/web` wasm_bindgen, `apps/desktop`
//! wry) stays Rust; everything else should be `.wj`.
//!
//! Empirically red on tip `wj` (2026-08-01) — each gate `cargo check`s emitted Rust:
//!
//! 1. **Enum match arms** (`routes.rs` / `pages.rs` `body_for_prefs`):
//!    idiomatic WJ `match route { Home => …, Money if mode == … => … }` emits
//!    bare `Home` / `Money` bindings → rustc **E0170** (`bindings_with_variant_name`).
//!    Must emit `Route::Home` / `Route::Money` (or equivalent).
//!
//! 2. **`chars()` loops** (`read_models.rs` `json_object_slices`):
//!    `for c in s.chars() { if c == '{' … }` emits `if *c == '{'` → **E0614**.
//!    Loop var is already `char`; must not star-deref.
//!
//! 3. **JSON field helper** (`read_models.rs` `json_string_field` / parsers):
//!    owned `string` key formal demotes to `&String`, call sites pass
//!    `"role".to_string()` → **E0308**; `find(owned)` fails **Pattern**.
//!    Formals should be `&str`/`String` consistently; `find` needs `&str`/`&`.
//!
//! Related (already filed): `codegen_finance_owned_string_trait_call_gate_test`
//! (backend login/close_period owned-string borrow).
//!
//! Out of scope (keep Rust hosts): `wasm_bindgen` / `Closure` / `wry` / `tao`.

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
        // CLI may name output after stem differently across tip versions.
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
name = "ledgerkit_finance_ui_gate"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "ledgerkit_finance_ui_gate"
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

/// Mirror `packages/finance-ui/src/pages.rs` `body_for_prefs` + `routes.rs` enums.
#[test]
fn ledgerkit_enum_match_arms_must_qualify_variants() {
    let source = r#"
enum Route { Home, Money, Unknown }
enum BooksMode { Workspace, Business }

pub fn body_for(route: Route, mode: BooksMode) -> string {
    match route {
        Money if mode == BooksMode::Business => "<div data-wj-business-workspace></div>".to_string(),
        Money => "<div class=\"hub-panel\"></div>".to_string(),
        Home => "<div class=\"home-hero\"></div>".to_string(),
        Unknown => "<p class=\"err\">Unknown</p>".to_string(),
    }
}

pub fn title(route: Route) -> string {
    match route {
        Home => "Home".to_string(),
        Money => "Money".to_string(),
        Unknown => "Unknown".to_string(),
    }
}

fn main() {
    println!("{}", body_for(Route::Money, BooksMode::Business))
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
        "LedgerKit Route/BooksMode match must cargo check (qualify Route::Variant). stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// Mirror `packages/finance-ui/src/read_models.rs` `json_object_slices` brace walk.
#[test]
fn ledgerkit_chars_for_loop_must_not_star_deref_char() {
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
    println!("{}", count_objects("[{},{\"a\":1}]".to_string()))
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        !generated.contains("*c ==") && !generated.contains("*c=="),
        "chars() loop var is char — must not emit *c. Got:\n{generated}"
    );
    assert!(
        ok,
        "json_object_slices-style chars() loop must cargo check. stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}

/// Mirror `packages/finance-ui/src/read_models.rs` `json_string_field` + party parse.
#[test]
fn ledgerkit_json_string_field_helper_must_cargo_check() {
    let source = r#"
pub fn json_string_field(obj: string, key: string) -> Option<string> {
    let needle = "\"".to_string() + key + "\""
    match obj.find(needle) {
        Some(idx) => {
            let after = obj.substring(idx, obj.len())
            Some(after.to_string())
        },
        None => None,
    }
}

pub fn role_hint(obj: string) -> string {
    match json_string_field(obj, "role".to_string()) {
        Some(r) => r,
        None => {
            if obj.contains("\"customer\"") {
                "customer".to_string()
            } else {
                "".to_string()
            }
        },
    }
}

pub struct PartyRow {
    pub name: string,
}

pub fn parse_parties(json: string) -> Vec<PartyRow> {
    let slices = vec![json]
    let mut out: Vec<PartyRow> = Vec::new()
    for obj in slices {
        match json_string_field(obj, "display_name".to_string()) {
            Some(name) => {
                out.push(PartyRow { name: name })
            },
            None => {},
        }
    }
    out
}

fn main() {
    println!("{}", role_hint("{\"role\":\"vendor\"}".to_string()))
    println!("{}", parse_parties("{\"display_name\":\"Acme\"}".to_string()).len())
}
"#;

    let (ok, generated, stderr) = wj_build_and_cargo_check(source);
    assert!(
        ok,
        "json_string_field + parse_parties must cargo check (owned keys / find Pattern / Option). stderr=\n{stderr}\ngenerated=\n{generated}"
    );
}
