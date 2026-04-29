/// TDD TEST: String parameters should work in match without .as_str()
///
/// WINDJAMMER PHILOSOPHY: Compiler handles string conversions automatically.
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_string_match_without_as_str() {
    let source = r#"
enum BuildType {
    Warrior,
    Rogue,
    Tech,
}

impl BuildType {
    pub fn from_name(name: string) -> BuildType {
        match name {
            "warrior" => BuildType::Warrior,
            "rogue" => BuildType::Rogue,
            "tech" => BuildType::Tech,
            _ => BuildType::Warrior,
        }
    }
}
"#;

    let dir = tempdir().expect("tempdir");
    let wj_path = dir.path().join("test_string_match.wj");
    fs::write(&wj_path, source).expect("write wj");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).expect("out");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build should succeed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_path = out.join("test_string_match.rs");
    let generated = fs::read_to_string(&rs_path).expect("Generated file should exist");

    let rlib = dir.path().join("libmatch_test.rlib");
    let compile = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            rs_path.to_str().unwrap(),
            "-o",
            rlib.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&compile.stderr);
    assert!(
        !stderr.contains("E0658"),
        "Generated Rust should not use unstable features\nStderr: {}",
        stderr
    );
    assert!(
        compile.status.success(),
        "Generated Rust should compile\nStderr: {}",
        stderr
    );

    assert!(
        generated.contains("match name {") || generated.contains("match name.as_str() {"),
        "Match statement should exist"
    );
}
