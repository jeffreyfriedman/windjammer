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
