#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

/// TDD: Scoped transpile to `gen/subdir` must not pollute output with sibling `.rs`
/// files from the parent `gen/` output directory.
///
/// Bug: `generate_mod_file` called `copy_sibling_rs_from_parent`, which copied every
/// `gen/*.rs` into `gen/ffi/` when rebuilding a single module — then `mod.rs` listed
/// the entire crate as submodules of `ffi`.
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn write_minimal_crate(project: &std::path::Path) {
    std::fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"pollution_test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(project.join("src/ffi")).unwrap();
    std::fs::write(
        project.join("src/ffi/api.wj"),
        r#"
pub fn api_fn() -> i32 {
    42
}
"#,
    )
    .unwrap();
}

fn seed_gen_root_decoys(project: &std::path::Path) {
    std::fs::create_dir_all(project.join("gen")).unwrap();
    std::fs::write(
        project.join("gen/decoy_module.rs"),
        "pub fn decoy() -> i32 { 0 }\n",
    )
    .unwrap();
    std::fs::write(
        project.join("gen/another_decoy.rs"),
        "pub fn another() -> i32 { 1 }\n",
    )
    .unwrap();
    std::fs::create_dir_all(project.join("gen/ffi")).unwrap();
}

#[test]
fn test_single_file_subdir_output_does_not_copy_gen_siblings() {
    let temp = TempDir::new().expect("tempdir");
    let project = temp.path();
    write_minimal_crate(project);
    seed_gen_root_decoys(project);

    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let status = Command::new(&wj)
        .args([
            "build",
            project.join("src/ffi/api.wj").to_str().unwrap(),
            "--output",
            project.join("gen/ffi").to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .status()
        .expect("run wj");

    assert!(status.success(), "single-file subdir build should succeed");

    assert!(
        !project.join("gen/ffi/decoy_module.rs").exists(),
        "decoy_module.rs must not be copied from gen/ into gen/ffi/"
    );
    assert!(
        !project.join("gen/ffi/another_decoy.rs").exists(),
        "another_decoy.rs must not be copied from gen/ into gen/ffi/"
    );

    let mod_rs_path = project.join("gen/ffi/mod.rs");
    assert!(mod_rs_path.exists(), "gen/ffi/mod.rs should exist");
    let mod_rs = std::fs::read_to_string(&mod_rs_path).unwrap();
    assert!(
        !mod_rs.contains("decoy_module"),
        "mod.rs must not declare decoy modules from gen/ root:\n{mod_rs}"
    );
    assert!(
        !mod_rs.contains("another_decoy"),
        "mod.rs must not declare another_decoy from gen/ root:\n{mod_rs}"
    );

    assert!(
        project.join("gen/ffi/api.rs").exists(),
        "api.rs should be emitted under gen/ffi/"
    );
}

#[test]
fn test_directory_subdir_output_does_not_copy_gen_siblings() {
    let temp = TempDir::new().expect("tempdir");
    let project = temp.path();
    write_minimal_crate(project);
    std::fs::write(
        project.join("src/ffi/types.wj"),
        r#"
pub struct ApiToken {
    pub value: i32,
}
"#,
    )
    .unwrap();
    seed_gen_root_decoys(project);

    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let status = Command::new(&wj)
        .args([
            "build",
            project.join("src/ffi").to_str().unwrap(),
            "--output",
            project.join("gen/ffi").to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .status()
        .expect("run wj");

    assert!(status.success(), "directory subdir build should succeed");

    assert!(!project.join("gen/ffi/decoy_module.rs").exists());
    assert!(!project.join("gen/ffi/another_decoy.rs").exists());

    let mod_rs = std::fs::read_to_string(project.join("gen/ffi/mod.rs")).unwrap();
    assert!(!mod_rs.contains("decoy_module"), "mod.rs polluted:\n{mod_rs}");
    assert!(!mod_rs.contains("another_decoy"), "mod.rs polluted:\n{mod_rs}");
}
