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

/// HashMap::remove() takes &Q where K: Borrow<Q>.
/// When key is &str (from string optimization), it can be passed directly.
/// When key is String/&String, it should be auto-borrowed.
#[test]
fn test_hashmap_remove_auto_borrows_key() {
    let source = r#"
use std::map::Map

pub struct Cache {
    pub items: Map<String, i32>,
}

impl Cache {
    pub fn remove_item(self, key: String) {
        self.items.remove(key)
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // With string optimization, key becomes &str, which can be passed directly
    // to HashMap::remove (String: Borrow<str>). Or it might be &key if &String.
    assert!(
        generated.contains(".remove(key)") || generated.contains(".remove(&key)"),
        "HashMap::remove() should accept the key (either &str directly or &key).\nGenerated:\n{}",
        generated
    );
}
