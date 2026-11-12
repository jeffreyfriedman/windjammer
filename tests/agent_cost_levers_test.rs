#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]
//! Agent-cost levers (docs/adr/0037-agent-cost-levers.md), checked through the real `wj` binary:
//! - a compiled run on a pipe prints the program's output and nothing else: no build progress and no rustc warnings
//!   about the generated Rust (pilot 6: every compiled build showed them, about 300 tokens per call);
//! - `--error-format agent` (and `WJ_ERROR_FORMAT=agent`) prints one compact line per diagnostic with code, location,
//!   help and fix, and is smaller than the human format for the same program.

use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// Uses `std::fs` (whose runtime module brought unused `Metadata`/`DirEntry` imports) and an index compound
/// assignment (whose generated index carried extra parentheses): the two warning sources pilot 6 recorded.
const NOISY: &str = "use std::fs\nuse std::collections::HashMap\n\nfn main() {\n    let text = fs::read_to_string(\"in.txt\") ?? \"\"\n    let mut totals: Vec<f64> = [0.0, 0.0]\n    let i: int = 1\n    totals[i] += 2.5\n    let mut m: HashMap<string, int> = HashMap::new()\n    m.insert(\"a\", 1)\n    println(\"${text.len()} ${totals[1]} ${m.len()}\")\n}\n";

/// Two errors (an unknown name and a deprecated macro with a machine-applicable fix).
const BAD: &str = "fn main() {\n    let v = vec![1, 2]\n    println(\"${v.len()} ${missing}\")\n}\n";

fn wj(dir: &TempDir, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut c = Command::new(WJ);
    c.args(args).current_dir(dir.path()).env_remove("WJ_ERROR_FORMAT").env_remove("WJ_PROGRESS");
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn project(src: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    fs::write(d.path().join("prog.wj"), src).unwrap();
    fs::write(d.path().join("in.txt"), "abc").unwrap();
    d
}

#[test]
fn compiled_run_on_a_pipe_prints_only_the_program() {
    let d = project(NOISY);
    // a second source in the directory makes the build incremental, which used to print `⚡ ...` cache notes
    fs::write(d.path().join("extra.wj"), "fn main() {\n    println(\"extra\")\n}\n").unwrap();
    let o = wj(&d, &["run", "--compiled", "prog.wj"], &[]);
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "3 2.5 1\n");
    let err = String::from_utf8_lossy(&o.stderr);
    for noise in ["warning:", "Transpilation complete", "Building Windjammer project", "Running ", "Output:", "\u{26a1}", "\u{27f3}"] {
        assert!(!err.contains(noise), "compiled run printed `{noise}` on a pipe:\n{err}");
    }
    // a second (warm) run replays nothing either
    let o = wj(&d, &["run", "--compiled", "prog.wj"], &[]);
    assert!(String::from_utf8_lossy(&o.stderr).trim().is_empty(), "warm run printed:\n{}", text(&o));
}

#[test]
fn progress_can_be_forced_on_a_pipe() {
    let d = project(NOISY);
    let o = wj(&d, &["run", "--compiled", "prog.wj"], &[("WJ_PROGRESS", "1")]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(String::from_utf8_lossy(&o.stderr).contains("Running"), "WJ_PROGRESS=1 must show progress:\n{}", text(&o));
}

fn code_lines(s: &str) -> Vec<&str> {
    s.lines().filter(|l| l.contains(" error WJ") || l.contains(" warning W") || l.contains(" error W")).collect()
}

#[test]
fn agent_format_is_one_line_per_diagnostic_and_smaller() {
    let d = project(BAD);
    let human = wj(&d, &["check", "prog.wj"], &[]);
    let agent = wj(&d, &["check", "--error-format", "agent", "prog.wj"], &[]);
    assert_eq!(human.status.code(), agent.status.code(), "the format must not change the verdict");
    assert_eq!(agent.status.code(), Some(1), "{}", text(&agent));
    let a = text(&agent);
    let lines = code_lines(&a);
    assert!(lines.len() >= 2, "expected one line per diagnostic:\n{a}");
    assert!(lines.iter().all(|l| l.starts_with("prog.wj:")), "every line starts with its location:\n{a}");
    assert!(!a.contains("= explain:") && !a.lines().any(|l| l.trim_start().starts_with('|')), "no excerpt or explain lines in the agent format:\n{a}");
    assert!(a.contains("| fix"), "the machine-applicable fix is on the line:\n{a}");
    assert!(a.len() * 10 <= text(&human).len() * 6, "agent format ({} bytes) is not at most 60% of the human one ({} bytes)", a.len(), text(&human).len());
}

#[test]
fn agent_format_from_the_environment_and_in_run() {
    let d = project(BAD);
    let flag = text(&wj(&d, &["check", "--error-format", "agent", "prog.wj"], &[]));
    let env = text(&wj(&d, &["check", "prog.wj"], &[("WJ_ERROR_FORMAT", "agent")]));
    assert_eq!(code_lines(&flag), code_lines(&env));
    let run = wj(&d, &["run", "--interpret", "--error-format", "agent", "prog.wj"], &[]);
    assert_ne!(run.status.code(), Some(0));
    let r = text(&run);
    assert!(!code_lines(&r).is_empty() && !r.contains("= explain:"), "interpreted refusal in the agent format:\n{r}");
    // `human` overrides the environment
    let h = text(&wj(&d, &["check", "--error-format", "human", "prog.wj"], &[("WJ_ERROR_FORMAT", "agent")]));
    assert!(h.contains("= explain:"), "--error-format human must win over the environment:\n{h}");
}

#[test]
fn unknown_error_format_is_rejected() {
    let d = project(BAD);
    let o = wj(&d, &["check", "--error-format", "terse", "prog.wj"], &[]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(text(&o).contains("possible values"), "{}", text(&o));
}

#[test]
fn escaped_dollar_before_braces_is_reported_with_the_rewrite() {
    // pilot 6, t103: `"$${d}.${c}"` printed `${d}.50`; the program ran and nothing said why
    let d = project("fn main() {\n    let d = 4\n    println(\"$${d}.50\")\n}\n");
    let o = wj(&d, &["check", "--error-format", "agent", "prog.wj"], &[]);
    let t = text(&o);
    assert_eq!(o.status.code(), Some(0), "a warning, not an error:\n{t}");
    assert!(t.contains("prog.wj:3:14: warning W0017") && t.contains("`\\$${`"), "{t}");
    // the suggested spelling prints what was meant and is not reported
    let good = project("fn main() {\n    let d = 4\n    println(\"\\$${d}.50\")\n}\n");
    assert!(!text(&wj(&good, &["check", "prog.wj"], &[])).contains("W0017"));
    let run = wj(&good, &["run", "--interpret", "prog.wj"], &[]);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "$4.50\n");
}
