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

//! Full windjammer-ui regenerate gates (pure Windjammer conversion).
//!
//! Tip `wj build src/components_wj --library --module-file` still cannot replace
//! `src/components/generated/` safely. These tests are the contract for the
//! compiler session — keep them **red** until full-tree regen + `cargo check`
//! of windjammer-ui succeeds without `SKIP_WJ_REGEN=1` / hand patches.
//!
//! Observed on tip (2026-08-01):
//! 1. Library multipass inserts `&col` when forwarding owned `TableColumn` /
//!    `TableRow` into `Table::{column,row}` → rustc E0308 (DataTable dogfood).
//! 2. Explicit `.clone()` / `.iter()` / ownership annotations (W0005/W0003/W0001)
//!    make `wj build --library` exit non-zero **after** codegen, so windjammer-ui
//!    `build.rs` panics even though `.rs` files were written.
//!
//! Related (already green regressions): `codegen_windjammer_ui_regen_unblock_test`,
//! `codegen_windjammer_ui_fix_generated_debt_test`,
//! `codegen_authfetch_mount_param_ambiguous_glob_test`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn write_table_datatable_project(src: &std::path::Path) {
    fs::create_dir_all(src).unwrap();
    fs::write(
        src.join("traits.wj"),
        r#"
pub trait Renderable {
    fn render(self) -> string
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("table.wj"),
        r#"
use super::traits::Renderable

pub struct TableColumn {
    header: string,
}

impl TableColumn {
    pub fn new(header: string) -> TableColumn {
        TableColumn { header: header }
    }
}

pub struct TableRow {
    cells: Vec<string>,
}

impl TableRow {
    pub fn new() -> TableRow {
        TableRow { cells: vec![] }
    }

    pub fn cell(self, value: string) -> TableRow {
        self.cells.push(value)
        self
    }
}

pub struct Table {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
}

impl Table {
    pub fn new() -> Table {
        Table { columns: vec![], rows: vec![] }
    }

    pub fn column(self, col: TableColumn) -> Table {
        self.columns.push(col)
        self
    }

    pub fn row(self, row: TableRow) -> Table {
        self.rows.push(row)
        self
    }
}

impl Renderable for Table {
    fn render(self) -> string {
        "table"
    }
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("datatable.wj"),
        r#"
use super::traits::Renderable
use super::table::{Table, TableColumn, TableRow}

pub struct DataTable {
    table: Table,
}

impl DataTable {
    pub fn new() -> DataTable {
        DataTable { table: Table::new() }
    }

    pub fn column(self, col: TableColumn) -> DataTable {
        self.table = self.table.column(col)
        self
    }

    pub fn row(self, row: TableRow) -> DataTable {
        self.table = self.table.row(row)
        self
    }
}

impl Renderable for DataTable {
    fn render(self) -> string {
        self.table.render()
    }
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("mod.wj"),
        "pub mod traits\npub mod table\npub mod datatable\n",
    )
    .unwrap();
}

/// Gate A — DataTable must forward owned aggregates without `&` demotion.
#[test]
fn datatable_owned_column_row_forward_must_cargo_check() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    write_table_datatable_project(&src);
    fs::create_dir_all(&out).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("wj");
    assert!(
        status.status.success(),
        "wj library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let dt = fs::read_to_string(out.join("datatable.rs")).expect("datatable.rs");
    assert!(
        !dt.contains(".column(&col)") && !dt.contains(".row(&row)"),
        "owned TableColumn/TableRow must not be forwarded as `&col`/`&row` into \
         Table methods that take owned values. Got:\n{dt}"
    );

    // Probe cargo-check like windjammer-ui selective regen.
    fs::write(
        out.join("Cargo.toml"),
        r#"[package]
name = "wjui_datatable_regen_probe"
version = "0.1.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#,
    )
    .unwrap();
    fs::write(
        out.join("lib.rs"),
        "pub mod traits;\npub mod table;\npub mod datatable;\n",
    )
    .unwrap();
    // Strip ambiguous globs if present (separate gate already covered).
    for name in ["traits.rs", "table.rs", "datatable.rs"] {
        let path = out.join(name);
        if let Ok(body) = fs::read_to_string(&path) {
            let cleaned = body.replace("#[allow(unused_imports)]\nuse super::*;\n\n", "");
            let _ = fs::write(path, cleaned);
        }
    }

    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&out)
        .output()
        .expect("cargo");
    assert!(
        check.status.success(),
        "DataTable→Table owned forward must cargo-check.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

/// Gate B — explicit `.clone()` must not fail the library build exit code after codegen.
///
/// windjammer-ui `build.rs` panics on non-zero `wj` status. Today tip exits non-zero
/// with "Rust leakage errors" even though `.rs` files were written — blocking regen.
///
/// Desired: either auto-eliminate `.clone()` (preferred) **or** exit 0 with warnings
/// once codegen completed for `--library` builds used by windjammer-ui.
#[test]
fn library_build_with_explicit_clone_must_exit_zero_after_codegen() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod clean\npub mod leaky\n").unwrap();
    fs::write(
        src.join("clean.wj"),
        r#"
pub fn ok() -> string { "ok" }
"#,
    )
    .unwrap();
    fs::write(
        src.join("leaky.wj"),
        r#"
pub fn twice(s: string) -> string {
    s.clone() + s
}
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("wj");

    assert!(
        out.join("clean.rs").exists() && out.join("leaky.rs").exists(),
        "codegen should still emit modules; stderr:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(
        status.status.success(),
        "library regen must exit 0 after codegen so windjammer-ui build.rs does not panic.\n\
         Prefer auto-removing `.clone()` (W0005). stderr:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let leaky = fs::read_to_string(out.join("leaky.rs")).unwrap_or_default();
    // Prefer eliminating source `.clone()` in emitted Rust once exit-0 is fixed.
    let _ = leaky;
}

/// Gate C — PeriodBadge-style reuse: owned string into helper then Badge::new again.
/// Mirrors remaining `.clone()` in windjammer-ui `periodbadge.wj` / `approvalcard.wj`.
#[test]
fn period_badge_style_owned_reuse_library_must_cargo_check_without_source_clone() {
    let tmp = TempDir::new().expect("tempdir");
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
    // Intentionally NO .clone() — compiler must insert clones / demote helpers.
    fs::write(
        src.join("periodbadge.wj"),
        r#"
use super::traits::Renderable
use super::badge::Badge

pub struct PeriodBadge {
    state: string,
    label: string,
}

impl PeriodBadge {
    pub fn new(state: string) -> PeriodBadge {
        PeriodBadge { state: state, label: "" }
    }
    pub fn label(self, label: string) -> PeriodBadge {
        self.label = label
        self
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

    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("wj");
    assert!(
        status.status.success(),
        "periodbadge library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );

    fs::write(
        out.join("Cargo.toml"),
        r#"[package]
name = "wjui_period_regen_probe"
version = "0.1.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#,
    )
    .unwrap();
    fs::write(
        out.join("lib.rs"),
        "pub mod traits;\npub mod badge;\npub mod periodbadge;\n",
    )
    .unwrap();
    for name in ["traits.rs", "badge.rs", "periodbadge.rs"] {
        let path = out.join(name);
        if let Ok(body) = fs::read_to_string(&path) {
            let cleaned = body.replace("#[allow(unused_imports)]\nuse super::*;\n\n", "");
            let _ = fs::write(path, cleaned);
        }
    }

    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&out)
        .output()
        .expect("cargo");
    assert!(
        check.status.success(),
        "PeriodBadge owned reuse without source `.clone()` must cargo-check.\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
