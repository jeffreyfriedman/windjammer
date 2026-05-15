use std::path::Path;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> String {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_dir = std::env::temp_dir().join(format!("wj_extern_unsafe_test_{}", test_id));
    let _ = std::fs::remove_dir_all(&test_dir);
    let _ = std::fs::create_dir_all(&test_dir);

    let input_file = test_dir.join("test_input.wj");
    std::fs::write(&input_file, source).unwrap();

    let wj_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let _output = Command::new(&wj_binary)
        .arg("build")
        .arg("--no-cargo")
        .arg("test_input.wj")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    for candidate in &[
        test_dir.join("build").join("test_input.rs"),
        test_dir.join("test_input.rs"),
    ] {
        if candidate.exists() {
            return std::fs::read_to_string(candidate).unwrap_or_default();
        }
    }
    fn find_rs(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.file_name().map(|f| f == name).unwrap_or(false) {
                    return Some(p);
                }
                if p.is_dir() {
                    if let Some(found) = find_rs(&p, name) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }
    if let Some(p) = find_rs(&test_dir, "test_input.rs") {
        return std::fs::read_to_string(p).unwrap_or_default();
    }
    String::from("NO RS FILE FOUND")
}

/// BUG: Module-qualified calls to extern functions (e.g. input::input_is_key_pressed)
/// must be wrapped in unsafe blocks. The compiler was incorrectly suppressing extern
/// detection for all module-qualified calls that were resolved via simple-name fallback.
///
/// Real example from breach-protocol:
///   input::input_is_key_pressed(input::KEY_W)
/// Generated: input::input_is_key_pressed(input::KEY_W)  // Missing unsafe!
/// Should be: (unsafe { input::input_is_key_pressed(input::KEY_W) })
#[test]
fn test_extern_fn_call_wrapped_in_unsafe() {
    let source = r#"
extern fn do_something(x: i32) -> bool

pub fn check() -> bool {
    do_something(42)
}
"#;
    let output = compile_wj_to_rust(source);

    assert!(
        output.contains("unsafe"),
        "Extern function calls should be wrapped in unsafe. Got:\n{}",
        output
    );
}

/// Crate-internal functions should NOT be wrapped in unsafe.
#[test]
fn test_internal_fn_not_wrapped_in_unsafe() {
    let source = r#"
pub fn helper(x: i32) -> bool {
    x > 0
}

pub fn check() -> bool {
    helper(42)
}
"#;
    let output = compile_wj_to_rust(source);

    assert!(
        !output.contains("unsafe"),
        "Internal function calls should NOT be wrapped in unsafe. Got:\n{}",
        output
    );
}
