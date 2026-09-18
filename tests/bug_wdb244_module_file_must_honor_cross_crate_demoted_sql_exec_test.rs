#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! WDB-244: `--module-file` multipass must honor cross-crate demoted `&str` formals.
//!
//! Single-file tip emit keeps bare `"props"`; the same source via `mod.wj --module-file`
//! still emits `"props".to_string()` into demoted `Batch::sql_exec`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wdb244_module_file_cross_crate_demoted_sql_exec_must_not_to_string() {
    let tmp = TempDir::new().expect("tempdir");
    let types_src = tmp.path().join("types_src");
    fs::create_dir_all(&types_src).unwrap();
    fs::write(
        types_src.join("batch.wj"),
        r#"
pub struct Batch { pub n: int }
impl Batch {
    pub fn sql_exec(self, right: Batch, left_table: string, right_table: string, sql: string) -> bool {
        left_table.len() + right_table.len() + sql.len() + right.n > 0
    }
}
"#,
    )
    .unwrap();
    fs::write(types_src.join("mod.wj"), "pub mod batch\npub use batch::Batch\n").unwrap();

    let types_gen = tmp.path().join("types_gen");
    let wj = env!("CARGO_BIN_EXE_wj");
    let types_build = Command::new(wj)
        .args([
            "build",
            types_src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "--no-cargo",
            "--output",
            types_gen.to_str().unwrap(),
        ])
        .output()
        .expect("types build");
    assert!(
        types_build.status.success(),
        "types build failed:\n{}",
        String::from_utf8_lossy(&types_build.stderr)
    );

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("graph")).unwrap();
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.types_pkg]\npath = \"{}\"\npackage = \"types_src\"\n",
            types_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("graph").join("datafusion.wj"),
        r#"
use types_pkg::Batch
pub fn run(left: Batch, right: Batch, sql: string) -> bool {
    left.sql_exec(right, "props", "edges", sql)
}
"#,
    )
    .unwrap();
    fs::write(app_src.join("graph").join("mod.wj"), "pub mod datafusion\n").unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(wj)
        .args([
            "build",
            app_src.join("graph").join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--no-cargo",
            "--output",
            app_gen.to_str().unwrap(),
        ])
        .output()
        .expect("app module-file build");
    assert!(
        app_build.status.success(),
        "app module-file build failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("datafusion.rs")).unwrap_or_default();
    eprintln!("WDB-244 module-file emit:\n{generated}");
    assert!(
        !generated.contains("\"props\".to_string()")
            && !generated.contains("\"edges\".to_string()"),
        "WDB-244 RED: module-file multipass owned string lits into demoted &str sql_exec:\n{generated}"
    );

    // Control: single-file of the same source must also stay bare (already GREEN).
    let single_gen = tmp.path().join("single_gen");
    let single_build = Command::new(wj)
        .args([
            "build",
            app_src.join("graph").join("datafusion.wj").to_str().unwrap(),
            "--no-cargo",
            "--output",
            single_gen.to_str().unwrap(),
        ])
        .output()
        .expect("single build");
    assert!(single_build.status.success());
    let single = fs::read_to_string(single_gen.join("datafusion.rs"))
        .or_else(|_| fs::read_to_string(single_gen.join("graph").join("datafusion.rs")))
        .unwrap_or_default();
    assert!(
        !single.contains("\"props\".to_string()"),
        "control single-file unexpectedly owned lit:\n{single}"
    );
}
