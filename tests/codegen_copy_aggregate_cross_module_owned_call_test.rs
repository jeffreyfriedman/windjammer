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

//! Cross-module Copy aggregate (`BatchHandle { id: u64 }`) with field-only body
//! usage must keep owned formals and owned call sites.
//!
//! Dogfood (types-crate `batch_release` → `arrow_batch_release`):
//! formal emits `handle: BatchHandle` but call site was `arrow_batch_release(&handle)`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn copy_aggregate_field_only_release_cross_module_no_ref_at_call_site() {
    let files = [
        (
            "handle.wj",
            r#"
pub struct BatchHandle {
    pub id: u64,
}

pub enum Data {
    Arrow(BatchHandle),
    Pure(BatchHandle),
}
"#,
        ),
        (
            "arrow.wj",
            r#"
use crate::handle::BatchHandle

extern fn arrow_batch_release_ffi(handle_id: u64)

pub fn arrow_batch_release(handle: BatchHandle) {
    arrow_batch_release_ffi(handle.id)
}

pub fn pure_batch_release(handle: BatchHandle) {
    let _ = handle
}
"#,
        ),
        (
            "batch.wj",
            r#"
use crate::handle::Data

pub fn batch_release(data: Data) {
    match data {
        Data::Pure(handle) => {
            crate::arrow::pure_batch_release(handle)
        }
        Data::Arrow(handle) => {
            crate::arrow::arrow_batch_release(handle)
        }
    }
}
"#,
        ),
        ("mod.wj", "pub mod handle\npub mod arrow\npub mod batch\n"),
    ];

    let map = test_utils::compile_project(&files);
    let batch = map.get("batch.rs").cloned().unwrap_or_default();
    let arrow = map.get("arrow.rs").cloned().unwrap_or_default();

    assert!(
        arrow.contains("fn arrow_batch_release(handle: BatchHandle)")
            || arrow.contains("fn arrow_batch_release(mut handle: BatchHandle)"),
        "arrow_batch_release must emit owned BatchHandle formal. Got:\n{arrow}"
    );
    assert!(
        !batch.contains("arrow_batch_release(&handle)"),
        "batch_release must pass owned handle, not &handle (types-crate dogfood). Got:\n{batch}"
    );
    assert!(
        batch.contains("arrow_batch_release(handle)"),
        "expected owned call site. Got:\n{batch}"
    );
}
