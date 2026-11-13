//! Agent-cost ratchet (docs/adr/0037-agent-cost-levers.md). The offline cost proxies in docs/generated/measured.json
//! (`cost.*`, written by bench/agent-dx/friction/replay.wj and cost_proxies.wj) may not regress against
//! bench/agent-dx/friction/ratchet.json: friction events, build noise and diagnostic tokens may only shrink, fix
//! coverage may only grow, and applying a fix may never create a diagnostic. An improvement must be written into the
//! ratchet in the same change (so the next regression is caught from the new level).
//! Also: the friction corpus is well formed and in step with its replay (status.json, open-bugs.md).

use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(rel: &str) -> Value {
    let p = root().join(rel);
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Violations of `ratchet` by `measured` (`{"key": {"value": n}}`). `shrink` keys: measured <= limit; `grow`: >=;
/// `zero`: == 0. A measured value strictly better than its limit is reported too (tighten the ratchet).
fn violations(measured: &Value, ratchet: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for (dir, sign) in [("shrink", 1.0), ("grow", -1.0), ("zero", 0.0)] {
        let Some(keys) = ratchet[dir].as_object() else { continue };
        for (k, limit) in keys {
            let limit = if dir == "zero" { 0.0 } else { limit.as_f64().unwrap_or(f64::NAN) };
            let Some(v) = measured[k]["value"].as_f64() else {
                out.push(format!("{k}: missing from docs/generated/measured.json (run the friction replay)"));
                continue;
            };
            let worse = if dir == "zero" { v != 0.0 } else { (v - limit) * sign > 1e-9 };
            let better = dir != "zero" && (limit - v) * sign > 1e-9;
            if worse {
                out.push(format!("{k}: {v} regressed past the ratchet {limit} ({dir}-only)"));
            } else if better {
                out.push(format!("{k}: {v} is better than the ratchet {limit}: tighten bench/agent-dx/friction/ratchet.json"));
            }
        }
    }
    out
}

#[test]
fn cost_proxies_do_not_regress() {
    let v = violations(&read_json("docs/generated/measured.json"), &read_json("bench/agent-dx/friction/ratchet.json"));
    assert!(v.is_empty(), "agent-cost ratchet:\n  {}", v.join("\n  "));
}

/// Paths of the repository whose changes since `commit` alter what the measurement would read: the feeding code.
/// `Err` when `commit` is unknown to this checkout (a rewritten history: measure again).
fn feed_changes_since(repo: &std::path::Path, commit: &str, feeds: &[String]) -> Result<Vec<String>, String> {
    let git = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(repo).args(args).output();
    let known = git(&["cat-file", "-e", &format!("{commit}^{{commit}}")]).map(|o| o.status.success()).unwrap_or(false);
    if !known {
        return Err(format!("commit {commit} is not in this checkout"));
    }
    let mut args = vec!["log", "--format=%h %s", "--no-merges"];
    let range = format!("{commit}..HEAD");
    args.push(&range);
    args.push("--");
    args.extend(feeds.iter().map(String::as_str));
    let out = git(&args).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect())
}

/// The measurement may not be older than the code that feeds it: every ratcheted key was measured at a commit after
/// the last change to `feeds` in ratchet.json (diagnostic rendering, the run driver, the source map, the corpus and
/// the replay). Otherwise the ratchet test passes on numbers the current compiler no longer produces.
#[test]
fn measured_json_is_not_older_than_the_code_that_feeds_it() {
    if !root().join(".git").exists() {
        return; // a source tarball has no history to compare with
    }
    let ratchet = read_json("bench/agent-dx/friction/ratchet.json");
    let measured = read_json("docs/generated/measured.json");
    let feeds: Vec<String> = ratchet["feeds"].as_array().expect("ratchet.json lists `feeds`").iter().filter_map(|f| f.as_str().map(str::to_string)).collect();
    assert!(!feeds.is_empty());
    let mut problems = Vec::new();
    for dir in ["shrink", "grow", "zero"] {
        for key in ratchet[dir].as_object().into_iter().flatten().map(|(k, _)| k) {
            let Some(commit) = measured[key]["commit"].as_str() else {
                problems.push(format!("{key}: no commit recorded in measured.json"));
                continue;
            };
            match feed_changes_since(&root(), commit, &feeds) {
                Ok(changes) if changes.is_empty() => {}
                Ok(changes) => problems.push(format!("{key}: measured at {commit}, feeding code changed since: {}", changes.join("; "))),
                Err(e) => problems.push(format!("{key}: {e}")),
            }
        }
    }
    problems.sort();
    problems.dedup_by(|a, b| a.split_once(": ").map(|x| x.1) == b.split_once(": ").map(|x| x.1));
    assert!(
        problems.is_empty(),
        "docs/generated/measured.json is older than the code that feeds it. Build wj at the head and run\n  \
         wj run --interpret --allow 'fs.read=**' --allow 'fs.write=**' --allow 'process=*' --allow 'env.read=*' bench/agent-dx/friction/replay.wj -- --wj /abs/path/wj --write\n\
         then commit measured.json and tighten ratchet.json:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn staleness_check_notices_a_feeding_change_and_ignores_others() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    let git = |args: &[&str]| {
        let o = std::process::Command::new("git").arg("-C").arg(r).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
        assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    git(&["init", "-q"]);
    std::fs::create_dir_all(r.join("feed")).unwrap();
    std::fs::write(r.join("feed/a.rs"), "1").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-qm", "measured here"]);
    let at = git(&["rev-parse", "HEAD"]);
    let feeds = vec!["feed".to_string()];
    std::fs::write(r.join("other.txt"), "x").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-qm", "unrelated"]);
    assert_eq!(feed_changes_since(r, &at, &feeds), Ok(vec![]));
    std::fs::write(r.join("feed/a.rs"), "2").unwrap();
    git(&["commit", "-qam", "feeding change"]);
    assert_eq!(feed_changes_since(r, &at, &feeds).unwrap().len(), 1);
    assert!(feed_changes_since(r, "0000000000000000000000000000000000000000", &feeds).is_err());
}

#[test]
fn ratchet_check_fails_on_regression_and_on_slack() {
    let ratchet = serde_json::json!({"shrink": {"a": 3}, "grow": {"b": 0.5}, "zero": {"c": 0}});
    let ok = serde_json::json!({"a": {"value": 3}, "b": {"value": 0.5}, "c": {"value": 0}});
    assert!(violations(&ok, &ratchet).is_empty());
    let worse = serde_json::json!({"a": {"value": 4}, "b": {"value": 0.4}, "c": {"value": 1}});
    assert_eq!(violations(&worse, &ratchet).len(), 3);
    let slack = serde_json::json!({"a": {"value": 2}, "b": {"value": 0.6}, "c": {"value": 0}});
    assert_eq!(violations(&slack, &ratchet).len(), 2);
    assert_eq!(violations(&serde_json::json!({}), &ratchet).len(), 3);
}

const CLASSES: [&str; 7] =
    ["compiler-bug", "user-error-good-message", "unhelpful-message", "noise", "skipped-loop", "program-output", "other"];

#[test]
fn friction_corpus_is_well_formed_and_replayed() {
    let events = root().join("bench/agent-dx/friction/events");
    let status = read_json("bench/agent-dx/friction/status.json");
    let open_bugs = std::fs::read_to_string(root().join("bench/agent-dx/friction/open-bugs.md")).unwrap();
    let mut n = 0;
    let mut problems = Vec::new();
    for e in std::fs::read_dir(&events).unwrap().flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        n += 1;
        let ev: Value = match std::fs::read_to_string(e.path().join("event.json")).ok().and_then(|t| serde_json::from_str(&t).ok()) {
            Some(v) => v,
            None => {
                problems.push(format!("{id}: no readable event.json"));
                continue;
            }
        };
        if ev["id"] != id.as_str() || !CLASSES.contains(&ev["class"].as_str().unwrap_or("")) || ev["command"].as_str().is_none() {
            problems.push(format!("{id}: event.json lacks a matching id, a known class or the command"));
        }
        if !e.path().join("observed.txt").exists() {
            problems.push(format!("{id}: no observed.txt"));
        }
        let has_src = e.path().join("main.wj").exists();
        if has_src != (ev["source"] != "none") {
            problems.push(format!("{id}: `source` says {} but main.wj {}", ev["source"], if has_src { "exists" } else { "is missing" }));
        }
        match status["events"][&id]["status"].as_str() {
            None => problems.push(format!("{id}: not in status.json (re-run replay.wj --write)")),
            Some("open-compiler-bug") if !open_bugs.contains(&id) => problems.push(format!("{id}: open compiler bug missing from open-bugs.md")),
            _ => {}
        }
    }
    assert!(n >= 300, "the corpus has {n} events; it was extracted with 414");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
