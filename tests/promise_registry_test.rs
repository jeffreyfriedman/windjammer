//! Promise registry gate: every user-facing promise has acceptance tests that run through the
//! installed `wj` binary, including at least one NEGATIVE case (bad input rejected / diagnosed).
//!
//! Promises: every `wj` subcommand and long flag (read from the clap tree, so adding a flag forces
//! a registry entry and tests), every code in docs/reference/errors.md, and every feature statement
//! registered in docs/promises.toml (with a verbatim quote of its source). Acceptance cases live in
//! tests/promises/*.toml; which CLI promises a case covers is derived from clap's own parse of its
//! arguments, which codes it covers from its expected output, never from a hand-written claim.
//! `pending` promises are a ratchet (`max_pending` may only shrink). See docs/testing/INVARIANTS.md.

#[allow(dead_code)]
#[path = "../src/bin/wj_cli_args.rs"]
mod wj_cli_args;

#[allow(dead_code)]
#[path = "common/installed_sandbox.rs"]
mod installed_sandbox;

#[path = "common/promise/mod.rs"]
mod promise;

use clap::CommandFactory;
use installed_sandbox::{real_wj, Sandbox};
use promise::model::{self, Case, Registry};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cli() -> clap::Command {
    wj_cli_args::Cli::command()
}

fn derived_ids(root: &Path) -> BTreeMap<String, String> {
    let mut ids = model::cli_promises(&cli());
    for c in model::code_promises(root) {
        ids.insert(c, String::new());
    }
    ids
}

#[test]
fn registry_lists_every_cli_flag_and_documented_code() {
    let reg = model::load_registry(&root());
    let have: BTreeSet<String> = reg.promises.iter().map(|p| p.id.clone()).collect();
    let derived = derived_ids(&root());
    let missing: Vec<_> = derived.keys().filter(|id| !have.contains(*id)).collect();
    let stale: Vec<_> = have
        .iter()
        .filter(|id| (id.starts_with("wj ") || id.starts_with("code ")) && !derived.contains_key(*id))
        .collect();
    if !missing.is_empty() && std::env::var("PROMISES_BLESS").is_ok() {
        let mut text = std::fs::read_to_string(root().join("docs/promises.toml")).unwrap();
        for id in &missing {
            let st = derived[*id].replace('"', "'");
            text.push_str(&format!("\n[[promise]]\nid = {id:?}\nstatement = {st:?}\npending = \"no acceptance cases yet\"\n"));
        }
        std::fs::write(root().join("docs/promises.toml"), text).unwrap();
        return;
    }
    assert!(
        missing.is_empty(),
        "new promises without a registry entry in docs/promises.toml (a new flag/command/code needs acceptance tests):\n  {}\n\
         Add [[promise]] entries and cases in tests/promises/ (PROMISES_BLESS=1 appends them as pending).",
        missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n  ")
    );
    assert!(stale.is_empty(), "registry promises that no longer exist (removed flag/code?): {stale:?}");
}

#[test]
fn feature_promises_still_quote_their_source() {
    let reg = model::load_registry(&root());
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut problems = Vec::new();
    for p in reg.promises.iter().filter(|p| p.id.starts_with("feature ")) {
        let (Some(src), Some(q)) = (&p.source, &p.quote) else {
            problems.push(format!("{}: features need `source` and `quote`", p.id));
            continue;
        };
        let text = std::fs::read_to_string(root().join(src)).unwrap_or_default();
        if !norm(&text).contains(&norm(q)) {
            problems.push(format!("{}: {src} no longer says {q:?}: update or retire the promise", p.id));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// `id -> (positive cases, negative cases)`.
fn coverage(reg: &Registry, cases: &[(String, Case)]) -> BTreeMap<String, (usize, usize)> {
    let codes: BTreeSet<String> = reg.promises.iter().filter(|p| p.id.starts_with("code ")).map(|p| p.id.clone()).collect();
    let mut cov: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let c = cli();
    for (_, case) in cases.iter().filter(|(_, c)| c.xfail.is_none()) {
        let mut ids = model::covered_cli(&c, &case.args);
        ids.extend(case.covers.iter().cloned());
        if case.is_negative() {
            ids.extend(model::covered_codes(case, &codes));
        }
        for id in ids {
            let e = cov.entry(id).or_default();
            if case.is_negative() {
                e.1 += 1
            } else {
                e.0 += 1
            }
        }
    }
    cov
}

fn satisfied(id: &str, (pos, neg): (usize, usize)) -> bool {
    if id.starts_with("code ") {
        neg > 0
    } else {
        pos > 0 && neg > 0
    }
}

/// Problems with the registry/case pairing (pure, so the self-tests can feed it broken inputs).
fn audit(reg: &Registry, cases: &[(String, Case)]) -> Vec<String> {
    let cov = coverage(reg, cases);
    let mut problems = Vec::new();
    let known: BTreeSet<&str> = reg.promises.iter().map(|p| p.id.as_str()).collect();
    for (file, c) in cases {
        if !matches!(c.polarity.as_str(), "positive" | "negative") {
            problems.push(format!("{file}: {}: polarity must be positive|negative", c.name));
        }
        for id in &c.covers {
            if !known.contains(id.as_str()) {
                problems.push(format!("{file}: {}: covers unknown promise {id:?}", c.name));
            }
        }
        if c.is_negative() && c.exit == Some(0) && c.expected_texts().next().is_none() {
            problems.push(format!("{file}: {}: a negative case that exits 0 must name the diagnostic it expects", c.name));
        }
    }
    let mut pending = 0;
    for p in &reg.promises {
        let got = cov.get(&p.id).copied().unwrap_or((0, 0));
        match (&p.pending, satisfied(&p.id, got)) {
            (Some(_), true) => problems.push(format!("{}: now has positive+negative cases: remove `pending`", p.id)),
            (Some(_), false) => pending += 1,
            (None, false) => problems.push(format!("{}: needs >=1 positive and >=1 negative acceptance case (has {} / {})", p.id, got.0, got.1)),
            (None, true) => {}
        }
    }
    if pending > reg.max_pending {
        problems.push(format!("{pending} pending promises > max_pending {} (the ratchet only shrinks: add tests instead)", reg.max_pending));
    } else if pending < reg.max_pending {
        problems.push(format!("only {pending} pending promises: lower max_pending in docs/promises.toml to {pending}"));
    }
    problems
}

#[test]
fn every_promise_has_positive_and_negative_acceptance_cases() {
    let problems = audit(&model::load_registry(&root()), &model::load_cases(&root()));
    assert!(problems.is_empty(), "promise registry:\n{}", problems.join("\n"));
}

fn run_all(sb: &Sandbox, cases: &[(String, Case)]) -> Vec<String> {
    let cargo = std::env::var("WJ_PROMISES_CARGO").is_ok();
    let runnable: Vec<&(String, Case)> = cases.iter().filter(|(_, c)| cargo || c.needs.as_deref() != Some("cargo")).collect();
    let fails = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for chunk in runnable.chunks(runnable.len().div_ceil(4).max(1)) {
            let fails = &fails;
            s.spawn(move || {
                for (file, c) in chunk {
                    match (promise::runner::run_case(sb, c), &c.xfail) {
                        (Err(e), None) => fails.lock().unwrap().push(format!("{file}: {} ({}):\n{e}", c.name, c.polarity)),
                        (Ok(()), Some(bug)) => fails.lock().unwrap().push(format!("{file}: {}: xfail case now passes ({bug}): remove `xfail`", c.name)),
                        _ => {}
                    }
                }
            });
        }
    });
    fails.into_inner().unwrap()
}

#[test]
fn acceptance_cases_hold_for_the_installed_binary() {
    let sb = Sandbox::new(&real_wj());
    let cases = model::load_cases(&root());
    assert!(cases.len() > 50, "only {} acceptance cases", cases.len());
    let fails = run_all(&sb, &cases);
    assert!(fails.is_empty(), "{} broken promises:\n\n{}", fails.len(), fails.join("\n\n"));
}

#[test]
fn every_documented_code_is_explained_by_wj_explain() {
    let sb = Sandbox::new(&real_wj());
    let mut bad = Vec::new();
    for id in model::code_promises(&root()) {
        let code = id.trim_start_matches("code ").to_string();
        let r = promise::runner::run_wj(&sb, &sb.cwds[0].1, &["explain".into(), code.clone()], None, &[]);
        if r.code != Some(0) || !r.stdout.contains(&code) {
            bad.push(format!("wj explain {code}: exit {:?} {}", r.code, r.stderr.lines().next().unwrap_or("")));
        }
    }
    assert!(bad.is_empty(), "documented codes that `wj explain` cannot explain:\n{}", bad.join("\n"));
}

// ------------------------------------------------------------------------------------------
// Docs may only mention `wj` commands and flags that exist.
// ------------------------------------------------------------------------------------------

const DOCS: [&str; 7] = ["README.md", "docs/WHY.md", "docs/agent/WINDJAMMER_FOR_AGENTS.md", "llms.txt", "AGENTS.md", "docs/README.md", "docs/index.md"];

/// `wj ...` invocations in inline code spans and fenced shell lines.
fn wj_mentions(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim_start().trim_start_matches("$ ");
        if l.starts_with("wj ") {
            out.push(l.to_string());
        }
        for (i, span) in line.split('`').enumerate() {
            if i % 2 == 1 && span.starts_with("wj ") {
                out.push(span.to_string());
            }
        }
    }
    out
}

/// Unknown subcommand or long flag in one mention, if any. Values/placeholders are not checked.
fn bad_mention(cli: &clap::Command, mention: &str) -> Option<String> {
    let toks: Vec<&str> = mention.split_whitespace().skip(1).take_while(|t| !matches!(*t, "|" | "&&" | ";" | "#" | ">" | "2>&1")).collect();
    let first = toks.first()?;
    if first.starts_with('-') || first.starts_with('<') || first.chars().next().is_some_and(|c| c.is_uppercase()) {
        return None; // `wj --version`, `wj <cmd>`, `wj FILE` placeholders
    }
    let mut sub = cli.find_subcommand(first).cloned()?;
    for t in &toks[1..] {
        if let Some(s) = sub.find_subcommand(t).cloned() {
            sub = s;
            continue;
        }
        if let Some(l) = t.strip_prefix("--") {
            let l = l.split('=').next().unwrap().trim_end_matches(['`', ',', ')', '.']);
            if !l.is_empty() && !matches!(l, "help" | "version") && !sub.get_arguments().any(|a| a.get_long() == Some(l)) {
                return Some(format!("`wj {}` has no flag --{l}", sub.get_name()));
            }
        }
    }
    None
}

fn unknown_subcommand(cli: &clap::Command, mention: &str) -> Option<String> {
    let first = mention.split_whitespace().nth(1)?;
    let looks_like_cmd = first.chars().all(|c| c.is_ascii_lowercase() || c == '-') && !first.starts_with('-');
    (looks_like_cmd && cli.find_subcommand(first).is_none()).then(|| format!("`wj {first}` is not a wj subcommand"))
}

#[test]
fn docs_mention_only_real_wj_commands_and_flags() {
    let c = cli();
    let mut bad = Vec::new();
    for d in DOCS {
        let Ok(text) = std::fs::read_to_string(root().join(d)) else { continue };
        for m in wj_mentions(&text) {
            if let Some(e) = unknown_subcommand(&c, &m).or_else(|| bad_mention(&c, &m)) {
                bad.push(format!("{d}: {m:?}: {e}"));
            }
        }
    }
    assert!(bad.is_empty(), "docs promise commands/flags that `wj` does not have:\n{}", bad.join("\n"));
}

// ------------------------------------------------------------------------------------------
// Mutation self-tests: the gate must fail on a broken binary and on a broken registry.
// ------------------------------------------------------------------------------------------

/// A "binary" that runs the real wj but always exits 0: every negative case must catch it.
fn always_succeeds_mutant(dir: &Path) -> PathBuf {
    let p = dir.join("wj-mutant");
    std::fs::write(&p, format!("#!/bin/sh\n\"{}\" \"$@\"\nexit 0\n", real_wj().display())).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

#[cfg(unix)]
#[test]
fn selftest_mutant_that_never_fails_is_caught_by_negative_cases() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sb = Sandbox::new(&always_succeeds_mutant(tmp.path()));
    let cases: Vec<(String, Case)> = model::load_cases(&root()).into_iter().filter(|(_, c)| c.is_negative() && c.needs.is_none() && c.xfail.is_none() && c.exit != Some(0)).take(12).collect();
    assert!(cases.len() >= 10, "need negative cases to run the self-test");
    let fails = run_all(&sb, &cases);
    assert_eq!(fails.len(), cases.len(), "every negative case must reject the always-exit-0 mutant");
}

fn case(args: &[&str], polarity: &str) -> (String, Case) {
    ("t.toml".into(), Case { name: "t".into(), polarity: polarity.into(), args: args.iter().map(|s| s.to_string()).collect(), ..Default::default() })
}

fn promise(id: &str, pending: bool) -> model::Promise {
    model::Promise { id: id.into(), statement: String::new(), source: None, quote: None, pending: pending.then(|| "x".into()) }
}

#[test]
fn selftest_audit_rejects_missing_negative_and_loose_ratchet() {
    let reg = Registry { max_pending: 0, promises: vec![promise("wj check", false), promise("wj check --json", false)] };
    // Positive only: both promises lack a negative case.
    let p = audit(&reg, &[case(&["check", "--json", "x.wj"], "positive")]);
    assert_eq!(p.len(), 2, "{p:?}");
    // Negative for the subcommand but not for the flag.
    let p = audit(&reg, &[case(&["check", "--json", "x.wj"], "positive"), case(&["check", "bad.wj"], "negative")]);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].contains("wj check --json"));
    // Short flags count as their long name; a pending promise that gains tests must drop `pending`.
    let reg = Registry { max_pending: 1, promises: vec![promise("wj new", false), promise("wj new --template", true)] };
    let p = audit(&reg, &[case(&["new", "p", "-t", "lib"], "positive"), case(&["new", "p", "-t", "nope"], "negative")]);
    assert!(p.iter().any(|x| x.contains("remove `pending`")), "{p:?}");
    // max_pending higher than reality must be lowered.
    let reg = Registry { max_pending: 3, promises: vec![promise("wj new", true)] };
    assert!(audit(&reg, &[]).iter().any(|x| x.contains("lower max_pending")));
}

#[test]
fn selftest_new_flag_without_registry_entry_is_detected() {
    let mut c = cli();
    c = c.mut_subcommand("check", |s| s.arg(clap::Arg::new("brand-new").long("brand-new").action(clap::ArgAction::SetTrue)));
    assert!(model::cli_promises(&c).contains_key("wj check --brand-new"));
    assert!(!model::cli_promises(&cli()).contains_key("wj check --brand-new"));
}

#[test]
fn selftest_docs_scan_flags_unknown_commands() {
    let c = cli();
    assert!(unknown_subcommand(&c, "wj eject src out").is_some() || c.find_subcommand("eject").is_some());
    assert!(bad_mention(&c, "wj check --no-such-flag x.wj").is_some());
    assert!(bad_mention(&c, "wj check --json FILE").is_none());
    assert_eq!(wj_mentions("run `wj check --json f.wj` then\n$ wj run --interpret f.wj"), vec!["wj check --json f.wj".to_string(), "wj run --interpret f.wj".to_string()]);
}
