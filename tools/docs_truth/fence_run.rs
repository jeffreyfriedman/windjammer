//! Executing a single fence: parse/analyze/lint/interpret Windjammer, or run a shell script in a sandbox.

use super::fences::Ann;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn parse(src: &str) -> Result<windjammer::parser::Program<'static>, String> {
    let src = src.to_string();
    catch_unwind(AssertUnwindSafe(|| {
        let tokens = Lexer::new(&src).tokenize_with_locations();
        // The parser owns the arena the Program borrows from (`'static` by contract): leak it.
        let parser: &'static mut Parser = Box::leak(Box::new(Parser::new_with_source(tokens, "doc.wj".to_string(), src.clone())));
        parser.parse()
    }))
    .unwrap_or_else(|_| Err("parser panicked".to_string()))
}

/// Parse as a program; fragments (statements) are retried as the body of `fn main`.
fn parse_fragment(code: &str) -> Result<(windjammer::parser::Program<'static>, String), String> {
    match parse(code) {
        Ok(p) => Ok((p, code.to_string())),
        Err(e1) => {
            let wrapped = format!("fn main() {{\n{code}\n}}\n");
            parse(&wrapped).map(|p| (p, wrapped)).map_err(|_| format!("parse error: {e1}"))
        }
    }
}

pub fn expects(code: &str) -> Vec<String> {
    code.lines().filter_map(|l| l.trim().strip_prefix("// expect:")).map(|e| e.strip_prefix(' ').unwrap_or(e).trim_end().to_string()).collect()
}

fn interpret(src: &str) -> Result<String, String> {
    let (prog, _) = parse_fragment(src)?;
    catch_unwind(AssertUnwindSafe(|| {
        let mut it = windjammer::interpreter::Interpreter::new_capturing();
        it.run(&prog).map(|_| it.get_output())
    }))
    .unwrap_or_else(|_| Err("interpreter panicked".into()))
}

fn analyze(prog: &windjammer::parser::Program<'static>) -> Result<(), String> {
    catch_unwind(AssertUnwindSafe(|| windjammer::analyzer::Analyzer::new().analyze_program(prog).map(|_| ()).map_err(|e| format!("{e}"))))
        .unwrap_or_else(|_| Err("analyzer panicked".into()))
}

fn lint_codes(prog: &windjammer::parser::Program<'static>, src: &str) -> Vec<String> {
    let mut l = windjammer::linter::rust_leakage::RustLeakageLinter::new("doc.wj");
    l.lint_program(prog);
    l.lint_source(src);
    l.into_diagnostics().into_iter().map(|d| d.lint_name).collect()
}

/// Run on a thread with a big stack: the parser/analyzer recurse deeply and test threads have 2 MB.
pub fn check_wj(code: &str, a: &Ann) -> Result<(), String> {
    let (code, a) = (code.to_string(), a.clone());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || check_wj_inner(&code, &a))
        .map_err(|e| e.to_string())?
        .join()
        .unwrap_or_else(|_| Err("checker thread panicked".into()))
}

fn check_wj_inner(code: &str, a: &Ann) -> Result<(), String> {
    if let Some(want) = &a.should_fail {
        return match parse_fragment(code) {
            Err(e) => match want {
                Some(c) if c.starts_with("WJ") && !e.contains(c.as_str()) => Err(format!("should_fail={c}: parse error lacks the code: {e}")),
                Some(c) if !c.starts_with("WJ") => Err(format!("should_fail={c}: expected a lint but the fence does not parse: {e}")),
                _ => Ok(()),
            },
            Ok((prog, src)) => match want {
                Some(c) if c.starts_with('W') && !c.starts_with("WJ") => {
                    let got = lint_codes(&prog, &src);
                    if got.iter().any(|g| g == c) { Ok(()) } else { Err(format!("should_fail={c}: lints reported {got:?}")) }
                }
                other => match analyze(&prog) {
                    Err(e) if other.as_ref().map_or(true, |c| e.contains(c.as_str())) => Ok(()),
                    Err(e) => Err(format!("should_fail={}: error lacks the code: {e}", other.clone().unwrap_or_default())),
                    Ok(()) => Err("marked should_fail but it parses and analyzes cleanly".to_string()),
                },
            },
        };
    }
    let (prog, _) = parse_fragment(code)?;
    if a.check {
        analyze(&prog).map_err(|e| format!("analyze error: {e}"))?;
    }
    let want = expects(code);
    if !want.is_empty() || a.run {
        let out = interpret(code).map_err(|e| format!("interpreter error: {e}"))?;
        if !want.is_empty() {
            let got: Vec<String> = out.lines().map(|l| l.trim_end().to_string()).collect();
            if got != want {
                return Err(format!("output {got:?} != expected {want:?}"));
            }
        }
    }
    Ok(())
}

/// Run a `sh,verify` fence: lines starting `$ ` are commands (run with `sh -c` in a fresh temp dir
/// whose PATH starts with the directory of `wj_bin`); other lines are expected output lines, each
/// of which must appear, in order, in the combined output of the preceding command.
pub fn run_sh(body: &str, wj_bin: &Path) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = format!("{}:{}", wj_bin.parent().unwrap().display(), std::env::var("PATH").unwrap_or_default());
    let mut cmds: Vec<(String, Vec<String>)> = Vec::new();
    for l in body.lines() {
        if let Some(c) = l.trim_start().strip_prefix("$ ") {
            cmds.push((c.to_string(), vec![]));
        } else if let Some(last) = cmds.last_mut() {
            if !l.trim().is_empty() {
                last.1.push(l.trim_end().to_string());
            }
        }
    }
    for (cmd, expected) in cmds {
        let out = std::process::Command::new("sh").arg("-c").arg(&cmd).current_dir(dir.path()).env("PATH", &path).output().map_err(|e| e.to_string())?;
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        if !out.status.success() {
            return Err(format!("`{cmd}` exited {:?}: {}", out.status.code(), text.trim()));
        }
        let mut from = 0;
        for e in expected {
            match text[from..].find(e.trim()) {
                Some(i) => from += i + e.trim().len(),
                None => return Err(format!("`{cmd}`: expected line {e:?} not found in output {:?}", text.trim())),
            }
        }
    }
    Ok(())
}
