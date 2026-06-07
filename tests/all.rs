//! Consolidated test binary — auto-generated module list from build.rs.
//!
//! All files matching `tests/*_test.rs` or `tests/*_tests.rs` are
//! automatically included. No manual registration needed.
//!
//! This eliminates ~800 separate link operations, reducing full test suite
//! runtime from 14+ minutes to ~2-3 minutes.
//!
//! Run all tests:  cargo test --release --test all
//! Run one module: cargo test --release --test all -- module_name::test_fn

include!(concat!(env!("OUT_DIR"), "/all_tests_generated.rs"));
