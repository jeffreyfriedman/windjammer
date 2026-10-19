//! `wj test --interpret`: the same discovery, filtering and reporting rules as the compiled run,
//! executing the discovered tests in the interpreter instead of compiling them.
//!
//! Discovery is `test_discovery::discover` (one implementation for both modes); only the
//! execution step differs. `--list`, `--filter`, `--allow-empty` and the "no tests found" error are
//! shared with the compiled path.

use super::test_discovery::{self, Discovery};
use super::test_extraction::TestScope;
use super::test_harness::select_files;
use crate::interpreter::runner::{report_tests, run_specs, TestSpec};
use anyhow::Result;
use std::path::Path;

/// The discovered tests as interpreter specs, filtered like the compiled run (a test is selected
/// when its name or display path contains the filter).
pub fn specs_for(discovery: &Discovery, filter: Option<&str>) -> Vec<TestSpec> {
    select_files(&discovery.files, filter)
        .into_iter()
        .flat_map(|f| f.tests.iter())
        .filter(|t| filter.map_or(true, |p| t.name.contains(p) || t.display_name().contains(p)))
        .map(|t| TestSpec {
            file: t.file.clone(),
            name: t.name.clone(),
            display_name: t.display_name(),
            mods: match &t.scope {
                TestScope::InlineMod(p) => p.clone(),
                _ => Vec::new(),
            },
            assoc: match &t.scope {
                TestScope::Assoc(ty) => Some(ty.clone()),
                _ => None,
            },
        })
        .collect()
}

/// Entry point of `wj test --interpret`. Exits the process with the test status on failure.
pub fn run_interpreted(
    path: Option<&Path>,
    filter: Option<&str>,
    list: bool,
    allow_empty: bool,
    json: bool,
) -> Result<()> {
    let test_dir = path.unwrap_or_else(|| Path::new("."));
    if !test_dir.exists() {
        anyhow::bail!("Test path does not exist: {:?}", test_dir);
    }
    let discovery = test_discovery::discover(test_dir)?;
    if !discovery.errors.is_empty() {
        anyhow::bail!("{}", discovery.error_summary());
    }
    if list {
        super::print_listing(&discovery, json);
        return Ok(());
    }
    if discovery.test_count() == 0 {
        return super::report_no_tests(test_dir, allow_empty, json);
    }
    let specs = specs_for(&discovery, filter);
    if specs.is_empty() {
        let f = filter.unwrap_or_default();
        if allow_empty {
            println!("No tests match filter {f:?} (--allow-empty).");
            return Ok(());
        }
        anyhow::bail!(
            "filter {f:?} matched none of the {} discovered test(s); use `wj test --list` to see them, or --allow-empty",
            discovery.test_count()
        );
    }
    let results = run_specs(specs);
    let code = report_tests(&results);
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}
