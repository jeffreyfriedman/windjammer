#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! Gate (web-run warning): ambiguous glob re-exports.
//!
//! windjammer-ui `components/generated/mod.rs` historically did:
//!   pub use vnode::*;
//!   pub use vdom::*;
//! both exporting `VNode`, which triggers rustc:
//!   warning: ambiguous glob re-exports
//!
//! Desired codegen/module layout: only one public VNode path (prefer vdom::VNode),
//! or explicit `pub use vdom::{VNode, VElement, VText}` without dual globs.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn dual_vnode_glob_reexports_should_not_be_ambiguous() {
    // Document the rustc diagnostic we hit under `make web-run`.
    // This is a compile-fixture expectation for generated crates, not a WJ source program.
    let bad_mod = r#"
pub mod vnode { pub enum VNode { Text(String) } }
pub mod vdom { pub enum VNode { Text(String) } }
pub use vnode::*;
pub use vdom::*;
"#;

    let good_mod = r#"
pub mod vnode { pub enum VNode { Text(String) } }
pub mod vdom {
    pub enum VNode { Text(String) }
    pub struct VElement;
    pub struct VText;
}
// Prefer explicit vdom exports — do not also glob vnode::*
pub use vdom::{VElement, VNode, VText};
"#;

    let dir = tempfile::tempdir().expect("tempdir");
    let bad_path = dir.path().join("bad.rs");
    let good_path = dir.path().join("good.rs");
    std::fs::write(&bad_path, bad_mod).unwrap();
    std::fs::write(&good_path, good_mod).unwrap();

    let bad_out = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "lib", "-o"])
        .arg(dir.path().join("bad.rlib"))
        .arg(&bad_path)
        .output()
        .expect("rustc");
    let bad_err = String::from_utf8_lossy(&bad_out.stderr);
    assert!(
        bad_err.contains("ambiguous glob re-exports") || !bad_out.status.success(),
        "expected ambiguous glob warning/error for dual VNode globs, got:\n{}",
        bad_err
    );

    let good_out = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "lib", "-o"])
        .arg(dir.path().join("good.rlib"))
        .arg(&good_path)
        .output()
        .expect("rustc");
    let good_err = String::from_utf8_lossy(&good_out.stderr);
    assert!(
        good_out.status.success(),
        "explicit vdom re-exports should compile cleanly:\n{}",
        good_err
    );
    assert!(
        !good_err.contains("ambiguous glob re-exports"),
        "good layout must not warn about ambiguous globs:\n{}",
        good_err
    );
}
