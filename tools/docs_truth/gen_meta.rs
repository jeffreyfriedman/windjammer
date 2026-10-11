//! Computed metrics (`docs/generated/metrics.json`) and the ADR index.

use super::{gen_cli, gen_errors, gen_targets, util, Ctx};
use std::collections::BTreeMap;
use windjammer::intrinsics::Intrinsic;
use windjammer::lexer::keywords::KEYWORDS;
use windjammer::linter::codes::LINT_CODES;

/// Metrics derived from the repo and the compiler tables; deterministic, so `--check` can verify them.
pub fn computed(ctx: &Ctx) -> BTreeMap<String, u64> {
    let mut m = BTreeMap::new();
    let tests = util::walk(&ctx.root.join("tests"));
    m.insert("tests.rust_files".into(), tests.iter().filter(|f| !f.contains('/') && f.ends_with(".rs")).count() as u64);
    m.insert("tests.wj_programs".into(), tests.iter().filter(|f| f.ends_with(".wj")).count() as u64);
    m.insert("conformance.programs".into(), gen_targets::programs(ctx).len() as u64);
    m.insert("conformance.backends".into(), gen_targets::backends(ctx).len() as u64);
    m.insert("conformance.unsupported_pairs".into(), gen_targets::unsupported(ctx).len() as u64);
    m.insert("errors.codes_total".into(), gen_errors::known_codes().len() as u64);
    m.insert("lints.w_codes".into(), LINT_CODES.len() as u64);
    m.insert("lexer.keywords".into(), KEYWORDS.len() as u64);
    m.insert("intrinsics.count".into(), Intrinsic::ALL.len() as u64);
    m.insert("cli.subcommands".into(), gen_cli::commands(&ctx.cli).len() as u64);
    m.insert("adr.count".into(), adrs(ctx).len() as u64);
    let std_files = util::walk(&ctx.root.join("std")).iter().filter(|f| f.ends_with(".wj") && !f.ends_with("_test.wj")).count();
    m.insert("stdlib.files".into(), std_files as u64);
    m
}

pub fn metrics_json(ctx: &Ctx) -> String {
    let body: BTreeMap<String, serde_json::Value> = computed(ctx).into_iter().map(|(k, v)| (k, v.into())).collect();
    let v = serde_json::json!({
        "_generated": "GENERATED FILE - DO NOT EDIT. Regenerate: scripts/docs_gen.sh. CI-measured numbers go in docs/generated/measured.json.",
        "metrics": body,
    });
    serde_json::to_string_pretty(&v).unwrap() + "\n"
}

/// Value of `key` from computed metrics, else from CI-recorded `measured.json` (`{"key": {"value": ..}}`).
pub fn metric(ctx: &Ctx, key: &str) -> Result<String, String> {
    if let Some(v) = computed(ctx).get(key) {
        return Ok(v.to_string());
    }
    let text = ctx.read("docs/generated/measured.json");
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| format!("unknown metric `{key}` (no docs/generated/measured.json)"))?;
    match v.get(key).and_then(|e| e.get("value")) {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(x) => Ok(x.to_string()),
        None => Err(format!("unknown metric `{key}`")),
    }
}

/// (number, title, status) per ADR, read from the ADR files themselves.
pub fn adrs(ctx: &Ctx) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for f in util::walk(&ctx.root.join("docs/adr")) {
        if f.contains('/') || !f.ends_with(".md") || !f.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        let text = ctx.read(&format!("docs/adr/{f}"));
        let title = text.lines().next().unwrap_or("").trim_start_matches('#').trim();
        let title = title.split_once(": ").map(|x| x.1).unwrap_or(title).to_string();
        let status = text.lines().find_map(|l| l.strip_prefix("- **Status:**")).unwrap_or("(missing)").trim();
        let status = status.split(['(', '.', ';']).next().unwrap_or(status).trim().to_string();
        out.push((f.clone(), title, status));
    }
    out
}

pub fn adr_index(ctx: &Ctx) -> String {
    let rows: Vec<Vec<String>> = adrs(ctx)
        .into_iter()
        .map(|(f, t, s)| vec![format!("[{}]({f})", &f[..4]), t, s])
        .collect();
    util::table(&["ADR", "Title", "Status"], &rows)
}
