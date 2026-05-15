use std::process::Command;
use std::{fs, path::Path};

fn setup_wj_build_and_build_dir(wj_code: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let test_root = tempfile::tempdir().expect("tempdir");
    let test_dir = test_root.path();
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, wj_code).expect("write test.wj");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", wj_file.to_str().unwrap()])
        .current_dir(test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let build_dir = test_dir.join("build");
    (test_root, build_dir)
}

#[test]
fn test_explicit_deref_both_borrowed() {
    // Case 1: *id == flag_id where BOTH are &String
    // Expected: Remove * → id == flag_id (both &String)
    let wj_code = r#"
pub fn check_flag(flag_id: string) -> bool {
    let flags: Vec<(string, bool)> = Vec::new()
    for (id, value) in &flags {
        if *id == flag_id {
            return *value
        }
    }
    false
}

pub fn main() {
    let test_flag = "test".to_string()
    let result = check_flag(test_flag)
}
"#;

    run_test(wj_code, "both_borrowed");
}

#[test]
fn test_explicit_deref_one_owned() {
    // Case 2: *id == flag_id where id is owned String, flag_id is borrowed &String
    // Expected: Add * to flag_id → *id == *flag_id (both String after deref)
    let wj_code = r#"
pub fn get_custom_flag(flag_id: string) -> bool {
    let custom_flags: Vec<(string, bool)> = Vec::new()
    for (id, value) in custom_flags {
        if *id == flag_id {
            return value
        }
    }
    false
}

pub fn main() {
    let test_flag = "test".to_string()
    let result = get_custom_flag(test_flag)
}
"#;

    run_test(wj_code, "one_owned");
}

fn run_test(wj_code: &str, test_name: &str) {
    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let manifest = build_dir.join("Cargo.toml");
    let cargo_output = Command::new("cargo")
        .args(["build", "--manifest-path", manifest.to_str().unwrap()])
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        let rs_file = build_dir.join("test.rs");
        if Path::new(&rs_file).exists() {
            let generated_code = fs::read_to_string(&rs_file).unwrap();
            println!("Generated Rust code:\n{}", generated_code);
        }
        panic!("Cargo build failed for {}:\n{}", test_name, stderr);
    }
}
