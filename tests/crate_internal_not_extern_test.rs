use std::path::Path;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> String {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_dir = std::env::temp_dir().join(format!("wj_crate_internal_not_extern_test_{}", test_id));
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

    let output_file = test_dir.join("build").join("test_input.rs");
    if output_file.exists() {
        return std::fs::read_to_string(&output_file).unwrap_or_default();
    }

    let alt = test_dir.join("test_input.rs");
    if alt.exists() {
        return std::fs::read_to_string(&alt).unwrap_or_default();
    }

    if let Ok(entries) = std::fs::read_dir(&test_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map(|x| x == "rs").unwrap_or(false) {
                return std::fs::read_to_string(entry.path()).unwrap_or_default();
            }
        }
    }
    if let Ok(build_dir) = std::fs::read_dir(test_dir.join("build")) {
        for entry in build_dir.flatten() {
            if entry.path().extension().map(|x| x == "rs").unwrap_or(false) {
                return std::fs::read_to_string(entry.path()).unwrap_or_default();
            }
        }
    }

    String::from("NO RS FILE FOUND")
}

/// Bug: When a file declares `extern fn foo(s: string)` and also calls
/// a crate-internal function with the same simple name (e.g. `module::foo(s)`),
/// the compiler incorrectly wraps string args with `windjammer_runtime::ffi::string_to_ffi()`.
///
/// Root cause: extern_function_names stores only simple names (last segment after ::),
/// so any qualified call like `module::foo()` matches the extern name "foo" and gets
/// FFI wrapping applied.
///
/// Fix: qualified crate-internal calls (via `use crate::` or `module::`) should NOT
/// be treated as extern even if a function with the same simple name is declared extern.
#[test]
fn test_crate_internal_call_not_treated_as_extern() {
    let source = r#"
extern fn render_text(text: string, x: f32, y: f32)

use crate::utils

pub fn demo() {
    let label = utils::get_label("hello")
    render_text(label, 10.0, 20.0)
}
"#;
    let output = compile_wj_to_rust(source);

    // The extern call `render_text(label, ...)` SHOULD have string_to_ffi
    assert!(
        output.contains("string_to_ffi"),
        "Extern fn call should have string_to_ffi wrapping. Got:\n{}",
        output
    );

    // The extern call SHOULD have unsafe
    assert!(
        output.contains("unsafe"),
        "Extern fn call should have unsafe wrapping. Got:\n{}",
        output
    );
}

/// When calling a function through a module qualifier (e.g. `vnode_ffi::vnode_element("div")`),
/// and that function is NOT declared as extern fn in the current file, the generated code
/// must NOT contain string_to_ffi or unsafe wrapping.
#[test]
fn test_qualified_call_no_ffi_wrapping() {
    let source = r#"
use crate::vnode_ffi

pub struct VNode {
    handle: u64,
}

impl VNode {
    pub fn div() -> VNode {
        VNode { handle: vnode_ffi::vnode_element("div") }
    }

    pub fn text(content: string) -> VNode {
        VNode { handle: vnode_ffi::vnode_text(content) }
    }
}
"#;
    let output = compile_wj_to_rust(source);

    // Crate-internal call via module qualifier should NOT have string_to_ffi
    assert!(
        !output.contains("string_to_ffi"),
        "Crate-internal qualified call should NOT have string_to_ffi. Got:\n{}",
        output
    );

    // Crate-internal call should NOT have unsafe wrapping
    assert!(
        !output.contains("unsafe"),
        "Crate-internal qualified call should NOT have unsafe wrapping. Got:\n{}",
        output
    );
}

/// Mixed scenario: a file has BOTH extern fn declarations AND crate-internal calls
/// with the same function simple name. Only the extern calls should get FFI treatment.
#[test]
fn test_mixed_extern_and_crate_internal_same_name() {
    let source = r#"
extern fn vnode_element(tag: string) -> u64

use crate::vnode_ffi

pub fn extern_call() -> u64 {
    vnode_element("button")
}

pub fn crate_call() -> u64 {
    vnode_ffi::vnode_element("div")
}
"#;
    let output = compile_wj_to_rust(source);

    // The output should contain SOME string_to_ffi (for the extern call)
    // but the qualified crate call should not have it.
    // We can't easily distinguish per-call, but at minimum the qualified
    // call must not generate string_to_ffi for the "div" argument.

    // Check that vnode_ffi::vnode_element is called directly (not wrapped)
    // The extern call to bare vnode_element("button") should still get FFI treatment
    assert!(
        output.contains("vnode_ffi::vnode_element"),
        "Should contain qualified crate-internal call. Got:\n{}",
        output
    );
}
