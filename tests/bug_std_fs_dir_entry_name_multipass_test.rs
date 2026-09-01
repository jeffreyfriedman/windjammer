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

//! FAILING REPRO — multipass `--module-file` `DirEntry.name()` must wire to `file_name()`.
//!
//! Complements `bug_std_fs_dir_entry_name_wiring_test` (single-file) with the
//! `wj-migrate` ecosystem fixture under full library multipass.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MIGRATE_DIR_ENTRY: &str = include_str!("fixtures/stdlib/migrate_dir_entry_name.wj");

#[test]
fn std_fs_dir_entry_name_multipass_links_to_runtime_file_name() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod migrate
"#,
    );
    test.add_file("migrate.wj", MIGRATE_DIR_ENTRY);

    let map = test
        .compile()
        .expect("DirEntry.name multipass compile should succeed");
    let migrate = map.get("migrate.rs").expect("migrate.rs");

    assert!(
        migrate.contains("file_name("),
        "DirEntry.name() must codegen to runtime file_name(); emitted:\n{migrate}"
    );
    assert!(
        !migrate.contains("entry.name()"),
        "must not emit missing DirEntry::name() method call; emitted:\n{migrate}"
    );
    test.cargo_check()
        .expect("DirEntry.name wiring must cargo-check");
}
