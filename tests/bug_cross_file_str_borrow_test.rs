#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Bug: Cross-file method signature not propagated to call-site codegen
//
// When struct A defines a method that takes `string` (inferred as `&str` by the
// analyzer), and struct B in a different file calls that method, the codegen
// generates `.clone()` instead of `&` for string field arguments.
//
// Root cause: library_multipass previously loaded same-crate `.wj.meta` then
// immediately dropped bare `Type::method` keys via a second
// `drop_dependency_signatures_for_local_types` call. Defining-module Borrowed
// ownership was lost on incremental skips; declaration stubs with Owned won.
// Fix: merge local `.wj.meta` after dependency filtering — do not drop again.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn cross_file_str_param_uses_borrow_not_clone() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    // File A: defines VNode with on_click taking string (inferred as &str)
    let vnode_wj = src.join("vnode.wj");
    std::fs::write(
        &vnode_wj,
        r#"
pub struct VNode {
    pub handle: i32,
}

impl VNode {
    pub fn new() -> VNode {
        VNode { handle: 0 }
    }

    pub fn on_click(self, handler_name: string) -> VNode {
        self
    }
}
"#,
    )
    .unwrap();

    // File B: Button calls VNode::on_click with self.click_handler
    let button_wj = src.join("button.wj");
    std::fs::write(
        &button_wj,
        r#"
use crate::vnode::VNode

pub struct Button {
    pub click_handler: string,
}

impl Button {
    pub fn to_vnode(self) -> VNode {
        let mut node = VNode::new()
        if !self.click_handler.is_empty() {
            node = node.on_click(self.click_handler)
        }
        node
    }
}
"#,
    )
    .unwrap();

    // Create wj.toml
    let toml = dir.path().join("wj.toml");
    std::fs::write(
        &toml,
        "[package]\nname = \"test_cross_file\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let wj = std::path::PathBuf::from(env!("CARGO_BIN_EXE_wj"));

    // Build 1: Full clean build (creates .wj.meta with converged ownership)
    let output = std::process::Command::new(&wj)
        .arg("build")
        .arg("--no-cargo")
        .arg(&src)
        .current_dir(dir.path())
        .output()
        .expect("wj build src/ (clean) failed");
    assert!(
        output.status.success(),
        "Clean build failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify clean build generated correct code
    let build_dir = dir.path().join("build");
    let button_rs = find_generated_button_rs(&build_dir);
    assert!(
        !button_rs.is_empty(),
        "Could not find generated button.rs after clean build"
    );

    let clean_has_clone = button_rs.contains("self.click_handler.clone()");
    let clean_has_borrow = button_rs.contains("&self.click_handler");
    assert!(
        !clean_has_clone,
        "Clean build should NOT generate .clone() for cross-file &str param.\n\
         Generated button.rs:\n{}",
        button_rs
    );

    // Touch only button.wj so vnode.wj is cached/skipped on next build
    std::thread::sleep(std::time::Duration::from_millis(200));
    let button_content = std::fs::read_to_string(&button_wj).unwrap();
    std::fs::write(&button_wj, format!("{}\n", button_content.trim())).unwrap();

    // Build 2: Incremental build (vnode.wj should be skipped)
    let output2 = std::process::Command::new(&wj)
        .arg("build")
        .arg("--no-cargo")
        .arg(&src)
        .current_dir(dir.path())
        .output()
        .expect("wj build src/ (incremental) failed");

    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        output2.status.success(),
        "Incremental build failed:\n{}{}",
        String::from_utf8_lossy(&output2.stdout),
        stderr2
    );

    // Verify incremental build was actually incremental (skipped files)
    let was_incremental = stderr2.contains("skipped") || stderr2.contains("Incremental");

    // Check the generated button.rs from the incremental build
    let button_rs_incr = find_generated_button_rs(&build_dir);
    assert!(
        !button_rs_incr.is_empty(),
        "Could not find generated button.rs after incremental build"
    );

    let incr_has_clone = button_rs_incr.contains("self.click_handler.clone()");

    // The key assertion: incremental build must NOT introduce .clone()
    assert!(
        !incr_has_clone,
        "Incremental build generated .clone() for cross-file &str param!\n\
         Was incremental: {}\n\
         Build stderr:\n{}\n\
         Generated button.rs:\n{}",
        was_incremental, stderr2, button_rs_incr
    );
}

fn find_generated_button_rs(build_dir: &std::path::Path) -> String {
    let candidates = [
        build_dir.join("button.rs"),
        build_dir.join("src").join("button.rs"),
    ];
    for path in &candidates {
        if path.exists() {
            return std::fs::read_to_string(path).unwrap_or_default();
        }
    }
    // Search recursively
    if let Ok(entries) = std::fs::read_dir(build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().map_or(false, |n| n == "button.rs") {
                return std::fs::read_to_string(&path).unwrap_or_default();
            }
            if path.is_dir() {
                let result = find_generated_button_rs(&path);
                if !result.is_empty() {
                    return result;
                }
            }
        }
    }
    String::new()
}
