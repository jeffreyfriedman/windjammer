//! ADR-0009 gate (WJ0173): untrusted data must not reach a command, a file path or SQL text.
//!
//! Oracle: hand-labeled cases in `tests/taint/cases/`. A line ending in `//~ WJ0173` must be the primary line of exactly one
//! WJ0173 diagnostic and no other line may be; a `clean_*` case has no marker and must produce none. The labels were written
//! from the ADR and the cases' comments, not from the analysis' output.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use windjammer::check::{check_sources, taint_findings_sources};
use windjammer::diagnostics::Diagnostic;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cases() -> Vec<(String, String)> {
    let dir = root().join("tests/taint/cases");
    let mut out = Vec::new();
    for e in fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "wj") {
            out.push((p.file_name().unwrap().to_string_lossy().to_string(), fs::read_to_string(&p).unwrap()));
        }
    }
    out.sort();
    out
}

fn taint_lines(diags: &[Diagnostic]) -> BTreeSet<usize> {
    diags.iter().filter(|d| d.code == "WJ0173").filter_map(|d| d.primary_span.as_ref().map(|s| s.line_start)).collect()
}

fn marked_lines(text: &str) -> BTreeSet<usize> {
    text.lines().enumerate().filter(|(_, l)| l.contains("//~ WJ0173")).map(|(i, _)| i + 1).collect()
}

#[test]
fn every_case_has_exactly_the_labeled_findings() {
    let cases = cases();
    assert!(cases.len() >= 20, "the oracle corpus shrank to {}", cases.len());
    let mut failures = Vec::new();
    for (name, text) in &cases {
        let want = marked_lines(text);
        if name.starts_with("flag_") && want.is_empty() {
            failures.push(format!("{name}: a flag_ case without a label"));
        }
        if name.starts_with("clean_") && !want.is_empty() {
            failures.push(format!("{name}: a clean_ case with a label"));
        }
        let got = taint_lines(&taint_findings_sources(Path::new("."), &[(name.clone(), text.clone())]));
        if got != want {
            failures.push(format!("{name}: expected WJ0173 on lines {want:?}, got {got:?}"));
        }
        // `wj check` reports the same through the whole pipeline
        let all = taint_lines(&check_sources(Path::new("."), &[(name.clone(), text.clone())]));
        if all != want {
            failures.push(format!("{name}: `wj check` reports lines {all:?}, wanted {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_finding_names_the_source_the_sink_and_the_remedy() {
    let text = fs::read_to_string(root().join("tests/taint/cases/flag_file_to_command.wj")).unwrap();
    let d = taint_findings_sources(Path::new("."), &[("c.wj".into(), text)]);
    let d = d.iter().find(|d| d.code == "WJ0173").expect("a finding");
    assert!(d.is_error());
    assert!(d.message.contains("file contents") && d.message.contains("command"), "{}", d.message);
    assert_eq!(d.primary_span.as_ref().unwrap().line_start, 7, "the span is on the sink call");
    assert!(d.secondary_spans.iter().any(|s| s.line_start == 6), "the second span is the source call: {:?}", d.secondary_spans);
    let help = d.help.join(" ");
    assert!(help.contains("fs::read_to_string") && help.contains("run_with_args") && help.contains("sanitizer"), "help: {help}");
}

#[test]
fn a_trusted_annotation_without_a_reason_is_an_error() {
    let src = "use std::env\nuse std::process\n\n/// wj-taint: trusted\nfn launch() -> string {\n    process::run(env::get_or(\"J\", \"\")).unwrap_or(\"\")\n}\n\nfn main() {\n    println(\"${launch()}\")\n}\n";
    let d = taint_findings_sources(Path::new("."), &[("t.wj".into(), src.into())]);
    assert!(d.iter().any(|d| d.code == "WJ0173" && d.message.contains("needs a reason")), "{d:?}");
}

#[test]
fn every_source_row_is_a_known_std_function() {
    use windjammer::ir::effect_facts::sinks;
    for s in windjammer::check::taint::SOURCES {
        let known = sinks::all_paths().any(|p| p == s.path) || sinks::NOT_SINKS.contains(&s.path);
        assert!(known, "source `{}` is neither a sink-table row nor a classified non-sink: the std function was renamed or removed", s.path);
    }
    for s in windjammer::check::taint::STD_SANITIZERS {
        let known = sinks::all_paths().any(|p| p == s.path) || sinks::NOT_SINKS.contains(&s.path) || s.path == "regex::escape";
        assert!(known, "sanitizer `{}` is not a std function the effect table classifies", s.path);
    }
}
