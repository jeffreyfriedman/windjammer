// wj test - Run Windjammer tests
//
// Discovers and runs Windjammer test files (*_test.wj)

use anyhow::Result;
use std::path::Path;

pub fn execute(filter: Option<&str>) -> Result<()> {
    // Call the main run_tests function from main.rs
    // For now, test current directory with default settings
    crate::run_tests(
        Some(Path::new(".")),
        filter,
        false, // nocapture
        true,  // parallel
        false, // json
    )
}
