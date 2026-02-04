/// Test that returning Some(&T) doesn't incorrectly add .clone()
///
/// Bug: Transpiler was adding .cloned() to Some(squad) when squad is already &Squad
/// This caused type mismatches: expected &Squad, found Squad
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
fn test_return_some_ref_no_clone() {
    let source = r#"
struct Squad {
    pub id: string,
}

struct Manager {
    squads: Vec<Squad>,
}

impl Manager {
    pub fn get_squad(&self, squad_id: string) -> Option<&Squad> {
        for squad in &self.squads {
            if squad.id == squad_id {
                return Some(squad)
            }
        }
        None
    }
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let result = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should NOT add .clone() or .cloned() to Some(squad)
    assert!(
        !result.contains("Some(squad.clone())"),
        "Should not add .clone() to Some(squad)"
    );
    assert!(
        !result.contains("Some(squad.cloned())"),
        "Should not add .cloned() to Some(squad)"
    );
    assert!(
        result.contains("Some(squad)"),
        "Should return Some(squad) without modification"
    );
}

#[test]
fn test_return_some_ref_mut_no_clone() {
    let source = r#"
struct Squad {
    pub id: string,
}

struct Manager {
    squads: Vec<Squad>,
}

impl Manager {
    pub fn get_squad_mut(&mut self, squad_id: string) -> Option<&mut Squad> {
        for squad in &mut self.squads {
            if squad.id == squad_id {
                return Some(squad)
            }
        }
        None
    }
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let result = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should NOT add .clone() or .cloned() to Some(squad)
    assert!(
        !result.contains("Some(squad.clone())"),
        "Should not add .clone() to Some(squad) in &mut return"
    );
    assert!(
        !result.contains("Some(squad.cloned())"),
        "Should not add .cloned() to Some(squad) in &mut return"
    );
    assert!(
        result.contains("Some(squad)"),
        "Should return Some(squad) without modification"
    );
}
