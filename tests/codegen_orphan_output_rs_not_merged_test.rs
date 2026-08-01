#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Orphan `.rs` files that exist only under the build output (e.g. a post-build
//! `[[bin]]` entry left from a previous HTML emit step) must not be merged into
//! `lib.rs` / `mod.rs` as library modules.
//!
//! Bug: `should_merge_extra_module` returned true for any undeclared name without
//! a matching `.wj`, treating build-only artifacts as hand-written FFI.
//! Symptom: `pub mod bin_main;` pulls a `fn main()` binary into the lib; paths
//! like `my_crate::foo` then fail with rustc E0433 (mapped to WJ0006).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;

#[test]
fn test_orphan_output_bin_rs_not_merged_into_lib() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod api
pub mod emit
"#,
    );
    test.add_file(
        "api/mod.wj",
        r#"
pub fn ping() -> int {
    1
}
"#,
    );
    test.add_file(
        "emit.wj",
        r#"
pub fn build_page() -> string {
    "ok".to_string()
}
"#,
    );

    // Simulate a previous post-build HTML emit that left a binary entry in build/.
    fs::create_dir_all(test.build_dir()).expect("create build dir");
    fs::write(
        test.build_dir().join("bin_main.rs"),
        r#"fn main() {
    let html = orphan_fixture::build_page();
    print!("{}", html);
}
"#,
    )
    .expect("plant orphan bin_main.rs");

    let _map = test.compile().expect("compile should succeed");

    let lib = fs::read_to_string(test.build_dir().join("lib.rs"))
        .or_else(|_| fs::read_to_string(test.build_dir().join("mod.rs")))
        .expect("lib.rs or mod.rs");

    assert!(
        !lib.contains("pub mod bin_main"),
        "orphan build/bin_main.rs must not become a lib module. lib/mod.rs:\n{lib}"
    );
    assert!(
        !lib.contains("pub use bin_main"),
        "orphan build/bin_main.rs must not be re-exported. lib/mod.rs:\n{lib}"
    );
    assert!(
        lib.contains("pub mod api") || lib.contains("pub mod emit"),
        "real source modules must still be declared. lib/mod.rs:\n{lib}"
    );
}

#[test]
fn test_hand_written_output_ffi_without_main_still_merged() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod emit
"#,
    );
    test.add_file(
        "emit.wj",
        r#"
pub fn build_page() -> string {
    "ok".to_string()
}
"#,
    );

    // Legacy FFI pattern: hand-written Rust already present under build/ (no fn main).
    fs::create_dir_all(test.build_dir()).expect("create build dir");
    fs::write(
        test.build_dir().join("ffi.rs"),
        "pub fn native_ping() -> i32 { 7 }\n",
    )
    .expect("plant ffi.rs");

    let _map = test.compile().expect("compile should succeed");

    let lib = fs::read_to_string(test.build_dir().join("lib.rs"))
        .or_else(|_| fs::read_to_string(test.build_dir().join("mod.rs")))
        .expect("lib.rs or mod.rs");

    assert!(
        lib.contains("pub mod ffi"),
        "hand-written build/ffi.rs (non-binary) must still be merged. lib/mod.rs:\n{lib}"
    );
}
