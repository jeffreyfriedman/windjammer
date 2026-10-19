//! Single-driver gate (WP13): the legacy `windjammer` binary (src/main.rs) must not offer a
//! subcommand that `wj` lacks. Drift between the two drivers is how `windjammer eject` existed for
//! months while `wj eject` did not. Known gaps are listed in `LEGACY_ONLY` (may only shrink; an entry
//! `wj` gains fails until it is removed). When the list is empty, retire src/main.rs.

#[allow(dead_code)]
#[path = "../src/bin/wj_cli_args.rs"]
mod wj_cli_args;

#[allow(dead_code)]
#[path = "../src/cli_args.rs"]
mod legacy_cli_args;

use clap::CommandFactory;
use std::collections::BTreeSet;

/// Legacy subcommands `wj` does not have yet: `name` -> owner/plan.
const LEGACY_ONLY: &[(&str, &str)] = &[("eject", "wt/eject restores `wj eject` (src/ejector.rs)")];

fn subcommands(cmd: &clap::Command) -> BTreeSet<String> {
    cmd.get_subcommands().filter(|s| !s.is_hide_set()).map(|s| s.get_name().to_string()).collect()
}

fn missing_in_wj(wj: &BTreeSet<String>, legacy: &BTreeSet<String>) -> BTreeSet<String> {
    legacy.difference(wj).cloned().collect()
}

#[test]
fn legacy_driver_has_no_subcommand_that_wj_lacks() {
    let wj = subcommands(&wj_cli_args::Cli::command());
    let legacy = subcommands(&legacy_cli_args::Cli::command());
    assert!(wj.len() > 10 && legacy.len() > 3, "clap trees not read: wj={wj:?} legacy={legacy:?}");
    let missing = missing_in_wj(&wj, &legacy);
    let allowed: BTreeSet<String> = LEGACY_ONLY.iter().map(|(n, _)| n.to_string()).collect();
    let new: Vec<_> = missing.difference(&allowed).collect();
    let fixed: Vec<_> = allowed.difference(&missing).collect();
    assert!(new.is_empty(), "legacy `windjammer` has subcommands `wj` lacks: {new:?}. Port them to `wj` (single driver).");
    assert!(fixed.is_empty(), "`wj` now has {fixed:?}: remove from LEGACY_ONLY in {}", file!());
}

#[test]
fn selftest_parity_detects_a_missing_subcommand() {
    let wj: BTreeSet<String> = ["build", "run"].iter().map(|s| s.to_string()).collect();
    let legacy: BTreeSet<String> = ["build", "eject", "run"].iter().map(|s| s.to_string()).collect();
    assert_eq!(missing_in_wj(&wj, &legacy), ["eject".to_string()].into_iter().collect());
}
