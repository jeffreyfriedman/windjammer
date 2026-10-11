// wj check - type check without building.
//
// For `.wj` files (or a directory of them) this runs the Windjammer front end in-process, with no cargo
// and no rustc: parse, the Rust-leakage lints (W0001-W0006), then the full transpile (analysis, IR,
// codegen) into a scratch directory. `--json` prints one machine-readable report on stdout using the
// same schema as `wj build --json`. A directory with only a `Cargo.toml` keeps the old `cargo check`.

use crate::cli::json_diagnostics::{
    DiagnosticSpan, JsonCompilationOutput, JsonDiagnostic, RepairHint, RepairKind, Severity,
};
use anyhow::{bail, Result};
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Legacy behaviour: `cargo check` in the current directory.
pub fn execute() -> Result<()> {
    println!("{} type checking", "Running".green().bold());
    let status = Command::new("cargo").arg("check").status()?;
    if !status.success() {
        bail!("Type check failed");
    }
    println!("{} No type errors found", "✓".green().bold());
    Ok(())
}

/// CLI entry: exits with status 1 when the report contains errors.
pub fn run(path: Option<&Path>, json: bool) -> Result<()> {
    let target = path.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut files = Vec::new();
    collect_wj_files(&target, &mut files);
    if files.is_empty() {
        if path.is_none() && Path::new("Cargo.toml").exists() {
            return execute();
        }
        let mut out = JsonCompilationOutput::new_failure();
        out.add_error(format!("no .wj files found at {}", target.display()));
        emit(&out, json);
        std::process::exit(1);
    }
    let report = check_path(&target);
    emit(&report, json);
    if !report.success {
        std::process::exit(1);
    }
    Ok(())
}

fn emit(report: &JsonCompilationOutput, json: bool) {
    if json {
        report.emit_to_stdout();
        return;
    }
    for d in &report.diagnostics {
        let sev = match d.severity {
            Severity::Error => "error".red().bold(),
            Severity::Warning => "warning".yellow().bold(),
            Severity::Info => "info".normal(),
            Severity::Hint => "hint".normal(),
        };
        let code = d.code.as_deref().map(|c| format!(" [{}]", c)).unwrap_or_default();
        println!("{}: {}{}", sev, d.message, code);
        if let Some(s) = &d.span {
            println!("  --> {}:{}:{}", s.file, s.start_line, s.start_col);
        }
        if let Some(h) = &d.repair_hint {
            println!("  = help: {}", h.description);
        }
    }
    if report.success {
        println!("{} No errors found", "✓".green().bold());
    }
}

fn collect_wj_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "wj") {
            out.push(path.to_path_buf());
        }
    } else if let Ok(rd) = std::fs::read_dir(path) {
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() && (name.starts_with('.') || name == "target" || name == "build") {
                continue;
            }
            collect_wj_files(&p, out);
        }
    }
}

/// Run parse + lint + transpile over `path` and collect every diagnostic. Never panics on bad input.
pub fn check_path(path: &Path) -> JsonCompilationOutput {
    let mut files = Vec::new();
    collect_wj_files(path, &mut files);
    let mut diagnostics: Vec<JsonDiagnostic> = Vec::new();
    let mut parse_failed = false;

    for f in &files {
        let source = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(diag(None, Severity::Error, format!("cannot read {}: {}", f.display(), e), None));
                parse_failed = true;
                continue;
            }
        };
        let file_name = f.to_string_lossy().to_string();
        // The lexer and parser may panic on malformed input; a checker must report, not crash.
        let src = source.clone();
        let fname = file_name.clone();
        let parsed = std::panic::catch_unwind(move || lint_one(&fname, &src));
        match parsed {
            Ok(Ok(mut lints)) => diagnostics.append(&mut lints),
            Ok(Err(msg)) => {
                parse_failed = true;
                let span = span_from_message(&msg, &file_name);
                diagnostics.push(diag(Some("parse".into()), Severity::Error, msg, span));
            }
            Err(_) => {
                parse_failed = true;
                diagnostics.push(diag(
                    Some("parse".into()),
                    Severity::Error,
                    format!("the parser failed on {} (unsupported or malformed syntax)", file_name),
                    None,
                ));
            }
        }
    }

    if !parse_failed {
        let tmp = std::env::temp_dir().join(format!("wj-check-{}-{}", std::process::id(), unique()));
        let _ = std::fs::create_dir_all(&tmp);
        let config = crate::compiler_config::CompilerConfig::default();
        let p = path.to_path_buf();
        let t = tmp.clone();
        let result = std::panic::catch_unwind(move || {
            crate::build_project_with_config(&p, &t, crate::CompilationTarget::Rust, false, false, &[], &config)
        });
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let msg = format!("{}", e);
                let span = span_from_message(&msg, &path.to_string_lossy());
                diagnostics.push(diag(Some("compile".into()), Severity::Error, msg, span));
            }
            Err(_) => diagnostics.push(diag(
                Some("compile".into()),
                Severity::Error,
                "internal compiler error while checking (please report with the source)".into(),
                None,
            )),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    let has_error = diagnostics.iter().any(|d| matches!(d.severity, Severity::Error));
    let mut out = if has_error { JsonCompilationOutput::new_failure() } else { JsonCompilationOutput::new_success() };
    out.diagnostics = diagnostics;
    out
}

fn unique() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn lint_one(file_name: &str, source: &str) -> std::result::Result<Vec<JsonDiagnostic>, String> {
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new_with_source(tokens, file_name.to_string(), source.to_string());
    let program = parser.parse()?;
    let mut linter = crate::linter::rust_leakage::RustLeakageLinter::new(file_name);
    linter.lint_program(&program);
    linter.lint_source(source);
    Ok(linter
        .into_diagnostics()
        .into_iter()
        .map(|d| {
            let severity = match d.level {
                crate::linter::LintLevel::Error => Severity::Error,
                crate::linter::LintLevel::Warning => Severity::Warning,
                _ => Severity::Info,
            };
            let span = DiagnosticSpan {
                file: d.location.file.clone(),
                start_line: d.location.line,
                start_col: d.location.column,
                end_line: d.location.line,
                end_col: d.location.column,
            };
            let hint = d.suggestion.clone().or(d.help.clone()).map(|description| RepairHint {
                kind: RepairKind::ReplaceText,
                description,
                confidence: 0.8,
                replacement: None,
            });
            let mut j = diag(Some(d.lint_name.clone()), severity, d.message.clone(), Some(span));
            j.repair_hint = hint;
            j
        })
        .collect())
}

fn diag(code: Option<String>, severity: Severity, message: String, span: Option<DiagnosticSpan>) -> JsonDiagnostic {
    JsonDiagnostic { code, severity, message, span, maturity: None, repair_hint: None }
}

/// Find the first `file:LINE:COL` (or `:LINE:COL`) in a compiler message.
pub fn span_from_message(msg: &str, default_file: &str) -> Option<DiagnosticSpan> {
    let bytes: Vec<char> = msg.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ':' {
            let (line, j) = number_at(&bytes, i + 1)?;
            if j < bytes.len() && bytes[j] == ':' {
                if let Some((col, k)) = number_at(&bytes, j + 1) {
                    let start = msg[..msg.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0)]
                        .rsplit(|c: char| c.is_whitespace() || c == '(' || c == '"')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let _ = k;
                    return Some(DiagnosticSpan {
                        file: if start.is_empty() { default_file.to_string() } else { start },
                        start_line: line,
                        start_col: col,
                        end_line: line,
                        end_col: col,
                    });
                }
            }
        }
        i += 1;
    }
    None
}

fn number_at(chars: &[char], from: usize) -> Option<(usize, usize)> {
    let mut j = from;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    if j == from {
        return None;
    }
    let n: String = chars[from..j].iter().collect();
    n.parse().ok().map(|v| (v, j))
}
