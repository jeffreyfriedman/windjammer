//! Dead-capability gate: code in `src/` that no user can reach from `wj`, the LSP or the MCP server
//! is a capability that exists but was never wired (historically: `ejector.rs` without `wj eject`).
//!
//! Ratchet: `docs/reachability-allowlist.toml`. `[dead_files]` must equal the measured set (a new
//! dead file fails; a listed file that became reachable fails until its entry is removed);
//! `[dead_pub_fns]` counts per file may only go down. `REACH_BLESS=1` rewrites the counts only
//! when every count shrinks. Library API that is intentionally not reachable from a driver is
//! declared in `docs/MODULE_MAP.toml` under `[library_api]` (file prefix = reason).

#[path = "common/reach/mod.rs"]
mod reach;

use reach::scan::scan_source;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const ALLOWLIST: &str = "docs/reachability-allowlist.toml";

#[derive(serde::Deserialize, Default)]
struct Allow {
    #[serde(default)]
    dead_files: BTreeMap<String, String>,
    #[serde(default)]
    dead_pub_fns: BTreeMap<String, usize>,
}

#[derive(serde::Deserialize, Default)]
struct MapApi {
    #[serde(default)]
    library_api: BTreeMap<String, String>,
}

fn library_api() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join("docs/MODULE_MAP.toml")).unwrap();
    toml::from_str::<MapApi>(&text).unwrap().library_api
}

fn exempt(api: &BTreeMap<String, String>, file: &str) -> bool {
    api.keys().any(|p| file.starts_with(p.as_str()))
}

fn measured() -> reach::Report {
    let (items, errors) = reach::scan_repo(&root());
    assert!(errors.is_empty(), "files syn cannot parse (the gate never skips silently):\n{}", errors.join("\n"));
    assert!(items.len() > 5000, "scan found only {} items: scanner broken?", items.len());
    let api = library_api();
    let mut r = reach::report(&items);
    r.dead_files.retain(|f| !exempt(&api, f));
    r.dead_pub_fns.retain(|f, _| !exempt(&api, f));
    r
}

fn render_allowlist(r: &reach::Report, reasons: &BTreeMap<String, String>) -> String {
    let mut s = String::from(
        "# Dead-capability ratchet. Read by tests/capability_reachability_test.rs (docs/testing/INVARIANTS.md).\n\
         # [dead_files]: src files with no item reachable from wj/LSP/MCP, with the reason. Must match exactly.\n\
         # [dead_pub_fns]: per-file count of unreachable pub fns/methods. May only go down (REACH_BLESS=1).\n\n[dead_files]\n",
    );
    for f in &r.dead_files {
        let why = reasons.get(f).cloned().unwrap_or_else(|| "TODO: wire into wj, move to library_api, or delete".into());
        s.push_str(&format!("{f:?} = {why:?}\n"));
    }
    s.push_str("\n[dead_pub_fns]\n");
    for (f, v) in &r.dead_pub_fns {
        s.push_str(&format!("{f:?} = {}\n", v.len()));
    }
    s
}

#[test]
fn every_capability_is_reachable_from_a_driver_or_allowlisted() {
    let r = measured();
    let text = std::fs::read_to_string(root().join(ALLOWLIST)).unwrap_or_default();
    let allow: Allow = toml::from_str(&text).expect("reachability allowlist parses");
    let mut problems = Vec::new();
    for f in &r.dead_files {
        if !allow.dead_files.contains_key(f) {
            let legacy = r.legacy_only.get(f).map(|v| format!(" (reachable only from the legacy `windjammer` binary: {})", v.join(", "))).unwrap_or_default();
            problems.push(format!("NEW dead capability: {f} is unreachable from wj/LSP/MCP{legacy}. Wire it into `wj`, declare it in MODULE_MAP [library_api], or delete it."));
        }
    }
    for f in allow.dead_files.keys() {
        if !r.dead_files.contains(f) {
            problems.push(format!("{f} is now reachable (or gone): remove it from [dead_files] in {ALLOWLIST}"));
        }
    }
    let mut shrunk = true;
    for (f, v) in &r.dead_pub_fns {
        let allowed = allow.dead_pub_fns.get(f).copied().unwrap_or(0);
        if v.len() > allowed {
            shrunk = false;
            problems.push(format!("{f}: {} unreachable pub fns (allowed {allowed}): {}", v.len(), v.join(", ")));
        }
    }
    let stale: Vec<_> = allow.dead_pub_fns.iter().filter(|(f, n)| r.dead_pub_fns.get(*f).map_or(0, |v| v.len()) < **n).map(|(f, _)| f.clone()).collect();
    if shrunk && std::env::var("REACH_BLESS").is_ok() {
        std::fs::write(root().join(ALLOWLIST), render_allowlist(&r, &allow.dead_files)).unwrap();
        return;
    }
    if !stale.is_empty() {
        problems.push(format!("counts went down in {} files: tighten with REACH_BLESS=1 (e.g. {})", stale.len(), stale[0]));
    }
    assert!(problems.is_empty(), "dead-capability gate:\n{}", problems.join("\n"));
}

/// Inventory for WP13 (single driver): capabilities that only the legacy binary reaches.
/// Printed with `--nocapture`; the legacy-subcommand parity gate is in single_driver_parity_test.rs.
#[test]
fn legacy_only_capabilities_are_reported() {
    let r = measured();
    for (f, v) in &r.legacy_only {
        eprintln!("legacy-only {f}: {}", v.join(", "));
    }
}

// ------------------------------------------------------------------------------------------
// Self-tests: hand-labelled programs (independent oracle) the analyzer must classify exactly.
// ------------------------------------------------------------------------------------------

fn analyze(files: &[(&str, &str)], roots: &[&str]) -> BTreeMap<String, bool> {
    let mut items = Vec::new();
    for (f, s) in files {
        items.extend(scan_source(f, s).unwrap());
    }
    let r = reach::graph::reach(&items, |f| reach::is_under(f, roots));
    items.iter().zip(r.reached).map(|(i, ok)| (i.name.clone(), ok)).collect()
}

const ROOT_SRC: &str = "fn main() { a(); let s = Shown::new(); s.show(); println!(\"{}\", via_macro()); }";
const LIB_SRC: &str = r#"
pub fn a() { b() }
fn b() {}
pub fn never_called() { b() }
pub struct Shown;
impl Shown { pub fn new() -> Self { Shown } pub fn show(&self) {} pub fn hidden(&self) {} }
pub struct Unused;
impl Unused { pub fn show(&self) {} }
impl std::fmt::Display for Shown { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { helper_of_display(); Ok(()) } }
fn helper_of_display() {}
pub fn via_macro() -> i32 { 1 }
#[cfg(test)] mod tests { #[test] fn t() { super::never_called(); super::Unused.show(); } }
"#;

#[test]
fn selftest_labelled_program_is_classified_exactly() {
    let got = analyze(&[("src/bin/wj.rs", ROOT_SRC), ("src/lib.rs", LIB_SRC)], &["src/bin/wj.rs"]);
    let want = [
        ("a", true),
        ("b", true),
        ("never_called", false),
        ("Shown", true),
        ("new", true),
        ("hidden", false),
        ("Unused", false),
        ("fmt", true),
        ("helper_of_display", true),
        ("via_macro", true),
    ];
    for (name, live) in want {
        assert_eq!(got.get(name).copied(), Some(live), "{name}: expected live={live}; all: {got:?}");
    }
    // `show` exists twice: Shown::show live, Unused::show dead; test-only callers never count.
    assert!(!got.contains_key("t"), "test functions must not be scanned");
}

#[test]
fn selftest_a_capability_used_only_by_tests_is_dead() {
    let lib = "pub fn eject_project() {}\n#[cfg(test)] mod tests { #[test] fn ok() { super::eject_project() } }";
    let got = analyze(&[("src/bin/wj.rs", "fn main() {}"), ("src/ejector.rs", lib)], &["src/bin/wj.rs"]);
    assert_eq!(got["eject_project"], false);
    let r = reach::report(&scan_source("src/ejector.rs", lib).unwrap());
    assert_eq!(r.dead_files, vec!["src/ejector.rs".to_string()], "a file with no reachable item is a dead file");
}

#[test]
fn selftest_wiring_a_capability_makes_it_live() {
    let lib = "pub fn eject_project() {}";
    let got = analyze(&[("src/bin/wj.rs", "fn main() { windjammer::ejector::eject_project() }"), ("src/ejector.rs", lib)], &["src/bin/wj.rs"]);
    assert!(got["eject_project"]);
}

#[test]
fn selftest_scanner_reads_real_wj_entry_points() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin/wj_cli_cmds.rs");
    let items = scan_source("src/bin/wj_cli_cmds.rs", &std::fs::read_to_string(p).unwrap()).unwrap();
    assert!(items.iter().any(|i| i.name == "run"), "wj_cli_cmds::run must be found");
}
