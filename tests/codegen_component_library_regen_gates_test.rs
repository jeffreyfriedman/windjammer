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

//! Gate — safe full regenerate of a multipass component library.
//!
//! Contract for `wj build --library --module-file` of a components tree:
//! emitted Rust must `cargo check`, and `wj` must exit 0 after codegen so
//! host `build.rs` wrappers do not panic.
//!
//! Language shapes covered (no product/repo names):
//! 1. Owned aggregate formals (`TableColumn`) must not demote to `&TableColumn`.
//! 2. Forwarding owned aggregates into callees must not insert `&col` / `&row`.
//! 3. Explicit `.clone()` in sources must not fail library build exit code.
//! 4. Owned string field reused after a helper (PeriodBadge-style) cargo-checks
//!    without requiring source `.clone()`.
//! 5. Struct field (`color: string`) reused after a formatting helper cargo-checks.
//! 6. `-> &'static str` runtime JS helpers survive library emit + rustc.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn wj_library(src_mod: &std::path::Path, out: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src_mod.to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run wj")
}

fn strip_super_glob(out: &std::path::Path, names: &[&str]) {
    for name in names {
        let path = out.join(name);
        if let Ok(body) = fs::read_to_string(&path) {
            let cleaned = body.replace("#[allow(unused_imports)]\nuse super::*;\n\n", "");
            let _ = fs::write(path, cleaned);
        }
    }
}

fn cargo_check_lib(out: &std::path::Path, pkg: &str, mods: &str) -> (bool, String) {
    fs::write(
        out.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{pkg}"
version = "0.0.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#
        ),
    )
    .unwrap();
    fs::write(out.join("lib.rs"), mods).unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(out)
        .output()
        .expect("cargo check");
    (
        check.status.success(),
        String::from_utf8_lossy(&check.stderr).to_string(),
    )
}

/// Multipass: `fn column(self, col: TableColumn)` must emit owned formal, and
/// call sites `dt.column(TableColumn::new(...))` must cargo-check.
#[test]
fn owned_aggregate_formal_must_not_demote_to_ref() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(
        src.join("traits.wj"),
        "pub trait Renderable { fn render(self) -> string }\n",
    )
    .unwrap();
    fs::write(
        src.join("table.wj"),
        r#"
use super::traits::Renderable

pub struct TableColumn { header: string }
impl TableColumn {
    pub fn new(header: string) -> TableColumn { TableColumn { header: header } }
}

pub struct Table { columns: Vec<TableColumn> }
impl Table {
    pub fn new() -> Table { Table { columns: vec![] } }
    pub fn column(self, col: TableColumn) -> Table {
        self.columns.push(col)
        self
    }
}
impl Renderable for Table {
    fn render(self) -> string { "t" }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("datatable.wj"),
        r#"
use super::traits::Renderable
use super::table::{Table, TableColumn}

pub struct DataTable { table: Table }
impl DataTable {
    pub fn new() -> DataTable { DataTable { table: Table::new() } }
    pub fn column(self, col: TableColumn) -> DataTable {
        self.table = self.table.column(col)
        self
    }
}
impl Renderable for DataTable {
    fn render(self) -> string { self.table.render() }
}

pub fn sample() -> string {
    DataTable::new().column(TableColumn::new("Name")).render()
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod traits\npub mod table\npub mod datatable\n").unwrap();

    let status = wj_library(&src.join("mod.wj"), &out);
    assert!(
        status.status.success(),
        "wj library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let dt = fs::read_to_string(out.join("datatable.rs")).expect("datatable.rs");
    assert!(
        !dt.contains("col: &TableColumn") && !dt.contains("col: & table::TableColumn"),
        "TableColumn formal must stay owned. Got:\n{dt}"
    );
    assert!(
        !dt.contains(".column(&col)"),
        "must not forward &col into owned Table::column. Got:\n{dt}"
    );

    strip_super_glob(&out, &["traits.rs", "table.rs", "datatable.rs"]);
    let (ok, stderr) = cargo_check_lib(
        &out,
        "owned_aggregate_formal_gate",
        "pub mod traits;\npub mod table;\npub mod datatable;\n",
    );
    assert!(
        ok,
        "owned TableColumn formal + call site must cargo check. stderr=\n{stderr}\nemitted=\n{dt}"
    );
}

/// Library build with explicit `.clone()` must exit 0 after codegen.
#[test]
fn library_build_with_explicit_clone_must_exit_zero_after_codegen() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();
    fs::write(src.join("mod.wj"), "pub mod clean\npub mod leaky\n").unwrap();
    fs::write(src.join("clean.wj"), "pub fn ok() -> string { \"ok\" }\n").unwrap();
    fs::write(
        src.join("leaky.wj"),
        "pub fn twice(s: string) -> string { s.clone() + s }\n",
    )
    .unwrap();

    let status = wj_library(&src.join("mod.wj"), &out);
    assert!(
        out.join("clean.rs").exists() && out.join("leaky.rs").exists(),
        "codegen should emit modules; stderr:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(
        status.status.success(),
        "library regen must exit 0 after codegen (prefer auto-remove .clone()). stderr:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
}

/// Owned string moved into helper then into builder again — no source `.clone()`.
#[test]
fn owned_string_reuse_after_helper_into_builder_must_cargo_check() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(
        src.join("traits.wj"),
        "pub trait Renderable { fn render(self) -> string }\n",
    )
    .unwrap();
    fs::write(
        src.join("badge.wj"),
        r#"
use super::traits::Renderable
pub struct Badge { text: string }
impl Badge {
    pub fn new(text: string) -> Badge { Badge { text: text } }
}
impl Renderable for Badge {
    fn render(self) -> string { self.text }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("periodbadge.wj"),
        r#"
use super::traits::Renderable
use super::badge::Badge

pub struct PeriodBadge { state: string, label: string }
impl PeriodBadge {
    pub fn new(state: string) -> PeriodBadge {
        PeriodBadge { state: state, label: "" }
    }
}
fn state_class(state: string) -> string {
    if state == "open" { "open" } else { "other" }
}
impl Renderable for PeriodBadge {
    fn render(self) -> string {
        let state = self.state
        let cls = state_class(state)
        let badge = Badge::new(state).render()
        cls + ":" + badge
    }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("mod.wj"),
        "pub mod traits\npub mod badge\npub mod periodbadge\n",
    )
    .unwrap();

    let status = wj_library(&src.join("mod.wj"), &out);
    assert!(
        status.status.success(),
        "wj library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
    strip_super_glob(&out, &["traits.rs", "badge.rs", "periodbadge.rs"]);
    let (ok, stderr) = cargo_check_lib(
        &out,
        "owned_reuse_builder_gate",
        "pub mod traits;\npub mod badge;\npub mod periodbadge;\n",
    );
    assert!(
        ok,
        "owned reuse into builder without source .clone() must cargo check. stderr=\n{stderr}"
    );
}

/// Field `color: string` used after a helper that consumed a copy — cargo-check.
#[test]
fn string_field_reuse_after_helper_must_cargo_check() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(
        src.join("rating.wj"),
        r#"
pub struct Rating { value: int, color: string }
impl Rating {
    pub fn new(value: int) -> Rating {
        Rating { value: value, color: "gold" }
    }
    pub fn color(self, color: string) -> Rating {
        self.color = color
        self
    }
}
fn display_text(color: string) -> string {
    "color:" + color
}
pub fn render_rating(r: Rating) -> string {
    let color = r.color
    let style = display_text(color)
    // Must reuse `color` after helper without source `.clone()` (tip E0382/E0507).
    format!("<span style=\"{}\" data-c=\"{}\">{}</span>", style, color, r.value)
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod rating\n").unwrap();

    let status = wj_library(&src.join("mod.wj"), &out);
    assert!(
        status.status.success(),
        "wj library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
    strip_super_glob(&out, &["rating.rs"]);
    let body = fs::read_to_string(out.join("rating.rs")).unwrap_or_default();
    let (ok, stderr) = cargo_check_lib(&out, "string_field_reuse_gate", "pub mod rating;\n");
    assert!(
        ok,
        "string field reuse in render must cargo check. stderr=\n{stderr}\nemitted=\n{body}"
    );
}

/// `pub fn foo_runtime_js() -> &'static str` must emit and rustc.
#[test]
fn static_str_runtime_js_helper_must_codegen() {
    let source = r#"
pub fn widget_runtime_js() -> &'static str {
    "(function(){window.__wjWidgetBound=true;})();"
}
fn main() {
    let _ = widget_runtime_js();
}
"#;
    let result = test_utils::compile_single(source);
    assert!(
        result.contains("pub fn widget_runtime_js")
            && (result.contains("-> &'static str") || result.contains("-> &str")),
        "runtime_js &'static str must codegen. Got:\n{result}"
    );
    assert!(
        !result.contains("error[E"),
        "runtime_js helper must rustc. Got:\n{result}"
    );
}

/// Multipass: read-only string formals consumed by owned builder methods + &str call sites.
#[test]
fn multipass_string_formal_into_owned_builder_must_cargo_check() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(
        src.join("tile.wj"),
        r#"
pub struct Tile { html: string }
impl Tile {
    pub fn new() -> Tile { Tile { html: "" } }
    pub fn value_html(self, html: string) -> Tile {
        self.html = html
        self
    }
    pub fn render(self) -> string { self.html }
}
pub fn grid(left: string, right: string) -> string {
    let a = Tile::new().value_html(left).render()
    let b = Tile::new().value_html(right).render()
    a + b
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod tile\n").unwrap();

    let status = wj_library(&src.join("mod.wj"), &out);
    assert!(status.status.success(), "wj library build must succeed");

    let generated = fs::read_to_string(out.join("tile.rs")).expect("tile.rs");
    assert!(
        !generated.contains("left: &String") && !generated.contains(": &String,"),
        "must not emit &String formals. Got:\n{generated}"
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
name = "mp_builder_formals"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "mp_builder_formals"
path = "src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!(
            "#![allow(dead_code, unused)]\n{body}\nfn main() {{ println!(\"{{}}\", grid(\"$1\", \"$2\")); }}\n"
        ),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo");
    assert!(
        check.status.success(),
        "multipass owned-builder formals must cargo check with &str literals. stderr=\n{}\ngenerated=\n{generated}",
        String::from_utf8_lossy(&check.stderr)
    );
}
