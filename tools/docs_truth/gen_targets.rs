//! `docs/reference/targets.md`: backend support matrix from `tests/conformance_programs/SUPPORT.toml`.

use super::util::{table, walk};
use super::Ctx;
use std::collections::BTreeMap;

pub const SUPPORT: &str = "tests/conformance_programs/SUPPORT.toml";

/// Backends exercised by the conformance harness: parsed from `ALL_BACKENDS` in its source.
pub fn backends(ctx: &Ctx) -> Vec<String> {
    let src = ctx.read("tests/common/conformance/backends.rs");
    let line = src.lines().find(|l| l.contains("pub const ALL_BACKENDS")).unwrap_or("");
    let mut out: Vec<String> = line.split("Backend::").skip(1).map(|r| r.chars().take_while(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()).collect();
    out.retain(|b| !b.is_empty());
    for (_, b, _) in unsupported(ctx) {
        if !out.contains(&b) {
            out.push(b);
        }
    }
    out
}

pub fn programs(ctx: &Ctx) -> Vec<String> {
    walk(&ctx.root.join("tests/conformance_programs"))
        .into_iter()
        .filter(|f| f.ends_with(".wj") && !f.contains('/'))
        .map(|f| f.trim_end_matches(".wj").to_string())
        .collect()
}

/// (program, backend, reason) of every known-unsupported pair.
pub fn unsupported(ctx: &Ctx) -> Vec<(String, String, String)> {
    let v: toml::Value = ctx.read(SUPPORT).parse().unwrap_or(toml::Value::Table(Default::default()));
    let mut out = Vec::new();
    for e in v.get("unsupported").and_then(|a| a.as_array()).into_iter().flatten() {
        let g = |k: &str| e.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        out.push((g("program"), g("backend"), g("reason")));
    }
    out.sort();
    out
}

pub fn targets(ctx: &Ctx) -> String {
    let progs = programs(ctx);
    let uns = unsupported(ctx);
    let mut rows = Vec::new();
    for b in backends(ctx) {
        let n = uns.iter().filter(|u| u.1 == b).count();
        rows.push(vec![format!("`{b}`"), progs.len().to_string(), (progs.len().saturating_sub(n)).to_string(), n.to_string()]);
    }
    let mut by_reason: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (p, b, r) in &uns {
        let head = r.split(':').next().unwrap_or(r).trim();
        let key = if head.contains(' ') { r.chars().take(70).collect::<String>() } else { head.to_string() };
        by_reason.entry((b.clone(), key)).or_default().push(p.clone());
    }
    let reason_rows: Vec<Vec<String>> = by_reason.iter().map(|((b, r), ps)| vec![format!("`{b}`"), r.clone(), ps.len().to_string()]).collect();
    format!(
        "# Backend support matrix\n\nGenerated from `{SUPPORT}` and the program corpus in `tests/conformance_programs/`. A listed pair must fail its oracle; the harness ratchets the list down.\n\n{}\n## Known-unsupported pairs by cause\n\n{}",
        table(&["Backend", "Corpus programs", "Passing", "Known unsupported"], &rows),
        table(&["Backend", "Cause", "Programs"], &reason_rows)
    )
}

/// Compact table for the README: conformance results per backend, computed from the same data as `targets()`.
pub fn summary(ctx: &Ctx) -> String {
    let progs = programs(ctx);
    let uns = unsupported(ctx);
    let rows: Vec<Vec<String>> = backends(ctx)
        .into_iter()
        .map(|b| {
            let n = uns.iter().filter(|u| u.1 == b).count();
            vec![format!("`{b}`"), (progs.len().saturating_sub(n)).to_string(), n.to_string()]
        })
        .collect();
    table(&["Backend", "Conformance programs passing", "Known unsupported"], &rows)
}
