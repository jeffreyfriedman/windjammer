//! Architecture fitness tests (docs/ARCHITECTURE_IMPROVEMENT_PLAN.md, WP Q1).
//!
//! Every check is a ratchet: measured numbers come from scanning `src/**/*.rs` and are compared
//! with `docs/architecture-allowlist.toml`. A number may never go up (or appear for a new file),
//! and the allow-list may never be higher than reality, so the baseline only moves down.
//! To shrink the baseline after an improvement run `scripts/depgraph.sh` (see its header).
//! The module contracts live in `docs/MODULE_MAP.toml`; module docs in `MODULE.md` files.

#[path = "common/arch_scan.rs"]
mod arch_scan;

use arch_scan::{compare, format_diff, per_file, Counts, SrcFile};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MAX_FN_LINES: usize = 300;
const MAX_FILE_LINES: usize = 2000;
const MODULE_MD_MAX_LINES: usize = 60;
const REQUIRED_HEADINGS: [&str; 8] = [
    "Purpose",
    "Inputs/Outputs",
    "Invariants",
    "Owns",
    "Never decides",
    "Allowed imports",
    "Entry points",
    "Tests",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------------------------
// Module map and allow-list loading
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct ModuleEntry {
    layer: String,
    owner_stage: String,
    module_md: String,
    allowed_imports: Vec<String>,
    known_violations: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct CrateEntry {
    module_md: String,
}

#[derive(Deserialize, Debug)]
struct ModuleMap {
    #[serde(default)]
    extra_module_md: Vec<String>,
    modules: BTreeMap<String, ModuleEntry>,
    crates: BTreeMap<String, CrateEntry>,
}

fn load_map() -> ModuleMap {
    let text = std::fs::read_to_string(root().join("docs/MODULE_MAP.toml")).expect("docs/MODULE_MAP.toml");
    toml::from_str(&text).expect("MODULE_MAP.toml parses")
}

type Allowlist = BTreeMap<String, Counts>;

const ALLOWLIST_PATH: &str = "docs/architecture-allowlist.toml";

fn load_allowlist() -> Allowlist {
    let Ok(text) = std::fs::read_to_string(root().join(ALLOWLIST_PATH)) else {
        return Allowlist::new();
    };
    toml::from_str(&text).expect("architecture-allowlist.toml parses")
}

fn allowed(section: &str) -> Counts {
    load_allowlist().remove(section).unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Measurements
// ---------------------------------------------------------------------------------------------

fn module_names(files: &[SrcFile]) -> BTreeSet<String> {
    files.iter().filter_map(|f| arch_scan::top_module(&f.rel)).filter(|m| m != "lib").collect()
}

/// Edge counts between top-level modules: `(from, to) -> number of crate:: references`.
fn module_edges(files: &[SrcFile]) -> BTreeMap<(String, String), u64> {
    let known = module_names(files);
    let mut edges = BTreeMap::new();
    for f in files {
        let Some(from) = arch_scan::top_module(&f.rel).filter(|m| known.contains(m)) else { continue };
        for to in arch_scan::crate_refs(&f.code) {
            if to != from && known.contains(&to) {
                *edges.entry((from.clone(), to)).or_insert(0) += 1;
            }
        }
    }
    edges
}

/// Per-file counts of references that the module map does not allow.
fn measure_violating_edges(files: &[SrcFile], map: &ModuleMap) -> Counts {
    let known = module_names(files);
    let mut out = Counts::new();
    for f in files {
        let Some(from) = arch_scan::top_module(&f.rel).filter(|m| known.contains(m)) else { continue };
        let Some(entry) = map.modules.get(&from) else { continue };
        for to in arch_scan::crate_refs(&f.code) {
            if to != from && known.contains(&to) && !entry.allowed_imports.contains(&to) {
                *out.entry(format!("{from}->{to} {}", f.rel)).or_insert(0) += 1;
            }
        }
    }
    out
}

fn measure_long_functions(files: &[SrcFile]) -> Counts {
    let mut out = Counts::new();
    for f in files {
        for func in arch_scan::functions(&f.code).into_iter().filter(|x| x.len > MAX_FN_LINES) {
            let e = out.entry(format!("{}::{}", f.rel, func.name)).or_insert(0);
            *e = (*e).max(func.len as u64);
        }
    }
    out
}

fn measure_long_files(files: &[SrcFile]) -> Counts {
    per_file(files, |f| {
        let n = f.raw.lines().count();
        if n > MAX_FILE_LINES {
            n as u64
        } else {
            0
        }
    })
}

fn measure_get_signature(files: &[SrcFile]) -> Counts {
    let n: u64 = files
        .iter()
        .filter(|f| !f.rel.ends_with("analyzer/signature_registry.rs"))
        .map(|f| arch_scan::count_get_signature(&f.code))
        .sum();
    Counts::from([("outside_signature_registry".to_string(), n)])
}

/// Every ratcheted section with its current measurement.
fn measure_all(files: &[SrcFile], map: &ModuleMap) -> BTreeMap<&'static str, Counts> {
    BTreeMap::from([
        ("forbidden_edges", measure_violating_edges(files, map)),
        ("long_functions", measure_long_functions(files)),
        ("long_files", measure_long_files(files)),
        ("env_var_reads", per_file(files, |f| arch_scan::count_env_reads(&f.code))),
        ("thread_local", per_file(files, |f| arch_scan::count_thread_local(&f.code))),
        ("static_state", per_file(files, |f| arch_scan::count_static_state(&f.code))),
        ("allow_dead_code", per_file(files, |f| arch_scan::count_allow_dead_code(&f.code))),
        ("crate_level_allow_dead_code", per_file(files, |f| arch_scan::count_crate_level_allow_dead_code(&f.code))),
        ("get_signature_calls", measure_get_signature(files)),
    ])
}

// ---------------------------------------------------------------------------------------------
// Ratchet assertions
// ---------------------------------------------------------------------------------------------

fn measured(section: &str) -> Counts {
    let files = arch_scan::load_src_files(&root());
    let map = load_map();
    measure_all(&files, &map).remove(section).expect("known section")
}

fn assert_no_regression(section: &str, advice: &str) {
    let diff = compare(&measured(section), &allowed(section));
    assert!(
        diff.regressions.is_empty(),
        "[{section}] architecture ratchet violated. {advice}\n{}",
        format_diff("regression", &diff.regressions)
    );
}

fn assert_allowlist_tight(section: &str) {
    let diff = compare(&measured(section), &allowed(section));
    assert!(
        diff.stale.is_empty(),
        "[{section}] the allow-list in {ALLOWLIST_PATH} is higher than reality. Good news: shrink it \
         (run scripts/depgraph.sh, which lowers the baseline) so the improvement cannot regress.\n{}",
        format_diff("stale", &diff.stale)
    );
}

macro_rules! ratchet_tests {
    ($section:literal, $no_regress:ident, $tight:ident, $advice:literal) => {
        #[test]
        fn $no_regress() {
            assert_no_regression($section, $advice);
        }
        #[test]
        fn $tight() {
            assert_allowlist_tight($section);
        }
    };
}

ratchet_tests!(
    "forbidden_edges",
    forbidden_edges_do_not_grow,
    forbidden_edges_allowlist_is_tight,
    "A module imports a module its layer may not depend on (see docs/MODULE_MAP.toml allowed_imports). \
     Move the shared item to a lower layer instead of adding the import."
);
ratchet_tests!(
    "long_functions",
    function_length_does_not_grow,
    function_length_allowlist_is_tight,
    "Functions must stay <= 300 lines; allow-listed giants may not grow. Extract named steps."
);
ratchet_tests!(
    "long_files",
    file_length_does_not_grow,
    file_length_allowlist_is_tight,
    "Files must stay <= 2000 lines; allow-listed files may not grow. Split by responsibility."
);
ratchet_tests!(
    "env_var_reads",
    env_var_reads_do_not_grow,
    env_var_reads_allowlist_is_tight,
    "Core code must not read env vars; take configuration from the driver."
);
ratchet_tests!(
    "thread_local",
    thread_local_does_not_grow,
    thread_local_allowlist_is_tight,
    "No new thread_local!; put state on a compilation session."
);
ratchet_tests!(
    "static_state",
    static_state_does_not_grow,
    static_state_allowlist_is_tight,
    "No new static atomics/OnceLock/lazy_static; put state on a compilation session."
);
ratchet_tests!(
    "allow_dead_code",
    allow_dead_code_does_not_grow,
    allow_dead_code_allowlist_is_tight,
    "Delete dead code instead of adding #[allow(dead_code)]."
);
ratchet_tests!(
    "crate_level_allow_dead_code",
    crate_level_allow_dead_code_does_not_grow,
    crate_level_allow_dead_code_allowlist_is_tight,
    "Do not add crate-level #![allow(dead_code)]."
);
ratchet_tests!(
    "get_signature_calls",
    get_signature_calls_do_not_grow,
    get_signature_calls_allowlist_is_tight,
    "Do not add bare-name get_signature(&str) lookups; resolve identities once."
);

// ---------------------------------------------------------------------------------------------
// Module map consistency
// ---------------------------------------------------------------------------------------------

#[test]
fn module_map_lists_every_top_level_module() {
    let files = arch_scan::load_src_files(&root());
    let map = load_map();
    let actual = module_names(&files);
    let listed: BTreeSet<String> = map.modules.keys().cloned().collect();
    let unlisted: Vec<_> = actual.difference(&listed).collect();
    let ghosts: Vec<_> = listed.difference(&actual).collect();
    assert!(unlisted.is_empty(), "src modules missing from docs/MODULE_MAP.toml: {unlisted:?}");
    assert!(ghosts.is_empty(), "docs/MODULE_MAP.toml lists modules that do not exist: {ghosts:?}");
}

#[test]
fn module_map_entries_are_well_formed() {
    let map = load_map();
    let layers = ["support", "syntax", "resolve", "types", "ownership", "ir", "backend", "tooling", "driver"];
    for (name, m) in &map.modules {
        assert!(layers.contains(&m.layer.as_str()), "{name}: unknown layer {}", m.layer);
        assert!(!m.owner_stage.is_empty(), "{name}: owner_stage is empty");
        for t in m.allowed_imports.iter().chain(&m.known_violations) {
            assert!(map.modules.contains_key(t), "{name}: references unknown module {t}");
        }
        let is_dir = root().join("src").join(name).is_dir();
        assert_eq!(is_dir, !m.module_md.is_empty(), "{name}: module_md must be set exactly for directory modules");
    }
}

#[test]
fn module_map_forbids_the_core_edges() {
    let map = load_map();
    let forbidden = [
        ("analyzer", "codegen"),
        ("ir", "codegen"),
        ("type_inference", "codegen"),
        ("parser", "codegen"),
        ("lexer", "codegen"),
        ("codegen", "compiler"),
        ("lexer", "parser"),
        ("parser", "analyzer"),
        ("lexer", "analyzer"),
    ];
    for (from, to) in forbidden {
        let m = &map.modules[from];
        assert!(!m.allowed_imports.iter().any(|a| a == to), "MODULE_MAP must not allow {from} -> {to}");
    }
}

#[test]
fn module_map_known_violations_match_the_allowlist() {
    let map = load_map();
    let mut from_allowlist: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for key in allowed("forbidden_edges").keys() {
        let (edge, _) = key.split_once(' ').expect("edge key");
        let (from, to) = edge.split_once("->").expect("edge");
        from_allowlist.entry(from.to_string()).or_default().insert(to.to_string());
    }
    for (name, m) in &map.modules {
        let listed: BTreeSet<String> = m.known_violations.iter().cloned().collect();
        let actual = from_allowlist.remove(name).unwrap_or_default();
        assert_eq!(listed, actual, "{name}: known_violations in MODULE_MAP.toml must equal the targets in {ALLOWLIST_PATH}");
    }
}

// ---------------------------------------------------------------------------------------------
// MODULE.md presence and shape
// ---------------------------------------------------------------------------------------------

fn module_md_paths(map: &ModuleMap) -> Vec<String> {
    let dirs = map.modules.values().map(|m| m.module_md.clone());
    let crates = map.crates.values().map(|c| c.module_md.clone());
    dirs.chain(crates).chain(map.extra_module_md.iter().cloned()).filter(|p| !p.is_empty()).collect()
}

fn module_md_problems(rel: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root().join(rel)) else {
        return vec![format!("{rel}: missing")];
    };
    let mut problems = Vec::new();
    let n = text.lines().count();
    if n > MODULE_MD_MAX_LINES {
        problems.push(format!("{rel}: {n} lines (max {MODULE_MD_MAX_LINES})"));
    }
    for h in REQUIRED_HEADINGS {
        let has = text.lines().any(|l| l.trim_start_matches('#').trim().starts_with(h) && l.starts_with('#'));
        if !has {
            problems.push(format!("{rel}: missing heading '{h}'"));
        }
    }
    problems
}

#[test]
fn every_mapped_module_has_a_valid_module_md() {
    let map = load_map();
    let problems: Vec<String> = module_md_paths(&map).iter().flat_map(|p| module_md_problems(p)).collect();
    assert!(problems.is_empty(), "MODULE.md problems:\n{}", problems.join("\n"));
}

#[test]
fn module_md_directories_are_all_in_the_map() {
    let map = load_map();
    let listed: BTreeSet<String> = module_md_paths(&map).into_iter().collect();
    let mut found = Vec::new();
    collect_module_md(&root().join("src"), &mut found);
    for p in found {
        let rel = p.strip_prefix(root()).unwrap().to_string_lossy().replace('\\', "/");
        assert!(listed.contains(&rel), "{rel} exists but is not listed in MODULE_MAP.toml (module_md or extra_module_md)");
    }
}

fn collect_module_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_module_md(&p, out);
        } else if p.file_name().is_some_and(|n| n == "MODULE.md") {
            out.push(p);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Generated dependency graph
// ---------------------------------------------------------------------------------------------

fn layer_of<'a>(map: &'a ModuleMap, m: &str) -> &'a str {
    map.modules.get(m).map_or("?", |e| e.layer.as_str())
}

fn render_deps_md(edges: &BTreeMap<(String, String), u64>, map: &ModuleMap) -> String {
    let rows: BTreeSet<&String> = edges.keys().map(|(f, _)| f).collect();
    let cols: BTreeSet<&String> = edges.keys().map(|(_, t)| t).collect();
    let mut s = String::from(
        "# Module dependency matrix\n\n<!-- GENERATED by scripts/depgraph.sh from `crate::<module>` references in src/. Do not edit. -->\n\n\
         Cell = number of `crate::<column>` references inside the row module. `!` marks an edge the layer \
         rules in docs/MODULE_MAP.toml do not allow (tracked in docs/architecture-allowlist.toml).\n\n",
    );
    s.push_str("| from \\ to |");
    cols.iter().for_each(|c| s.push_str(&format!(" {c} |")));
    s.push_str("\n|---|");
    cols.iter().for_each(|_| s.push_str("---:|"));
    s.push('\n');
    for r in &rows {
        s.push_str(&format!("| **{r}** ({}) |", layer_of(map, r)));
        for c in &cols {
            s.push_str(&match edges.get(&((*r).clone(), (*c).clone())) {
                Some(n) if is_violation(map, r, c) => format!(" {n}! |"),
                Some(n) => format!(" {n} |"),
                None => " |".to_string(),
            });
        }
        s.push('\n');
    }
    s
}

fn is_violation(map: &ModuleMap, from: &str, to: &str) -> bool {
    map.modules.get(from).is_some_and(|m| !m.allowed_imports.iter().any(|a| a == to))
}

fn render_deps_dot(edges: &BTreeMap<(String, String), u64>, map: &ModuleMap) -> String {
    let mut s = String::from("// GENERATED by scripts/depgraph.sh. Do not edit.\ndigraph deps {\n  rankdir=BT;\n  node [shape=box,fontsize=10];\n");
    let mut by_layer: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for (name, m) in &map.modules {
        by_layer.entry(m.layer.as_str()).or_default().push(name);
    }
    for (layer, names) in &by_layer {
        s.push_str(&format!("  subgraph cluster_{layer} {{\n    label=\"{layer}\";\n"));
        names.iter().for_each(|n| s.push_str(&format!("    \"{n}\";\n")));
        s.push_str("  }\n");
    }
    for ((from, to), n) in edges {
        let attrs = if is_violation(map, from, to) { ",color=red,fontcolor=red" } else { "" };
        s.push_str(&format!("  \"{from}\" -> \"{to}\" [label=\"{n}\"{attrs}];\n"));
    }
    s.push_str("}\n");
    s
}

fn assert_generated_fresh(rel: &str, expected: &str) {
    let actual = std::fs::read_to_string(root().join(rel)).unwrap_or_default();
    assert!(actual == expected, "{rel} is stale. Run scripts/depgraph.sh and commit the result.");
}

#[test]
fn committed_deps_md_is_fresh() {
    let files = arch_scan::load_src_files(&root());
    assert_generated_fresh("docs/generated/deps.md", &render_deps_md(&module_edges(&files), &load_map()));
}

#[test]
fn committed_deps_dot_is_fresh() {
    let files = arch_scan::load_src_files(&root());
    assert_generated_fresh("docs/generated/deps.dot", &render_deps_dot(&module_edges(&files), &load_map()));
}

// ---------------------------------------------------------------------------------------------
// Baseline regeneration (`scripts/depgraph.sh`). Ignored so it never runs in CI.
// ---------------------------------------------------------------------------------------------

/// New baseline = min(existing, measured) per key; refuses to raise any number. A missing
/// allow-list file is bootstrapped from the measurement.
fn shrink(existing: &Allowlist, measured: BTreeMap<&str, Counts>, bootstrap: bool) -> Allowlist {
    let mut out = Allowlist::new();
    for (section, now) in measured {
        let old = existing.get(section).cloned().unwrap_or_default();
        let diff = compare(&now, &old);
        assert!(
            bootstrap || diff.regressions.is_empty(),
            "[{section}] refusing to raise the baseline:\n{}",
            format_diff("regression", &diff.regressions)
        );
        out.insert(section.to_string(), now);
    }
    out
}

fn render_allowlist(a: &Allowlist) -> String {
    let mut s = String::from(
        "# Architecture ratchet baseline. Read by tests/architecture.rs.\n\
         # Numbers may only go DOWN. Lower them with scripts/depgraph.sh after an improvement.\n\
         # Sections: forbidden_edges (\"from->to file\" = reference count), long_functions (\"file::fn\" = lines),\n\
         # long_files, env_var_reads, thread_local, static_state, allow_dead_code,\n\
         # crate_level_allow_dead_code, get_signature_calls.\n",
    );
    for (section, counts) in a {
        s.push_str(&format!("\n[{section}]\n"));
        for (k, v) in counts {
            s.push_str(&format!("\"{k}\" = {v}\n"));
        }
    }
    s
}

#[test]
#[ignore = "writes docs/architecture-allowlist.toml and docs/generated/*; run via scripts/depgraph.sh"]
fn regenerate_architecture_baselines() {
    let files = arch_scan::load_src_files(&root());
    let map = load_map();
    let bootstrap = !root().join(ALLOWLIST_PATH).exists();
    let new = shrink(&load_allowlist(), measure_all(&files, &map), bootstrap);
    std::fs::write(root().join(ALLOWLIST_PATH), render_allowlist(&new)).unwrap();
    let gen = root().join("docs/generated");
    std::fs::create_dir_all(&gen).unwrap();
    let edges = module_edges(&files);
    std::fs::write(gen.join("deps.md"), render_deps_md(&edges, &map)).unwrap();
    std::fs::write(gen.join("deps.dot"), render_deps_dot(&edges, &map)).unwrap();
}
