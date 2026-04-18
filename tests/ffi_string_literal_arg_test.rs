// TDD Test: String literals passed to extern fn should auto-convert to FfiString
//
// When a Windjammer extern fn declares a parameter as `string`,
// the generated Rust signature uses `FfiString`. String literal arguments
// should be automatically wrapped with `string_to_ffi()`.

use std::io::Write;

fn compile_wj(code: &str) -> String {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let wj_path = temp_dir.path().join("test.wj");
    let mut f = std::fs::File::create(&wj_path).unwrap();
    write!(f, "{}", code).unwrap();

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = std::process::Command::new(wj_bin)
        .arg("build")
        .arg(wj_path.to_str().unwrap())
        .arg("-o")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        panic!(
            "WJ compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let rs_path = temp_dir.path().join("test.rs");
    std::fs::read_to_string(&rs_path).unwrap_or_else(|_| {
        panic!("Generated .rs file not found at {:?}", rs_path);
    })
}

#[test]
fn test_string_literal_to_extern_fn_is_wrapped() {
    let code = r#"
extern fn save_file(path: string, data: i32)

pub fn do_save() {
    save_file("/tmp/test.txt", 42)
}
"#;
    let rust_code = compile_wj(code);

    assert!(
        rust_code.contains("string_to_ffi") || rust_code.contains("FfiString"),
        "String literal to extern fn should be wrapped with FfiString conversion.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("save_file(\"/tmp/test.txt\""),
        "Raw string literal should NOT be passed directly to extern fn.\nGenerated:\n{}",
        rust_code
    );
}
