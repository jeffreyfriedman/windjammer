//! Build utilities for CLI - generate_mod_file, strip_main_functions.
//!
//! Extracted from main.rs for use by cli/build.rs when the cli feature is enabled.

mod file_operations;
mod module_generation;
mod nested_module_structure;
mod path_utilities;
pub use crate::rust_integration_tests::{
    find_project_root_with_tests, generate_tests_lib_rs, sync_rust_integration_tests,
    wire_tests_lib_into_crate_root,
};
pub use file_operations::strip_main_functions;
pub(crate) use module_generation::generate_mod_file_with_layout;
pub(crate) use module_generation::mod_file_layout_for_build;
pub use module_generation::{
    cleanup_stale_module_files, cleanup_stale_module_files_recursive, generate_mod_file,
};
pub use nested_module_structure::generate_nested_module_structure;
