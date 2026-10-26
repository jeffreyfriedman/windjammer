//! Text and JSON rendering of a [`Report`].

use super::report::{Finding, Reason, Report};
use std::fmt::Write;

pub fn to_json(report: &Report) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

fn reason_line(r: &Reason) -> String {
    let at = match (r.line, r.col) {
        (Some(l), Some(c)) => format!(" @{}:{}", l, c),
        (Some(l), None) => format!(" @{}", l),
        _ => String::new(),
    };
    let flag = if r.binding {
        "*"
    } else if r.dropped {
        "!"
    } else {
        " "
    };
    format!("{} [{}]{} {}", flag, r.rule, at, r.detail)
}

fn finding_line(f: &Finding) -> String {
    format!("    ! {}: {}", f.id, f.message)
}

pub fn to_text(report: &Report) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{}", report.file);
    if !report.has_emitted {
        let _ = writeln!(s, "  (no emitted Rust available: `emitted` columns omitted)");
    }
    for n in &report.notes {
        let _ = writeln!(s, "  note: {}", n);
    }
    for f in &report.functions {
        let _ = writeln!(s, "\nfn {}", f.name);
        for p in &f.params {
            let pin = p.pinned.as_deref().map(|x| format!(" pinned {}", x)).unwrap_or_default();
            let _ = writeln!(
                s,
                "  param {}: {}  ->  {}{}",
                p.name, p.declared, p.effective, pin
            );
            let _ = writeln!(
                s,
                "    stages: analyzer={} solver={} ir={} emitted={}",
                p.analyzer.as_deref().unwrap_or("-"),
                p.solver.as_deref().unwrap_or("-"),
                p.ir_final.as_deref().unwrap_or("-"),
                p.emitted.as_deref().unwrap_or("-")
            );
            if p.trail.len() > 1 {
                let _ = writeln!(s, "    trail: {}", p.trail.join(" -> "));
            }
            for r in &p.reasons {
                let _ = writeln!(s, "    {}", reason_line(r));
            }
            for x in &p.findings {
                let _ = writeln!(s, "{}", finding_line(x));
            }
        }
        for l in &f.locals {
            let _ = writeln!(
                s,
                "  local {}: solver={} emitted={}{}",
                l.name,
                l.solver_width.as_deref().unwrap_or("-"),
                l.emitted_width.as_deref().unwrap_or("-"),
                match l.mutable_in_emitted {
                    Some(true) => " (let mut in emitted)",
                    _ => "",
                }
            );
            for r in &l.reasons {
                let _ = writeln!(s, "    {}", reason_line(r));
            }
            for x in &l.findings {
                let _ = writeln!(s, "{}", finding_line(x));
            }
        }
        for c in &f.clones {
            let _ = writeln!(
                s,
                "  clone {}: solver_needs_clone={} emitted_clones={}",
                c.var, c.solver_needs_clone, c.emitted_clones
            );
            for r in &c.reasons {
                let _ = writeln!(s, "    {}", reason_line(r));
            }
            for x in &c.findings {
                let _ = writeln!(s, "{}", finding_line(x));
            }
        }
        for c in &f.calls {
            for a in &c.args {
                let _ = writeln!(
                    s,
                    "  call {}(arg{}): {}{} formal={} used_later={} {}",
                    c.callee,
                    a.index,
                    a.form,
                    a.var.as_deref().map(|v| format!(" `{}`", v)).unwrap_or_default(),
                    a.formal.as_deref().unwrap_or("?"),
                    a.used_later.map(|b| b.to_string()).unwrap_or_else(|| "-".into()),
                    a.rule.as_deref().map(|r| format!("[{}]", r)).unwrap_or_default()
                );
                for x in &a.findings {
                    let _ = writeln!(s, "{}", finding_line(x));
                }
            }
        }
    }
    let n = report.findings().len();
    let _ = writeln!(s, "\n{} finding(s). `*` binding constraint, `!` constraint dropped by the solver.", n);
    s
}
