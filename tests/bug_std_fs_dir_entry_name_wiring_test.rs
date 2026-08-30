//! FAILING REPRO — `std::fs::DirEntry.name()` must map to runtime `file_name()`.
//!
//! Ecosystem `wj-migrate` `db_apply::discover_migration_paths` initially called
//! `entry.name()` per `std/fs.wj`. Generated Rust calls `DirEntry::name()` but
//! `windjammer_runtime::fs::DirEntry` exposes `file_name()` only (E0599).
//! Workaround: `entry.path()` + local basename helper.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_fs_dir_entry_name_links_to_runtime_file_name() {
    let source = r#"
use std::fs

pub fn first_file_name(dir: string) -> Result<string, string> {
    match fs.read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                if entry.is_file() {
                    return Ok(entry.name())
                }
            }
            Err("no files")
        },
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["read_dir", "file_name("]);
    assert!(
        !generated.contains(".name()"),
        "DirEntry.name() must codegen to runtime file_name(), not missing .name(); emitted:\n{generated}"
    );
}
