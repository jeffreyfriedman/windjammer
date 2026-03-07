/// TDD Test: Field assignment from &String parameter to String field
///
/// Problem: self.data.field = data where data: &String, field: String
/// Error: expected `String`, found `&String`
///
/// Solution: Auto-insert .clone() or .to_string() when assigning borrowed to owned field
use std::fs;
use std::process::Command;

#[test]
fn test_assign_borrowed_to_owned_field() {
    let source = r#"
struct Data {
    value: string
}

struct Manager {
    data: Data
}

impl Manager {
    pub fn set_value(self, value: string) {
        self.data.value = value
    }
}

fn main() {
    let mut mgr = Manager { data: Data { value: "initial" } }
    mgr.set_value("updated")
    println("{}", mgr.data.value)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    // WINDJAMMER DESIGN: String params infer to &str (not String!)
    // When assigning borrowed param to owned field:
    // - value: &str (borrowed parameter, read-only inference)
    // - self.data.value: String (owned field)
    // - Must clone: self.data.value = value.clone() or value.to_string()
    //
    // This is CORRECT! Converting &str to String requires allocation.
    assert!(
        generated.contains("value: &str"),
        "Should generate &str parameter. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("self.data.value = value.clone()")
            || generated.contains("self.data.value = value.to_string()"),
        "Should clone/convert &str to String for owned field. Got:\n{}",
        generated
    );

    fs::remove_dir_all(&test_dir).ok();
}
