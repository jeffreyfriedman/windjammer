use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let input = dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test.wj");
    let output = dir.path().join("output");
    std::fs::create_dir_all(&output).expect("create output dir");

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", input.to_str().unwrap(), "--no-cargo", "-o"])
        .arg(output.to_str().unwrap())
        .output()
        .expect("run wj");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let generated_path = output.join("test.rs");
    let generated = if generated_path.exists() {
        std::fs::read_to_string(&generated_path).unwrap_or_default()
    } else {
        String::new()
    };

    (result.status.success(), generated, combined)
}

/// HashMap::get() returns Option<&V>. When matching with Some(v),
/// v is &V. Wrapping in Some(v) should use *v for Copy types, not **v.
#[test]
fn test_hashmap_get_match_no_double_deref() {
    let source = r#"
use std::map::Map

pub struct Data {
    pub values: Map<String, i32>,
}

impl Data {
    pub fn get_value(self, key: string) -> Option<i32> {
        match self.values.get(key) {
            Some(v) => Some(v),
            None => None,
        }
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // Should NOT generate **v (double deref on i32)
    assert!(
        !generated.contains("**v"),
        "Should not generate double deref **v for Copy types.\nGenerated:\n{}",
        generated
    );
}
