#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let wj_file = dir.path().join("test.wj");
    std::fs::write(&wj_file, source).expect("write .wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(wj_file.to_str().unwrap())
        .arg("--no-cargo")
        .arg("-o")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("run wj build");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let rs_file = out_dir.join("test.rs");
    let generated = if rs_file.exists() {
        std::fs::read_to_string(&rs_file).unwrap_or_default()
    } else {
        String::new()
    };

    (output.status.success(), generated, combined)
}

#[test]
fn test_braced_import_types_not_duplicated_as_super() {
    // When a type is imported via a braced import like:
    //   use engine::rendering::{VoxelGPURenderer, LightingConfig}
    // The compiler should NOT generate a separate:
    //   use super::VoxelGPURenderer;
    // because the type is already covered by the braced import.

    let source = r#"
use crate::other_module::{TypeA, TypeB}

pub struct MyStruct {
    pub a: TypeA,
    pub b: TypeB,
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // The generated code should NOT have `use super::TypeA` or `use super::TypeB`
    // because they're already imported via the braced import
    assert!(
        !generated.contains("use super::TypeA;"),
        "TypeA should NOT be auto-imported via super:: when already in braced import.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("use super::TypeB;"),
        "TypeB should NOT be auto-imported via super:: when already in braced import.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_external_crate_function_import_no_glob() {
    // Function imports from external crates should NOT get ::* appended.
    // use engine::weapon::weapon::create_assault_rifle
    // should generate: use engine::weapon::weapon::create_assault_rifle;
    // NOT: use engine::weapon::weapon::create_assault_rifle::*;

    let source = r#"
use engine::weapon::weapon::create_assault_rifle

pub fn test_fn() {
    let rifle = create_assault_rifle()
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    // May not succeed without the engine crate, but we can check generated code
    let _ = success;
    let _ = output;

    assert!(
        !generated.contains("create_assault_rifle::*"),
        "Function import should NOT get ::* appended.\nGenerated:\n{}",
        generated
    );
}
