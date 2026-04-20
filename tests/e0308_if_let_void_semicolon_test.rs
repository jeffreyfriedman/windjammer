use std::process::Command;

fn compile_wj_to_rust(input: &str) -> String {
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = temp_dir.path().join("test.wj");
    std::fs::write(&input_path, input).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(input_path.to_str().unwrap())
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("wj stderr: {}", stderr);

    // Search for generated .rs files
    for dir_name in ["build", "src", "."] {
        let dir = temp_dir.path().join(dir_name);
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "rs") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            return content;
                        }
                    }
                }
            }
        }
    }

    format!("No .rs file found in temp dir {:?}", temp_dir.path())
}

#[test]
fn test_if_let_void_block_adds_semicolon() {
    let input = r#"
use std::collections::HashMap

struct Asset {
    id: i64,
    name: string,
    status: i32,
}

struct Manager {
    assets: HashMap<i64, Asset>,
}

impl Manager {
    pub fn update_status(self, asset_id: i64) -> bool {
        if let Some(asset) = self.assets.get(asset_id) {
            let mut copy = asset.clone()
            copy.status = 1
            self.assets.insert(asset_id, copy)
        }
        true
    }
}
"#;

    let rust_code = compile_wj_to_rust(input);
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("self.assets.insert(asset_id, copy);"),
        "Expected semicolon after insert in if-let block.\nGenerated:\n{}",
        rust_code
    );
}
