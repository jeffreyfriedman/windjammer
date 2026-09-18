#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! WDB-246: `--module-file` format! temps into demoted `&str` method formals must borrow.
//!
//! Tip-out residual: `left.hash_join_semi_i64(right, _temp0, _temp1)` while formals are
//! `&str`. Signature-driven: hoist emits `&_temp0`, `&_temp1`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wdb246_module_file_format_temp_into_demoted_method_str_must_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let types_src = tmp.path().join("types_src");
    fs::create_dir_all(&types_src).unwrap();
    fs::write(
        types_src.join("batch.wj"),
        r#"
pub struct Batch { pub n: int }
impl Batch {
    pub fn hash_join_semi_i64(self, right: Batch, left_col: string, right_col: string) -> bool {
        left_col.len() + right_col.len() + right.n > 0
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

    let batch_rs = fs::read_to_string(types_gen.join("batch.rs")).unwrap_or_default();
    assert!(
        batch_rs.contains("left_col: &str") && batch_rs.contains("right_col: &str"),
        "expected demoted &str formals in types:\n{batch_rs}"
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
pub fn run(left: Batch, right: Batch, vname: string, rv: string) -> bool {
    left.hash_join_semi_i64(right, vname + "", rv + "")
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
    eprintln!("WDB-246 module-file emit:\n{generated}");
    let bad = generated.contains("hash_join_semi_i64(right, _temp0, _temp1)")
        && !generated.contains("hash_join_semi_i64(right, &_temp0, &_temp1)");
    assert!(
        !bad,
        "WDB-246 RED: format temps into demoted &str must borrow:\n{generated}"
    );
    assert!(
        generated.contains("&_temp0") && generated.contains("&_temp1"),
        "expected &_tempN borrow into demoted formals:\n{generated}"
    );
}
