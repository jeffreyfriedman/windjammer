//! Numbers from the agent pilot (bench/agent-dx/results/pilot6/report.json) for the book, as generated regions, and the gate
//! that keeps comparative claims honest: a page may say "cheaper", "faster" and the like only with a generated number
//! right beside it, never from memory.

use super::{book, util, Ctx, Finding};
use serde_json::Value;

pub const REPORT: &str = "bench/agent-dx/results/pilot6/report.json";

fn report(ctx: &Ctx) -> Result<Value, String> {
    let text = ctx.read(REPORT);
    if text.is_empty() {
        return Err(format!("{REPORT} is missing"));
    }
    serde_json::from_str(&text).map_err(|e| format!("{REPORT}: {e}"))
}

/// `mean [lo, hi]` from a `[mean, lo, hi]` list.
fn interval(v: &Value, digits: usize) -> String {
    match v.as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).collect::<Vec<_>>()) {
        Some(a) if a.len() >= 3 => format!("{:.d$} [{:.d$}, {:.d$}]", a[0], a[1], a[2], d = digits),
        Some(a) if a.len() == 1 => format!("{:.d$}", a[0], d = digits),
        _ => v.as_f64().map(|x| format!("{x:.digits$}")).unwrap_or_else(|| "-".into()),
    }
}

/// The arm summary: pass rate, cost, turns, tokens, wall time.
pub fn table(ctx: &Ctx) -> Result<String, String> {
    let r = report(ctx)?;
    let rows: [(&str, &str, usize); 6] = [
        ("pass@1 against the hidden tests", "pass", 3),
        ("cost per trial, USD", "cost", 3),
        ("agent turns", "turns", 1),
        ("tokens in (including cache)", "tok_in", 0),
        ("tokens out", "tok_out", 0),
        ("wall seconds (a loaded machine, see the limits)", "wall", 1),
    ];
    let body: Vec<Vec<String>> = rows.iter().map(|(label, key, d)| vec![label.to_string(), interval(&r["arms"][key]["R"], *d), interval(&r["arms"][key]["U"], *d), interval(&r["arms"][key]["V2"], *d)]).collect();
    Ok(util::table(&["Per trial", "Plain Rust", "Windjammer, compiled first", "Windjammer, interpreter first"], &body))
}

/// Paired differences in cost: first minus second, with whether the 95% interval contains zero.
pub fn paired(ctx: &Ctx) -> Result<String, String> {
    let r = report(ctx)?;
    let mut rows = Vec::new();
    for (pair, label) in [("R-U", "Rust minus compiled-first"), ("R-V2", "Rust minus interpreter-first"), ("U-V2", "compiled-first minus interpreter-first")] {
        let v = &r["diffs"]["cost"][pair];
        let a: Vec<f64> = v.as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
        let zero = a.len() >= 3 && a[1] <= 0.0 && a[2] >= 0.0;
        rows.push(vec![label.to_string(), interval(v, 3), if zero { "yes: no difference shown".to_string() } else { "no".to_string() }]);
    }
    Ok(util::table(&["Cost per trial, USD", "Difference [95% interval]", "Interval contains zero"], &rows))
}

/// Cost per trial for each task and arm, and which arm cost least.
pub fn by_task(ctx: &Ctx) -> Result<String, String> {
    let r = report(ctx)?;
    let tasks = r["by_task"].as_object().ok_or("pilot6_by_task: no by_task in the report")?;
    let mut rows = Vec::new();
    for (name, v) in tasks {
        let cost = |a: &str| v[a]["cost"].as_f64().unwrap_or(f64::NAN);
        let (rc, uc, vc) = (cost("R"), cost("U"), cost("V2"));
        let cheapest = if rc <= uc && rc <= vc { "Rust" } else if uc <= vc { "Windjammer, compiled first" } else { "Windjammer, interpreter first" };
        rows.push(vec![name.clone(), format!("{rc:.3}"), format!("{uc:.3}"), format!("{vc:.3}"), cheapest.to_string()]);
    }
    Ok(util::table(&["Task", "Plain Rust", "Compiled first", "Interpreter first", "Least cost"], &rows))
}

/// One value by dot path (`rust_errors.n`, `noisy.R.noisy`); a `[mean, lo, hi]` list gives its mean.
pub fn value(ctx: &Ctx, path: &str, digits: usize) -> Result<String, String> {
    let r = report(ctx)?;
    let mut v = &r;
    for part in path.split('.') {
        v = &v[part];
    }
    if v.is_null() {
        return Err(format!("pilot6_value: no `{path}` in {REPORT}"));
    }
    let x = match v.as_array() {
        Some(a) => a.first().and_then(|x| x.as_f64()),
        None => v.as_f64(),
    };
    x.map(|x| format!("{x:.digits$}")).ok_or_else(|| format!("pilot6_value: `{path}` is not a number"))
}

const CLAIM_WORDS: [&str; 10] = ["cheaper", "faster", "slower", "quicker", "fewer tokens", "fewer turns", "fewer lines", "less expensive", "cost less", "more efficient"];
const NUMBER_REGIONS: [&str; 5] = ["name=metric", "name=pilot6_", "name=showcase_loc", "name=file_bytes", "name=book_stats"];

/// A prose line with a comparative claim needs a generated number within a few lines of it.
pub fn claims(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for (file, text) in book::pages(ctx) {
        if text.lines().take(3).any(|l| l.contains(super::genfiles::HEADER_MARK)) {
            continue; // generated reference pages
        }
        let lines: Vec<&str> = text.lines().collect();
        for (ln, line) in super::md::prose_lines(&text) {
            let low = line.to_lowercase();
            if let Some(w) = CLAIM_WORDS.iter().find(|w| low.contains(**w)) {
                let from = ln.saturating_sub(13);
                let to = (ln + 12).min(lines.len());
                let near = lines[from..to].iter().any(|l| l.contains("wj:gen") && NUMBER_REGIONS.iter().any(|r| l.contains(r)));
                if !near {
                    out.push(Finding::fail("book", &file, ln, format!("comparative claim (\"{w}\") with no generated number beside it: put a `wj:gen` metric or pilot region within a few lines, or say it without the comparison")));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_format_with_the_requested_digits() {
        assert_eq!(interval(&serde_json::json!([0.0334, 0.026, 0.042]), 3), "0.033 [0.026, 0.042]");
        assert_eq!(interval(&serde_json::json!(1.0), 1), "1.0");
    }
}
