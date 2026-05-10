use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_dir() -> std::path::PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("wj_str_method_{pid}_{id}"))
}

fn compile_wj(source: &str) -> String {
    let dir = unique_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let wj_bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/wj");

    let output = Command::new(&wj_bin)
        .arg("build")
        .arg("test.wj")
        .current_dir(&dir)
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        eprintln!("wj build stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("wj build stdout: {}", String::from_utf8_lossy(&output.stdout));
    }

    let rs_file = dir.join("build/test.rs");
    let generated = if rs_file.exists() {
        std::fs::read_to_string(&rs_file).unwrap_or_default()
    } else {
        String::new()
    };

    let _ = std::fs::remove_dir_all(&dir);
    generated
}

/// When a method call passes a string literal to a parameter that expects
/// owned String, the codegen should add .to_string() to the literal.
#[test]
fn test_string_literal_to_method_expecting_string() {
    let source = r#"
struct Registry {
    items: Vec<String>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry { items: Vec::new() }
    }

    pub fn add_item(self, name: String) -> i32 {
        self.items.push(name)
        self.items.len() as i32
    }
}

pub fn test_string_literal_method() {
    let mut reg = Registry::new()
    let idx = reg.add_item("hello")
}
"#;

    let generated = compile_wj(source);
    assert!(!generated.is_empty(), "Generated Rust should not be empty");

    // The string literal "hello" must be converted to "hello".to_string()
    // when passed to add_item(name: String)
    assert!(
        generated.contains(r#""hello".to_string()"#),
        "String literal should be converted to .to_string() for method expecting String.\nGenerated:\n{}",
        generated
    );
}
