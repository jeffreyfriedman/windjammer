//! Keeps `docs/spec/inference.md`, the rule catalog (`ir::explain::rules::RULES`) and the divergence
//! register in sync: every rule has a spec section, every spec rule heading is cataloged, and every
//! divergence ID mentioned in the spec is defined in `inference-divergences.md`.

use std::collections::BTreeSet;
use windjammer::ir::explain::rules::RULES;

fn read(rel: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn is_rule_id(s: &str) -> bool {
    s.contains('-')
        && s.chars().all(|c| c.is_ascii_uppercase() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

fn spec_heading_ids(spec: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in spec.lines() {
        if let Some(h) = line.strip_prefix("### ") {
            for part in h.split(',') {
                let id = part.trim();
                if is_rule_id(id) {
                    out.insert(id.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn every_cataloged_rule_has_a_spec_section() {
    let ids = spec_heading_ids(&read("docs/spec/inference.md"));
    for r in RULES {
        assert!(ids.contains(r.id), "rule {} is in RULES but has no `### {}` section in docs/spec/inference.md", r.id, r.id);
    }
}

#[test]
fn every_spec_rule_heading_is_cataloged() {
    let ids = spec_heading_ids(&read("docs/spec/inference.md"));
    let cat: BTreeSet<&str> = RULES.iter().map(|r| r.id).collect();
    for id in ids {
        assert!(cat.contains(id.as_str()), "spec heading {id} is not in ir::explain::rules::RULES");
    }
}

fn divergence_ids(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let b = text.as_bytes();
    let mut i = 0;
    while let Some(p) = text[i..].find("D-") {
        let start = i + p;
        let prev = text[..start].chars().next_back();
        if prev.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-') {
            i = start + 2;
            continue;
        }
        let mut end = start + 2;
        while end < b.len() && (b[end].is_ascii_uppercase() || b[end] == b'-') {
            end += 1;
        }
        let id = text[start..end].trim_end_matches('-');
        if id.len() > 3 {
            out.insert(id.to_string());
        }
        i = end;
    }
    out
}

#[test]
fn every_divergence_cited_in_the_spec_is_defined_in_the_register() {
    let spec = divergence_ids(&read("docs/spec/inference.md"));
    let reg = read("docs/spec/inference-divergences.md");
    let defined: BTreeSet<String> = reg
        .lines()
        .filter_map(|l| l.strip_prefix("## "))
        .map(|h| h.split_whitespace().next().unwrap_or("").to_string())
        .collect();
    for id in spec {
        assert!(defined.contains(&id), "{id} is cited in inference.md but has no `## {id}` section in inference-divergences.md");
    }
}
