/// Test that pushing borrowed iterator vars to Vec<&T> doesn't add .clone()
///
/// Bug: When building Vec<&Quest> from iterator, transpiler was adding
/// .clone() to borrowed iterator variables, causing type mismatch:
/// expected Vec<&Quest>, found Vec<Quest>
#[test]
fn test_push_borrowed_iter_var_to_vec_ref() {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    let source = r#"
struct Quest {
    pub id: string,
    pub completed: bool,
}

struct QuestManager {
    quests: Vec<Quest>,
}

impl QuestManager {
    pub fn get_completed_quests(&self) -> Vec<&Quest> {
        let mut result = Vec::new()
        for quest in &self.quests {
            if quest.completed {
                result.push(quest)
            }
        }
        result
    }
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let result = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should NOT add .clone() when pushing to Vec<&T>
    assert!(
        !result.contains("result.push(quest.clone())"),
        "Should not add .clone() when pushing &Quest to Vec<&Quest>"
    );
    assert!(
        result.contains("result.push(quest)"),
        "Should push quest without .clone() to Vec<&Quest>"
    );
}

