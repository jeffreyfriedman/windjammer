// TDD: Vec::with_capacity / push literal typing (usize, f64 unification)
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn wj_to_rs(test_wj: &str) -> (String, String) {
    let temp = tempdir().expect("tempdir");
    let wj = temp.path().join("cap.wj");
    fs::write(&wj, test_wj).expect("write .wj");
    let out = temp.path().join("out");
    fs::create_dir_all(&out).expect("out");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    if !output.status.success() {
        return (
            String::new(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        );
    }

    let rs = fs::read_dir(&out)
        .expect("read out")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.extension() == Some(std::ffi::OsStr::new("rs"))
                && p.file_stem().and_then(|s| s.to_str()) != Some("lib")
        });
    let rs = rs.expect("one .rs in out");
    (fs::read_to_string(&rs).expect("read rs"), String::new())
}

#[test]
fn test_vec_with_capacity_literal() {
    let test_wj = r#"
fn test() {
    let mut data = Vec::with_capacity(10)
    data.push(42)
}
"#;

    let (rust_code, err) = wj_to_rs(test_wj);
    assert!(
        err.is_empty() && !rust_code.is_empty(),
        "Compilation failed: {err}"
    );

    assert!(
        rust_code.contains("with_capacity(10_usize)") || rust_code.contains("with_capacity(10)"),
        "Vec::with_capacity should use usize or plain 10\nGenerated:\n{rust_code}"
    );
    assert!(
        !rust_code.contains("with_capacity(10_i32)"),
        "Should not use: with_capacity(10_i32)\n{rust_code}"
    );
}

#[test]
fn test_vec_with_capacity_variable() {
    let test_wj = r#"
fn test() {
    let size: int = 10
    let mut data = Vec::with_capacity(size)
    data.push(42)
}
"#;

    let (rust_code, err) = wj_to_rs(test_wj);
    assert!(
        err.is_empty() && !rust_code.is_empty(),
        "Compilation failed: {err}"
    );

    assert!(rust_code.contains("with_capacity(") && rust_code.contains("as usize"));
}

#[test]
fn test_vec_push_float_unification() {
    let test_wj = r#"
fn test(alpha: f64) {
    let mut data = Vec::new()
    data.push(alpha)
    data.push(0.5)
    data.push(32.0)
}
"#;

    let (rust_code, err) = wj_to_rs(test_wj);
    assert!(
        err.is_empty() && !rust_code.is_empty(),
        "Compilation failed: {err}"
    );

    assert!(
        (rust_code.contains("0.5_f64") || (rust_code.contains("0.5") && rust_code.contains("f64")))
            && rust_code.contains("push("),
        "literals should match f64 context\n{rust_code}"
    );
    let has_f32 = rust_code.contains("_f32");
    let has_f64 = rust_code.contains("_f64");
    if has_f32 && has_f64 {
        panic!("mixed f32/f64 in one Vec: {rust_code}");
    }
}
