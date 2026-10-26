// wj errors - browse the diagnostics of a file or project.
//
// A non-interactive error browser over the front-end checker (the same diagnostics as
// `wj check --json`): a per-code summary followed by one entry per diagnostic, filterable by
// code and severity so an agent or a person can walk a long list one class at a time.

use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::path::Path;

use crate::diagnostics::{Diagnostic, Severity};

/// Parsed `--severity` value.
fn parse_severity(s: &str) -> Result<Severity> {
    match s.to_ascii_lowercase().as_str() {
        "error" => Ok(Severity::Error),
        "warning" => Ok(Severity::Warning),
        other => bail!("invalid --severity '{other}': expected 'error' or 'warning'"),
    }
}

/// Keep the diagnostics matching `code` (case-insensitive) and `severity`.
pub fn filter<'a>(
    diags: &'a [Diagnostic],
    code: Option<&str>,
    severity: Option<Severity>,
) -> Vec<&'a Diagnostic> {
    diags
        .iter()
        .filter(|d| code.is_none_or(|c| d.code.eq_ignore_ascii_case(c)))
        .filter(|d| severity.is_none_or(|s| d.severity == s))
        .collect()
}

/// Render the browser view: per-code summary, then every selected diagnostic.
pub fn render(all: &[Diagnostic], selected: &[&Diagnostic]) -> String {
    let mut out = String::new();
    let mut by_code: BTreeMap<&str, usize> = BTreeMap::new();
    for d in selected {
        *by_code.entry(d.code.as_str()).or_default() += 1;
    }
    out.push_str(&format!(
        "wj errors: {} of {} diagnostic(s) shown\n",
        selected.len(),
        all.len()
    ));
    if !by_code.is_empty() {
        out.push_str("\nby code:\n");
        for (code, n) in &by_code {
            out.push_str(&format!("  {code}  x{n}\n"));
        }
    }
    for (i, d) in selected.iter().enumerate() {
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        };
        let loc = d
            .primary_span
            .as_ref()
            .map(|s| format!("{}:{}:{}", s.file, s.line_start, s.col_start))
            .unwrap_or_else(|| "<no location>".to_string());
        out.push_str(&format!(
            "\n[{}] {sev}[{}] {loc}\n    {}\n",
            i + 1,
            d.code,
            d.message
        ));
        for h in &d.help {
            out.push_str(&format!("    help: {h}\n"));
        }
        out.push_str(&format!("    explain: wj explain {}\n", d.code));
    }
    if selected.is_empty() {
        out.push_str("\nNo diagnostics match.\n");
    }
    out
}

pub fn execute(path: &Path, code: Option<&str>, severity: Option<&str>) -> Result<()> {
    if !path.exists() {
        bail!("path not found: {}", path.display());
    }
    let severity = severity.map(parse_severity).transpose()?;
    let report = super::check::check_path(path);
    let selected = filter(&report.diagnostics, code, severity);
    print!("{}", render(&report.diagnostics, &selected));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(code: &str, sev: Severity) -> Diagnostic {
        Diagnostic::new(code, sev, format!("message for {code}"))
    }

    #[test]
    fn filters_by_code_and_severity() {
        let all = vec![
            diag("WJ0001", Severity::Error),
            diag("W0005", Severity::Warning),
            diag("WJ0001", Severity::Error),
        ];
        assert_eq!(filter(&all, Some("wj0001"), None).len(), 2);
        assert_eq!(filter(&all, None, Some(Severity::Warning)).len(), 1);
        assert_eq!(filter(&all, Some("W0005"), Some(Severity::Error)).len(), 0);
    }

    #[test]
    fn render_lists_summary_and_entries() {
        let all = vec![
            diag("WJ0001", Severity::Error),
            diag("WJ0001", Severity::Error),
        ];
        let sel = filter(&all, None, None);
        let text = render(&all, &sel);
        assert!(
            text.contains("2 of 2 diagnostic(s)")
                && text.contains("WJ0001  x2")
                && text.contains("[2] error[WJ0001]"),
            "{text}"
        );
        assert!(render(&all, &[]).contains("No diagnostics match"));
    }

    #[test]
    fn rejects_unknown_severity() {
        assert!(parse_severity("fatal").is_err());
    }
}
