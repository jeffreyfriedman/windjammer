//! Entry points the cache tests use but no `wj` command calls (declared in docs/MODULE_MAP.toml `[library_api]`).

use super::{enforce_size_budget, PruneReport};
use std::path::Path;

/// [`enforce_size_budget`] with a budget the test chooses (tests/cargo_cache_active_test.rs).
pub fn enforce_size_budget_for_test(tree: &Path, budget: u64, report: &mut PruneReport) -> u64 {
    enforce_size_budget(tree, budget, report)
}
