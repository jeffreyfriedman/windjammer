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
    feature = "integration_tests",
))]

//! FAILING REPRO — `std::mime` must codegen to `windjammer_runtime::mime`.
//!
//! Ecosystem `wj-mime` initially wrapped `std::mime`. Codegen emitted:
//! - `mime.APPLICATION_JSON` (E0423: expected value, found module — needs `mime::`)
//! - `windjammer_runtime::mime::APPLICATION_JSON` (E0432: unresolved import)
//! - aliased `mime.from_extension` imports resolve as modules, not fns (E0423)
//!
//! Runtime `windjammer_runtime::mime` exports `from_extension` / `from_path` but not
//! the std/mime.wj constants table. Stdlib wiring must align std/ and runtime exports.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_mime_from_extension_codegen_resolves() {
    let source = r#"
use std::mime

pub fn lookup(ext: string) -> string {
    mime.from_extension(ext)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("from_extension"),
        "must call from_extension, got:\n{generated}"
    );
    let wired = generated.contains("mime::from_extension")
        || generated.contains("windjammer_runtime::mime::from_extension");
    assert!(wired, "std::mime.from_extension must resolve to runtime, got:\n{generated}");
}

#[test]
fn std_mime_constants_codegen_resolves() {
    let source = r#"
use std::mime

pub fn json_type() -> string {
    mime.APPLICATION_JSON
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("mime.APPLICATION_JSON"),
        "must not use dot for module constants, got:\n{generated}"
    );
    assert!(
        generated.contains("APPLICATION_JSON"),
        "must reference APPLICATION_JSON, got:\n{generated}"
    );
}
