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

    let success = result.status.success();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    let generated = if success {
        let rs_path = output.join("test.rs");
        std::fs::read_to_string(&rs_path).unwrap_or_default()
    } else {
        String::new()
    };

    (success, generated, stderr)
}

/// Bug: When a Windjammer function has a borrowed string parameter, the
/// generated Rust takes `&str`. Cross-crate callers passing a String must
/// auto-borrow (generate `&variable`). This failed because the metadata
/// serialized the param type as `Reference(Custom("str"))`, which the
/// deserializer couldn't handle, so the function's signature was lost.
///
/// Fix: Add Reference(...) support to deserialize_type so cross-crate
/// signatures with &str parameters are correctly loaded and auto-borrowing
/// works.
#[test]
fn test_borrowed_string_param_generates_borrow() {
    let source = r#"
extern fn read_text_file(path: string) -> string

pub fn load_scene() -> string {
    let path = "test.wjscene"
    let text = read_text_file(path)
    return text
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "wj build failed:\n{}", stderr);

    assert!(
        generated.contains("read_text_file")
            && (generated.contains("&path") || generated.contains("string_to_ffi")),
        "Expected auto-borrow (&path) or FFI wrapper for string param:\n{}",
        generated
    );
}
